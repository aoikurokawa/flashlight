macro_rules! echo_num {
    ($($num:expr), *) => {
        $(
       print!("{}, ", $num);
) *
        println!("");
    }
}

macro_rules! easy_for {
    (
    for $i:ident = $from:tt to $to:tt
    $block:block

) => {{
        for $i in $from..=$to {
            $block
        }
    }};

    // for i = 1 to 10 step 2
    (
    for $i:ident = $from:tt to $to:tt step $step:tt
    $block:block
) => {{
    let mut $i = $from;
    loop {
        if $i > $to {break}
        $block
        $i += $step
    }
}};
}

macro_rules! map_init {
    ($($key: expr => $val:expr), *) => {{
        let mut tmp = std::collections::HashMap::new();
        $(
            tmp.insert($key, $val);
        ) *
        tmp
    }};
}

fn main() {
    echo_num![10, 20, 30];

    let mut total = 0;
    easy_for! {
        for i = 1 to 10 {
            total += i;
        }
    }
    println!("{total}");

    easy_for! {
        for i = 0 to 10 step 3 {
            println!("i = {i}");
        }
    }

    let week = map_init!("mon" => "Monday", "tue" => "Tuesday");
    println!("{week:?}");
}
