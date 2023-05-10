#[macro_export]
macro_rules! hashmap {
    (  ) => {
        {
            ::std::collections::HashMap::new()
        }
    };
    ( $( $k:expr => $v:expr ),+ $(,)? ) => {
        {
            use ::std::collections::HashMap;
            let mut hm = HashMap::new();
            $(
                hm.insert($k, $v);
            )*
            hm
        }
    };
}

#[macro_export]
macro_rules! avec {
    () => {
        Vec::new()
    };
    ($($element:expr),+ $(,)*) => {{
        let mut vs: Vec<u32> = Vec::with_capacity($crate::avec![@COUNT; $($element),*]);
        $(vs.push($element);)*
        vs
    }};
    ($element:expr; $count:expr) => {{
        let mut vs: Vec<u32> = Vec::new();
        vs.resize($count, $element);
        vs
    }};

    (@COUNT; $($element:expr),*) => {
        <[()]>::len(&[$($crate::avec![@SUBST; $element]),*])
    };
    (@SUBST; $_element:expr) => { () };
}

#[test]
fn foo() {
    let x: Vec<u32> = avec!();
    assert!(x.is_empty());
}

#[test]
fn single() {
    let x: Vec<u32> = avec!(42);
    assert!(!x.is_empty());
    assert_eq!(x.len(), 1);
    assert_eq!(x[0], 42);
}

#[test]
fn double() {
    let x: Vec<u32> = avec!(42, 43);
    assert!(!x.is_empty());
    assert_eq!(x.len(), 2);
    assert_eq!(x[0], 42);
    assert_eq!(x[1], 43);
}

#[test]
fn traling() {
    let x: Vec<u32> = avec!(
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27,
    );
    assert!(!x.is_empty());
}

#[test]
fn clone_2() {
    let mut y = Some(42);
    let x: Vec<u32> = avec!(y.take().unwrap(); 2);
    assert!(!x.is_empty());
    assert_eq!(x.len(), 2);
    assert_eq!(x[0], 42);
    assert_eq!(x[1], 42);
}

/// ```compile_fail
/// let x: Vec<u32> = vecmac::avec![42; "foo"];
/// ```
#[allow(dead_code)]
struct CompileFailtTest;
