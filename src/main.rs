mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use tokio::sync::mpsc::{self, Receiver};

use models::network::{Network, GlobalEvent};
use tokio::spawn;

#[tokio::main]
async fn start_tokio(rx: Receiver<GlobalEvent>) {
    let mut network = Network::new(rx);
    network.daemon().await;
}

#[tokio::main]
async fn main() -> Result<(), String> {
    pretty_env_logger::init();

    let (tx, rx) = mpsc::channel::<GlobalEvent>(200);

    let daemon_handle = std::thread::spawn(move || {
        start_tokio(rx);
    });

    // let daemon_handle = spawn(async move {
    //     let mut network = Network::new().await;
    //     network.daemon(rx).await;
    // });

    let frontend_handle = spawn(async move {
        crossterm::run(&tx).await.unwrap();
    });

    // daemon_handle.await.expect("to listen to daemon_handle");
    frontend_handle.await.expect("to listen to frontend_handle");
    daemon_handle.join().unwrap();

    Ok(())
}
