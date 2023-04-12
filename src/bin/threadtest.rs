use std::{sync::mpsc, thread, time};

pub fn sleep_print(name: &str) {
    for i in 1..=3 {
        println!("{}: i={}", name, i);
        thread::sleep(time::Duration::from_millis(1000));
    }
}

pub fn sleep_sender(name: &str, sender: mpsc::Sender<String>) {
    for i in 1..=5 {
        let msg = format!("{}: {}", name, i);
        sender.send(msg).unwrap();
        thread::sleep(time::Duration::from_millis(1000));
    }
    sender.send("quit".to_string()).unwrap();
}

pub fn fib(n: i64) -> i64 {
    if n == 1 {
        return 0;
    }
    if n == 2 {
        return 1;
    }
    return fib(n - 2) + fib(n - 1);
}

pub fn show_time(start_time: time::Instant) {
    let elapsed = start_time.elapsed();
    println!("execution time: {:?}", elapsed);
}

fn main() {
    // let request_nums = [43, 42, 20, 39, 37, 35, 30];
    // let start_time = time::Instant::now();

    // // no thread
    // for num in request_nums {
    //     let answer = fib(num);
    //     println!("Result: fib({})={}", num, answer);
    // }
    // show_time(start_time);
    //

    let request_nums = [43, 42, 20, 39, 37, 35, 30];
    let start_time = time::Instant::now();
    let (tx, rx) = mpsc::channel::<(i64, i64)>();
    for num in request_nums {
        let sender = tx.clone();
        thread::spawn(move || {
            let answer = fib(num);
            sender.send((num, answer)).unwrap();
        });
    }

    let mut job = request_nums.len();
    loop {
        if let Ok((arg, answer)) = rx.recv() {
            job -= 1;
            println!("Result fib({})={} (remaining={})", arg, answer, job);
            if job <= 0 {
                show_time(start_time);
                break;
            }
        }
        thread::sleep(time::Duration::from_millis(300));
    }
}
