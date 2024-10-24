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

    test_sync(&args, tests.clone()).await;
    test_rps(&args, tests.clone()).await;
}

fn get_proxy_client(port: u16) -> Arc<Client> {
    let proxy = Proxy::http(format!("http://localhost:{}", port))
        .expect("error building proxy for reqwest::Client");

    let client = Client::builder()
        .proxy(proxy)
        .build()
        .expect("error building reqwest::Client with proxy");

    Arc::new(client)
}

async fn test_sync(args: &Args, tests: Vec<(String, String)>) {
    let clients = args.clients;
    let port = args.port;

    for (url, expected) in tests {
        let handle = tokio::spawn(async move {
            let client = get_proxy_client(port);

            let mut handles = Vec::new();

            for _ in 0..clients {
                handles.push(tokio::spawn({
                    let client = client.clone();
                    let expected = expected.clone();
                    let url = url.clone();

                    async move {
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
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }
        });

        handle.await.unwrap();
    }

    println!(
        "successfully got responses with {} clients for {} urls",
        args.clients,
        args.urls.len()
    );
}

async fn test_rps(args: &Args, tests: Vec<(String, String)>) {
    let client = get_proxy_client(args.port);
    let rps = args.rps;
    let client = Arc::new(client);
    let counter = Arc::new(AtomicU64::new(0));

    let timer = tokio::time::Instant::now();

    for (url, expected) in tests {
        tokio::spawn({
            let client = client.clone();
            let counter = counter.clone();
            async move {
                let request = client
                    .get(&url)
                    .version(Version::HTTP_10)
                    .build()
                    .expect("error building request");

                loop {
                    let start = tokio::time::Instant::now();

                    let actual = client
                        .execute(request.try_clone().unwrap())
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
        });
    }

    tokio::signal::ctrl_c().await.unwrap();

    let elapsed = timer.elapsed();
    let count = counter.load(Ordering::Relaxed);
    let rps = count as f32 / elapsed.as_secs_f32();

    println!();
    println!("was requesting {} urls:", args.urls.len());

    for url in &args.urls {
        println!("{url}");
    }

    println!("elapsed time: {:?}", elapsed);
    println!("# responses received: {}", count);
    println!("RPS: {}", rps);
}
