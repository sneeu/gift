mod app;
mod cache;
mod config;
mod download;
mod events;
mod handlers;
mod preview;
mod run;
mod s3;
mod search;
mod ui;
mod widgets;

#[tokio::main]
async fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("gift {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let config = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("gift: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = run::run(config).await {
        eprintln!("gift: {e}");
        std::process::exit(1);
    }
}
