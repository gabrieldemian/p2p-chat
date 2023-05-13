mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use libp2p::gossipsub::IdentTopic;
use models::network::Network;
use tokio::{spawn, sync::mpsc};

#[derive(Debug, Clone)]
pub enum GlobalEvent {
    Quit,
    MessageReceived(IdentTopic, String),
    Subscribed(IdentTopic),
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let (tx, rx) = mpsc::channel::<GlobalEvent>(20);
    let tx2 = tx.clone();

    let mut network = Network::new().await;

    let daemon_handle = spawn(async move {
        network.daemon(tx, rx).await;
    });

    let frontend_handle = spawn(async move {
        crossterm::run(&tx2).await.unwrap();
    });

    daemon_handle.await.expect("to listen to daemon_handle");
    frontend_handle.await.expect("to listen to frontend_handle");

    Ok(())
}
