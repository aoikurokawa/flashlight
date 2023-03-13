pub fn verse(n: u32) -> String {
    let mut str = String::new();

    if n == 0 {
        str = format!("No more bottles of beer on the wall, no more bottles of beer.\nGo to the store and buy some more, 99 bottles of beer on the wall.\n");
    } else if n == 1 {
        str = format!("1 bottle of beer on the wall, 1 bottle of beer.\nTake it down and pass it around, no more bottles of beer on the wall.\n");
    } else if n == 2 {
        str = format!("2 bottles of beer on the wall, 2 bottles of beer.\nTake one down and pass it around, 1 bottle of beer on the wall.\n");
    } else {
        str = format!("{} bottles of beer on the wall, {} bottles of beer.\nTake one down and pass it around, {} bottles of beer on the wall.\n", n, n, n-1);
    }

    str
}

pub fn sing(start: u32, end: u32) -> String {
    let str = String::new();

    str
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verse_0() {
        assert_eq!(verse(0), "No more bottles of beer on the wall, no more bottles of beer.\nGo to the store and buy some more, 99 bottles of beer on the wall.\n");
    }

    #[test]
    fn test_verse_1() {
        assert_eq!(verse(1), "1 bottle of beer on the wall, 1 bottle of beer.\nTake it down and pass it around, no more bottles of beer on the wall.\n");
    }

    #[test]
    fn test_verse_2() {
        assert_eq!(verse(2), "2 bottles of beer on the wall, 2 bottles of beer.\nTake one down and pass it around, 1 bottle of beer on the wall.\n");
    }

    #[test]
    fn test_verse_8() {
        assert_eq!(verse(8), "8 bottles of beer on the wall, 8 bottles of beer.\nTake one down and pass it around, 7 bottles of beer on the wall.\n");
    }
}
