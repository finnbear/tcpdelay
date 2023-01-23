#![feature(async_closure)]

use rand::{thread_rng, Rng};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::time::delay_queue::DelayQueue;

/// Simulates latency on proxied TCP connections.
#[derive(StructOpt)]
struct Options {
    /// Upstream TCP port on localhost (to forward connections from).
    #[structopt(short, long, default_value = "8081")]
    upstream: u16,
    /// Downstream TCP port on localhost (to forward connections to).
    #[structopt(short, long, default_value = "8080")]
    downstream: u16,
    /// Base one-way latency (millis).
    #[structopt(short, long, default_value = "75")]
    latency: u64,
    /// Max additional one-way latency (millis).
    #[structopt(short, long, default_value = "25")]
    jitter: u64,
    /// Don't log anything.
    #[structopt(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::from_args();

    let random_delay = move |prev: &mut Instant| -> tokio::time::Instant {
        let duration =
            Duration::from_millis(options.latency + thread_rng().gen_range(0..=options.jitter));
        let time = Instant::now() + duration;
        *prev = (*prev).max(time);
        prev.clone().into()
    };

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, options.upstream);
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut upstream, client) = listener.accept().await?;

        if !options.quiet {
            println!("{client} - connected");
        }

        let proxy = async move || -> Result<(), Box<dyn std::error::Error>> {
            struct Item {
                bytes: Vec<u8>,
                to_upstream: bool,
            }

            let mut downstream =
                TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, options.downstream))
                    .await?;
            let mut queue = DelayQueue::new();

            // Hack: make sure the queue is never empty.
            queue.insert(
                Item {
                    bytes: Vec::new(),
                    to_upstream: false,
                },
                Duration::from_secs(3600 * 24 * 365),
            );

            let mut upstream_buf = [0; 1024];
            let mut downstream_buf = [0; 1024];
            let mut upstream_prev = Instant::now();
            let mut downstream_prev = Instant::now();

            loop {
                tokio::select! {
                    read_result = upstream.read(&mut upstream_buf) => {
                        let n = match read_result? {
                            0 => return Ok(()),
                            n => n
                        };

                        queue.insert_at(Item{
                            bytes: upstream_buf[0..n].to_owned(),
                            to_upstream: false,
                        }, random_delay(&mut upstream_prev));
                    },
                    read_result = downstream.read(&mut downstream_buf) => {
                        let n = match read_result? {
                            0 => return Ok(()),
                            n => n
                        };

                        queue.insert_at(Item{
                            bytes: downstream_buf[0..n].to_owned(),
                            to_upstream: true,
                        }, random_delay(&mut downstream_prev));
                    },
                    item = std::future::poll_fn(|ctx| {
                        queue.poll_expired(ctx)
                    }) => {
                        let item = item.unwrap().into_inner();
                        if item.to_upstream {
                            upstream.write_all(&item.bytes).await?;
                        } else {
                            downstream.write_all(&item.bytes).await?;
                        }
                    }

                }
            }
        };

        tokio::spawn(async move {
            let result = proxy().await;
            if !options.quiet {
                if let Err(e) = result {
                    println!("{client} - error ({e})");
                } else {
                    println!("{client} - disconnected");
                }
            }
        });
    }
}
