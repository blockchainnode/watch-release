use log::error;
use watch_release::cli;
#[tokio::main]
async fn main() {
    cli::init_log();
    if let Err(e) = cli::run().await {
        error!("Error: {:?}. shutting down ...", e);
        std::process::exit(1);
    }
}
