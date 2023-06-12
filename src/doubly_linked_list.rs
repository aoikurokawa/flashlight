use std::{marker::PhantomData, ptr::NonNull};

// this module adds some functionality based on the required implementations
// here like: `LinkedList::pop_back` or `Clone for LinkedList<T>`
// You are free to use anything in it, but it's mainly for the test framework.
// mod pre_implemented;
//
pub struct Node<T> {
    val: T,
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    fn new(val: T) -> Self {
        Self {
            val,
            next: None,
            prev: None,
        }
    }

    fn into_val(self) -> T {
        self.val
    }
}

pub struct LinkedList<T> {
    len: usize,
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    _marker: PhantomData<Box<Node<T>>>,
}

pub struct Cursor<'a, T> {
    list: &'a mut LinkedList<T>,
    node: Option<NonNull<Node<T>>>,
}

pub struct Iter<'a, T> {
    head: Option<NonNull<Node<T>>>,
    len: usize,
    _marker: PhantomData<&'a Node<T>>,
}

impl<T> Default for LinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            len: 0,
            head: None,
            tail: None,
            _marker: PhantomData,
        }
    }

    // You may be wondering why it's necessary to have is_empty()
    // when it can easily be determined from len().
    // It's good custom to have both because len() can be expensive for some types,
    // whereas is_empty() is almost always cheap.
    // (Also ask yourself whether len() is expensive for LinkedList)
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Return a cursor positioned on the front element
    pub fn cursor_front(&mut self) -> Cursor<'_, T> {
        Cursor {
            node: self.head,
            list: self,
        }
    }

    /// Return a cursor positioned on the back element
    pub fn cursor_back(&mut self) -> Cursor<'_, T> {
        Cursor {
            node: self.tail,
            list: self,
        }
    }

    /// Return an iterator that moves from front to back
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            head: self.head,
            len: self.len,
            _marker: PhantomData,
        }
    }

    pub fn pop_front(&mut self) -> Option<T> {
        None
    }

    pub fn pop_back(&mut self) -> Option<T> {
        None
    }

    pub fn push_front(&mut self, _data: T) {}

    pub fn push_back(&mut self, _data: T) {}
}

// the cursor is expected to act as if it is at the position of an element
// and it also has to work with and be able to insert into an empty list.
impl<T> Cursor<'_, T> {
    /// Take a mutable reference to the current element
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        match &mut self.node {
            None => None,
            Some(node) => Some(unsafe { &mut node.as_mut().val }),
        }
    }

    /// Move one position forward (towards the back) and
    /// return a reference to the new position
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&mut T> {
        if let Some(x) = self.node {
            self.node = unsafe { x.as_ref().next };
        }
        self.peek_mut()
    }

    /// Move one position backward (towards the front) and
    /// return a reference to the new position
    pub fn prev(&mut self) -> Option<&mut T> {
        if let Some(x) = self.node {
            self.node = unsafe { x.as_ref().prev };
        }
        self.peek_mut()
    }

    /// Remove and return the element at the current position and move the cursor
    /// to the neighboring element that's closest to the back. This can be
    /// either the next or previous position.
    pub fn take(&mut self) -> Option<T> {
        self.node?;

        self.node.map(|node| {
            self.list.len -= 1;

            if self.list.head.unwrap() == node {
                self.list.head = unsafe { node.as_ref().next };
            }

            if self.list.tail.unwrap() == node {
                self.list.tail = unsafe { node.as_ref().prev };
            }

            unsafe {
                let mut node = Box::from_raw(node.as_ptr());
                if let Some(x) = &mut node.prev {
                    x.as_mut().next = node.next;
                }

                if let Some(x) = &mut node.next {
                    x.as_mut().prev = node.prev;
                }

                match node.next {
                    Some(x) => self.node = Some(x),
                    None => self.node = node.prev,
                }

                node.into_val()
            }
        })
    }

    pub fn insert_after(&mut self, element: T) {
        let mut node = Box::new(Node::new(element));
        self.list.len += 1;

        if self.node.is_none() {
            let node = NonNull::new(Box::into_raw(node));
            self.node = node;
            self.list.head = node;
            self.list.tail = node;
        } else {
            node.next = unsafe { self.node.unwrap().as_ref().next };
            node.prev = self.node;
            let node = NonNull::new(Box::into_raw(node));
            match unsafe { &mut self.node.unwrap().as_mut().next } {
                Some(x) => unsafe { x.as_mut().prev = node },
                None => self.list.tail = node,
            }
            unsafe { self.node.unwrap().as_mut().next = node };
        }
    }

    pub fn insert_before(&mut self, element: T) {
        let mut node = Box::new(Node::new(element));
        self.list.len += 1;

        if self.node.is_none() {
            let node = NonNull::new(Box::into_raw(node));
            self.node = node;
            self.list.head = node;
            self.list.tail = node;
        } else {
            node.next = self.node;
            node.prev = unsafe { self.node.unwrap().as_ref().prev };
            let node = NonNull::new(Box::into_raw(node));
            match unsafe { &mut self.node.unwrap().as_mut().prev } {
                Some(x) => unsafe { x.as_mut().next = node },
                None => self.list.head = node,
            }
            unsafe { self.node.unwrap().as_mut().prev = node };
        }
    }

    pub fn seek_forward(&self, _element: T) -> bool {
        false
    }

    pub fn seek_backward(&self, _element: T) -> bool {
        false
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.len == 0 {
            None
        } else {
            self.head.map(|node| {
                self.len -= 1;

                unsafe {
                    let node = &*node.as_ptr();
                    self.head = node.next;
                    &node.val
                }
            })
        }
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        while let Some(node) = self.pop_front() {
            drop(node);
        }
    }
}
