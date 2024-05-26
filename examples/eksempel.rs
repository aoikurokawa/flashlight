pub fn strlen(s: impl AsRef<str>) -> usize {
    s.as_ref().len()
}

pub fn strlen2<S>(s: S) -> usize
where
    S: AsRef<str>,
{
    s.as_ref().len()
}



fn main() {
    let _n = strlen("hello"); // &'static str
    let n = strlen(String::from("hei verden")); // String
    println!("{n}");
}
