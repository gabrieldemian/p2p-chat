mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use tokio::sync::mpsc::{self, Receiver, Sender};

use models::network::{GlobalEvent, Network};
use tokio::spawn;

#[tokio::main]
async fn start_tokio(rx: Receiver<GlobalEvent>, tx: Sender<BkEvent>, tx_global: Sender<GlobalEvent>) {
    let mut network = Network::new(rx, tx_global);
    network.daemon(tx).await;
}

#[derive(Debug, Clone)]
pub enum BkEvent {
    MessageReceived(String),
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    // frontend will send events to backend
    let (tx, rx) = mpsc::channel::<GlobalEvent>(200);

    // backend will send events to frontend
    let (tx_bk, rx_bk) = mpsc::channel::<BkEvent>(200);

    let tx_global = tx.clone();
    let daemon_handle = std::thread::spawn(move || {
        start_tokio(rx, tx_bk, tx_global);
    });

    let frontend_handle = spawn(async move {
        crossterm::run(&tx, rx_bk).await.unwrap();
    });

    // daemon_handle.await.expect("to listen to daemon_handle");
    frontend_handle.await.expect("to listen to frontend_handle");
    daemon_handle.join().unwrap();

    Ok(())
}
