use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use args::Args;
use clap::Parser;
use reqwest::{Client, Proxy, Version};

mod args;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let client = Client::new();
    let mut tests = Vec::new();

    for url in &args.urls {
        let response = client
            .get(url)
            .version(Version::HTTP_10)
            .send()
            .await
            .expect("error sending request")
            .text()
            .await
            .expect("error receiving response");

        tests.push((url.clone(), response));
    }

    let mut handles = Vec::new();

    let proxy = Proxy::http(format!("http://localhost:{}", args.port))
        .expect("error building proxy for reqwest::Client");

    let client = Client::builder()
        .proxy(proxy)
        .build()
        .expect("error building reqwest::Client with proxy");

    let rps = args.rps;
    let client = Arc::new(client);
    let counter = Arc::new(AtomicU64::new(0));

    let timer = tokio::time::Instant::now();

    for (url, expected) in tests {
        handles.push(tokio::spawn({
            let client = client.clone();
            let counter = counter.clone();
            async move {
                loop {
                    let start = tokio::time::Instant::now();

                    let actual = client
                        .get(&url)
                        .version(Version::HTTP_10)
                        .send()
                        .await
                        .expect("error sending request")
                        .text()
                        .await
                        .expect("error receiving response");

                    assert_eq!(actual, expected);
                    counter.fetch_add(1, Ordering::Relaxed);

                    if rps < 1000 {
                        tokio::time::sleep_until(start + Duration::from_millis(1000 / rps as u64))
                            .await;
                    }
                }
            }
        }));
    }

    tokio::signal::ctrl_c().await.unwrap();

    let elapsed = timer.elapsed();
    let count = counter.load(Ordering::Relaxed);
    let rps = count as f32 / elapsed.as_secs_f32();

    println!();
    println!("was requesting {} urls:", args.urls.len());

    for url in args.urls {
        println!("{url}");
    }

    println!("elapsed time: {:?}", elapsed);
    println!("# responses received: {}", count);
    println!("RPS: {}", rps);
}
