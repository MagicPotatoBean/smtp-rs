use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, time::Duration};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:25").unwrap();
    for mut incoming in listener.incoming().flatten() {
        incoming.write_all(b"220 zoe.soutter.com ESMTP Postfix\r\n").unwrap();
        let data = read_timeout(&mut incoming);
        println!("{}", String::from_utf8_lossy(&data));
        incoming.write_all(b"250 Hello ").unwrap();
        incoming.write_all(&data).unwrap();
        incoming.write_all(b", I am glad to meet you\r\n").unwrap();
        loop {
            let data = read_timeout(&mut incoming);
            println!("{}", String::from_utf8_lossy(&data));
            if data == b"DATA\r\n" {
                break;
            }
        }
        incoming.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n").unwrap();

        println!("{}", String::from_utf8_lossy(&data));
    }
}
fn read_timeout(stream: &mut TcpStream) -> Vec<u8> {
    let mut buffer = Vec::new();
    stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let _ = stream.read_to_end(&mut buffer);
    buffer
}
