use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread, time,
};

fn main() {
    let server_addr = "127.0.0.1:8888";
    let (tx, rx) = mpsc::channel::<String>();
    let mut clients: Vec<TcpStream> = Vec::new();

    let server = TcpListener::bind(server_addr).expect("fail to start server");
    server.set_nonblocking(true).expect("Not available");
    println!("start listening at: {}", server_addr);

    loop {
        if let Ok((client, addr)) = server.accept() {
            println!("client: {}", addr);
            clients.push(client.try_clone().unwrap());
            start_thread(client, tx.clone());
        }

        if let Ok(msg) = rx.try_recv() {
            println!("Send all: {}", msg.trim());
            clients = send_all(clients, &msg);
        }
        thread::sleep(time::Duration::from_millis(100));
    }
}

pub fn start_thread(client: TcpStream, tx: mpsc::Sender<String>) {
    let mut reader = BufReader::new(client);
    thread::spawn(move || loop {
        let mut msg = String::new();
        if let Ok(n) = reader.read_line(&mut msg) {
            if n > 0 {
                tx.send(msg).unwrap();
            }
        }
        thread::sleep(time::Duration::from_millis(100));
    });
}

pub fn send_all(clients: Vec<TcpStream>, s: &str) -> Vec<TcpStream> {
    let mut collector = vec![];
    for mut socket in clients.into_iter() {
        let bytes = String::from(s).into_bytes();
        if let Err(e) = socket.write_all(&bytes) {
            println!("Error: {e}");
            continue;
        }
        collector.push(socket);
    }
    collector
}
