use std::{
    io::{stdin, BufRead, BufReader, Write},
    net::TcpStream,
    thread, time,
};

fn main() {
    let server_addr = "127.0.0.1:8888";
    let mut socket = TcpStream::connect(server_addr).expect("Can not connect to server");
    socket.set_nonblocking(true).expect("Not available");
    println!("Connect: {}", server_addr);

    start_thread(socket.try_clone().unwrap());

    let user = input("What is your name?");
    println!("{}, please type any message", user);
    loop {
        let msg = input("");
        let msg = format!("{}> {}\n", user, msg);
        let buf = msg.as_bytes();
        socket.write_all(buf).unwrap();
    }
}

fn start_thread(socket: TcpStream) {
    let mut reader = BufReader::new(socket);
    thread::spawn(move || loop {
        let mut buf = String::new();
        if let Ok(n) = reader.read_line(&mut buf) {
            if n > 0 {
                println!("Got {}", buf.trim());
            }
        }
        thread::sleep(time::Duration::from_millis(100));
    });
}

fn input(msg: &str) -> String {
    if !msg.is_empty() {
        println!("{}", msg);
    }
    let mut buf = String::new();
    stdin().read_line(&mut buf).expect("Input error");
    String::from(buf.trim())
}
