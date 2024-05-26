pub trait Hei {
    fn hei(&self);
}

impl Hei for &str {
    fn hei(&self) {
        println!("hei {}", self);
    }
}

impl Hei for String {
    fn hei(&self) {
        println!("hei {}", self);
    }
}

fn main() {
    // let hello = String::from("hello");
    // hello.hei();
    let x: [&dyn Hei; 2] = [&"hello", &String::from("hei")];
    bar(&x);
}

pub fn strlen<S: AsRef<str>>(s: S) -> usize {
    s.as_ref().len()
}

pub fn bar(s: &[&dyn Hei]) {
    for elem in s {
        elem.hei();
    }
}

pub trait HeiAsRef: Hei + AsRef<str> {}

pub fn baz(s: &dyn HeiAsRef) {
    s.hei();
    let s = s.as_ref();
    let n = s.len();

    println!("{n}");
}

pub fn drop(v: &mut dyn Drop) {}

pub fn say_hei(s: Box<dyn AsRef<str>>) {}

// dynimaically sized struct
struct Foo {
    f: bool,
    b: bool,
    t: [u8],
}

// Box<[u8]> != Vec<u8>
//
// dyn Fn != fn

fn foo(f: &dyn Fn()) {}

fn bar1(f: fn()) {}

// coherence

fn call(f: &dyn Fn()) {
    f()
}

fn say_hi(v: &[&dyn AsRef<str>]) {
    for s in v {
        s.as_ref();
    }
}
