use std::{borrow::Borrow, iter::Cycle, slice::Iter};

#[derive(Clone)]
pub struct Xorcism<'a> {
    key: Cycle<Iter<'a, u8>>,
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    ///
    /// Should accept anything which has a cheap conversion to a byte slice.
    pub fn new<Key>(key: &'a Key) -> Xorcism<'a>
    where
        Key: AsRef<[u8]> + ?Sized,
    {
        Xorcism {
            key: key.as_ref().iter().cycle(),
        }
    }

    /// XOR each byte of the input buffer with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    pub fn munge_in_place(&mut self, data: &mut [u8]) {
        data.iter_mut().for_each(|b| *b ^= self.key.next().unwrap())
    }

    /// XOR each byte of the data with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    ///
    /// Should accept anything which has a cheap conversion to a byte iterator.
    /// Shouldn't matter whether the byte iterator's values are owned or borrowed.
    pub fn munge<'b, Data>(&'b mut self, data: Data) -> impl Iterator<Item = u8> + Captures<'a> + 'b
    where
        Data: IntoIterator,
        Data::IntoIter: 'b,
        Data::Item: Borrow<u8>,
    {
        data.into_iter()
            .map(move |b| b.borrow() ^ self.key.next().unwrap())
    }
}

pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}
