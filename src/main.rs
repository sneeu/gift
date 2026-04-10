#[tokio::main]
async fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("gift {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let config = match gift::config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("gift: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = gift::run::run(config).await {
        eprintln!("gift: {e}");
        std::process::exit(1);
    }
}
