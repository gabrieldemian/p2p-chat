mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use app::{AppHandle, AppMessage};

use models::network::{GlobalEvent, Network};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn start_tokio(tx: Sender<GlobalEvent>, rx: Receiver<GlobalEvent>, tx_app: Sender<AppMessage>) {
    let mut network = Network::new(tx, rx);
    network.daemon(tx_app).await;
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    // `Network` will own these channels, but
    // `Frontend` will also have a `tx` to it.
    let (tx_network, rx_network) = mpsc::channel::<GlobalEvent>(200);
    let tx_network_cloned = tx_network.clone();

    // `Network` will communicate with the frontend,
    // using this `tx`.
    let (tx_app, rx_app) = mpsc::channel::<AppMessage>(200);
    let tx_app_cloned = tx_app.clone();

    AppHandle::new(tx_app, rx_app, tx_network);

    let daemon_handle = std::thread::spawn(move || {
        start_tokio(tx_network_cloned, rx_network, tx_app_cloned);
    });

    daemon_handle.join().unwrap();

    Ok(())
}
