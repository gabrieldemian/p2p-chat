mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use models::network::Network;
use tokio::{spawn, sync::broadcast};

#[derive(Debug, Clone)]
pub enum GlobalEvent {
    MessageReceived(String),
    Quit,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let (tx, rx) = broadcast::channel::<GlobalEvent>(3);
    let tx2 = tx.clone();
    let rx2 = tx2.subscribe();

    let mut network = Network::new().await;

    let daemon_handle = spawn(async move {
        network.daemon(tx, rx).await;
    });

    let frontend_handle = spawn(async move { crossterm::run(tx2, rx2).await });

    daemon_handle.await.expect("to listen to daemon_handle");
    frontend_handle.await.expect("to listen to frontend_handle").unwrap();

    Ok(())
}
