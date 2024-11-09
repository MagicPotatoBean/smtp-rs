use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, time::Duration};

use regex::Regex;

fn main() {
    let mut file = std::fs::OpenOptions::new().append(true).create(true).open("./email_log").unwrap();
    let listener = TcpListener::bind("0.0.0.0:25").unwrap();

    for mut incoming in listener.incoming().flatten() {
        incoming.write_all(b"220 zoe.soutter.com ESMTP Postfix\r\n").unwrap();
        file.write_all(b"C: 220 zoe.soutter.com ESMTP Postfix\r\n").unwrap();
        let data = read_timeout(&mut incoming);
        println!("Intro: {}", String::from_utf8_lossy(&data));
        write!(file, "S: {}", String::from_utf8_lossy(&data)).unwrap();
        incoming.write_all(b"250 Hello ").unwrap();
        incoming.write_all(&data[5..(data.len() - 2)]).unwrap();
        incoming.write_all(b", I am glad to meet you\r\n").unwrap();
        file.write_all(b"C: 250 Hello ").unwrap();
        file.write_all(&data[5..(data.len() - 2)]).unwrap();
        file.write_all(b", I am glad to meet you\r\n").unwrap();
        loop {
            let data = read_timeout(&mut incoming);
            if data == b"DATA\r\n" {
                file.write_all(b"S: DATA\r\n").unwrap();
                break;
            }
            incoming.write_all(b"250 Ok\r\n").unwrap();
            file.write_all(b"C: 250 Ok\r\n").unwrap();
            println!("S: {}", String::from_utf8_lossy(&data));
        write!(file, "S: {}", String::from_utf8_lossy(&data)).unwrap();
        }
        incoming.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n").unwrap();
        file.write_all(b"C: 354 End data with <CR><LF>.<CR><LF>\r\n").unwrap();
        let data = read_timeout(&mut incoming);

        let x = regex::bytes::Regex::new(r"[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();
        let urls = x.find(&data);

        println!("{}", String::from_utf8_lossy(&data));
        write!(file, "S: {}", String::from_utf8_lossy(&data)).unwrap();
        incoming.write_all(b"250 Ok: queued as 12345\r\n").unwrap();
        file.write_all(b"C: 250 Ok: queued as 12345\r\n").unwrap();
        let data = read_timeout(&mut incoming);
        println!("{}", String::from_utf8_lossy(&data));
        write!(file, "{}", String::from_utf8_lossy(&data)).unwrap();
        incoming.write_all(b"221 Bye\r\n").unwrap();
        file.write_all(b"C: 221 Bye\r\n").unwrap();
        incoming.shutdown(std::net::Shutdown::Both).unwrap();
        println!("CONNECTION CLOSED");
        write!(file, "CONNECTION CLOSED").unwrap();

        for url in urls.iter() {
            println!("URL FOUND: {}", String::from_utf8_lossy(url.as_bytes()));
        }
    }
}
fn read_timeout(stream: &mut TcpStream) -> Vec<u8> {
    let mut buffer = Vec::new();
    stream.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
    if let Err(err) = stream.read_to_end(&mut buffer) {
        if let std::io::ErrorKind::WouldBlock = err.kind() {

        } else {
            println!("{err:?}");
        }
    }
    buffer
}
struct EmailAddress {
    username: String,
    domain: String,
}
struct IncomingEmail {
    to_address: EmailAddress,
    from_address: EmailAddress,
}
