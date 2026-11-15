use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const ENDPOINT: &str = "127.0.0.1:45871";

fn usage() {
    eprintln!(
        "isle-hook — send agent events to the Isle island\n\
         \n\
         Usage:\n\
         \x20 isle-hook <source> <event> [detail]\n\
         \x20 echo '{{\"source\":\"claude\",\"event\":\"stop\",\"title\":\"Done\"}}' | isle-hook\n\
         \x20 isle-hook --ping\n\
         \n\
         Examples:\n\
         \x20 isle-hook claude stop \"Task finished\"\n\
         \x20 isle-hook codex notify \"Waiting for input\"\n\
         \x20 isle-hook opencode build \"Tests passed\"\n"
    );
}

fn request(method: &str, path: &str, body: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(ENDPOINT)
        .map_err(|error| format!("Isle is not running ({ENDPOINT}): {error}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let payload = format!(
        "{method} {path} HTTP/1.1\r\nHost: {ENDPOINT}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(payload.as_bytes())
        .map_err(|error| format!("Could not send the event: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("Could not read the response: {error}"))?;
    Ok(response)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        let mut input = String::new();
        if std::io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
            usage();
            std::process::exit(2);
        }
        match request("POST", "/event", input.trim()) {
            Ok(response) if response.starts_with("HTTP/1.1 200") => {
                println!("Isle: event delivered");
            }
            Ok(response) => {
                eprintln!("Isle: rejected ({})", response.lines().next().unwrap_or("?"));
                std::process::exit(1);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    if args[0] == "--help" || args[0] == "-h" {
        usage();
        return;
    }

    if args[0] == "--ping" {
        match request("GET", "/health", "") {
            Ok(response) if response.starts_with("HTTP/1.1 200") => println!("Isle: online"),
            Ok(response) => {
                eprintln!("Isle: unexpected response ({})", response.lines().next().unwrap_or("?"));
                std::process::exit(1);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    if args[0] == "--clear" {
        match request("POST", "/clear", "") {
            Ok(response) if response.starts_with("HTTP/1.1 200") => println!("Isle: events cleared"),
            Ok(response) => {
                eprintln!("Isle: unexpected response ({})", response.lines().next().unwrap_or("?"));
                std::process::exit(1);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

    let source = args.first().cloned().unwrap_or_else(|| "agent".into());
    let event = args.get(1).cloned().unwrap_or_else(|| "notice".into());
    let detail = args.get(2).cloned().unwrap_or_default();
    let body = serde_json::json!({
        "source": source,
        "event": event,
        "title": format!("{source}: {event}"),
        "detail": detail,
    })
    .to_string();

    match request("POST", "/event", &body) {
        Ok(response) if response.starts_with("HTTP/1.1 200") => {
            println!("Isle: event delivered");
        }
        Ok(response) => {
            eprintln!("Isle: rejected ({})", response.lines().next().unwrap_or("?"));
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
