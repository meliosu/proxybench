use clap::Parser;

const URLS: [&str; 4] = [
    "http://http.badssl.com",
    "http://majesticgrandastoundinglight.neverssl.com/online/",
    "http://icio.us",
    "http://httpforever.com",
];

fn urls() -> Vec<String> {
    URLS.iter().map(|s| s.to_string()).collect()
}

#[derive(Parser)]
pub struct Args {
    #[arg(short, long, default_values_t = urls())]
    pub urls: Vec<String>,

    #[arg(short, long, default_value_t = 100)]
    pub rps: u32,

    #[arg(short, long, default_value_t = 1080)]
    pub port: u16,
}
