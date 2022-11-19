mod models;
use clap::Parser;
use models::{cli::Opt, network::Network};
use tokio::spawn;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let opt = Opt::parse();
    let mut network = Network::new(&opt).await;

    let daemon_handle = spawn(async move { network.daemon().await });

    daemon_handle.await.expect("to listen to daemon_handle");
}
