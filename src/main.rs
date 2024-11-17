#![feature(iter_map_windows)]
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    ops::Shl,
    time::Duration,
};

use regex::Regex;

fn main() {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("./email_log")
        .unwrap();
    let listener = TcpListener::bind("0.0.0.0:25").unwrap();

    for mut incoming in listener.incoming().flatten() {
        let email = match parse_smtp_packet(&mut incoming) {
            Ok(email) => email,
            Err(err) => {
                println!("Hit error: {err:?}");
                continue;
            }
        };
        println!(
            "{}\n============================================================================",
            String::from_utf8_lossy(&email.body)
        );
        let x = regex::bytes::Regex::new(r"(https?://)?[-a-zA-Z0-9%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();
        let urls = x.find_iter(&email.body);
        for url in urls {
            println!("URL found: {}", String::from_utf8_lossy(url.as_bytes()));
        }
    }
}
fn read_timeout(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    if let Err(err) = stream.read_to_end(&mut buffer) {
        if std::io::ErrorKind::WouldBlock != err.kind() {
            println!("{err:?}");
        }
    }
    Ok(buffer)
}
fn parse_smtp_packet(stream: &mut TcpStream) -> std::io::Result<IncomingEmail> {
    stream.write_all(b"220 zoe.soutter.com ESMTP Postfix\r\n")?;
    let data = read_timeout(stream)?;
    if data == b"" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Client sent no data",
        ));
    }
    println!("Got request, introduced self");
    if data.len() <= 5 {
        println!(
            "Recieved: DBG:\"{:?}\" ~= \"{}\"",
            data,
            String::from_utf8_lossy(&data)
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Client returned too short of a response",
        ));
    };
    let name = &data[5..(data.len() - 2)];
    println!("End user introduced: \"{}\"", String::from_utf8_lossy(name));
    stream.write_all(
        format!(
            "250 Hello {}, I am glad to meet you\r\n",
            String::from_utf8_lossy(&name)
        )
        .as_bytes(),
    )?;
    let mut recipients = Vec::new();
    let mut sender = None;
    loop {
        let data = read_timeout(stream)?;
        if let Ok(val) = prse::try_parse!(String::from_utf8_lossy(&data), "MAIL FROM:<{}>\r\n") {
            println!("MAIL FROM:<{val}>");
            let val: String = val;
            if let Some((username, domain)) = val.split_once("@") {
                sender = Some(EmailAddress {
                    username: username.to_string(),
                    domain: domain.to_string(),
                })
            }
            stream.write_all(b"250 Ok\r\n")?;
        } else if let Ok(val) = prse::try_parse!(String::from_utf8_lossy(&data), "RCPT TO:<{}>\r\n")
        {
            println!("RCPT TO:<{val}>");
            let val: String = val;
            if let Some((username, domain)) = val.split_once("@") {
                recipients.push(EmailAddress {
                    username: username.to_string(),
                    domain: domain.to_string(),
                })
            }
            stream.write_all(b"250 Ok\r\n")?;
        } else if &data == b"DATA\r\n" {
            let data = read_timeout(stream)?;
            stream.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")?;
            break;
        } else {
            stream.write_all(b"250 Ok\r\n")?;
        }
    }
    let Some(sender) = sender else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Client addressed no sender",
        ));
    };
    if recipients.len() == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Client addressed no recipients",
        ));
    }
    println!("Reading body");
    let body_data = read_timeout(stream)?;
    let mut body_data = String::from_utf8_lossy(&body_data).replace("=\r\n", "");

    let mut skip_n = 0;
    body_data = body_data
        .chars()
        .map_windows(|&[eq, val1, val2]| {
            if skip_n == 0 {
                if eq == '=' {
                    if let (Some(a), Some(b)) = (val1.to_digit(16), val2.to_digit(16)) {
                        let chr = char::from_u32(a.shl(4) + b).unwrap();
                        skip_n = 2;
                        println!("FOUND ESCAPE CODE: {eq}{val1}{val2} replaced with {chr}");
                        Some(chr)
                    } else {
                        Some(eq)
                    }
                } else {
                    Some(eq)
                }
            } else {
                skip_n -= 1;
                None
            }
        })
        .flatten()
        .collect();
    stream.write_all(b"250 Ok: Queued as\r\n")?;
    for recipient in recipients.iter().cloned() {
        if recipient.is_safe() && recipient.domain == "zoe.soutter.com" && sender.is_safe() {
            let time = chrono::Local::now().format("%Y.%m.%d-%H:%M:%S").to_string();
            let path = format!(
                "./inboxes/{}@{}/{}@{}-{}.email",
                recipient.username, recipient.domain, sender.username, sender.domain, time
            );
            match std::fs::create_dir(format!(
                "./inboxes/{}@{}",
                recipient.username, recipient.domain
            )) {
                Ok(_) => {}
                Err(_) => {}
            }
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                file.write_all(body_data.as_bytes())?;
            } else {
                println!("Failed to create email file for {}", path);
            }
        }
        println!("Unsafe client {}@{}", recipient.username, recipient.domain);
    }
    let response = read_timeout(stream)?;
    if response == b"QUIT\r\n" {
        stream.write_all(b"221 Bye\r\n")?;
        stream.shutdown(std::net::Shutdown::Both)?;
        return Ok(IncomingEmail {
            to_addresses: recipients,
            from_address: sender,
            body: body_data.into(),
        });
    } else {
        println!("Client didnt call QUIT, forcing shutdown");
        stream.shutdown(std::net::Shutdown::Both)?;
        return Ok(IncomingEmail {
            to_addresses: recipients,
            from_address: sender,
            body: body_data.into(),
        });
    }
}
#[derive(Clone, Debug)]
struct EmailAddress {
    username: String,
    domain: String,
}
impl EmailAddress {
    fn is_safe(&self) -> bool {
        println!("username: {:?}", self.username);
        println!("domain: {:?}", self.domain);
        //self.username.chars().all(|chr| {
        //    chr.is_alphanumeric() || chr == '+' || chr == '-' || chr == '_' || chr == '.'
        //}) && self.domain.chars().all(|chr| {
        //    chr.is_alphanumeric() || chr == '+' || chr == '-' || chr == '_' || chr == '.'
        //})
        true
    }
}
#[derive(Clone, Debug)]
struct IncomingEmail {
    to_addresses: Vec<EmailAddress>,
    from_address: EmailAddress,
    body: Vec<u8>,
}
