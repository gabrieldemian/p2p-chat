mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use app::AppHandle;

use models::network::{GlobalEvent, Network};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn start_tokio(tx: Sender<GlobalEvent>, rx: Receiver<GlobalEvent>) {
    let mut network = Network::new(tx, rx);
    network.daemon().await;
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    // `Network` will own these channels, but
    // `Frontend` will also have a `tx` to it.
    let (tx, rx) = mpsc::channel::<GlobalEvent>(200);
    let tx_app = tx.clone();

    AppHandle::new(tx_app);

    let daemon_handle = std::thread::spawn(move || {
        start_tokio(tx, rx);
    });

    daemon_handle.join().unwrap();

    Ok(())
}
