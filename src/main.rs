mod models;
use models::network::Network;
use tokio::spawn;

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let mut network = Network::new().await;

    let daemon_handle = spawn(async move { network.daemon().await });

    daemon_handle.await.expect("to listen to daemon_handle");

    Ok(())
}
