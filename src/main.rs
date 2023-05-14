mod app;
mod chat_room;
mod crossterm;
mod models;
mod topic_list;
mod ui;
use std::sync::Arc;

use app::App;
use tokio::sync::{
    mpsc::{self, Receiver},
    Mutex,
};

use models::network::{GlobalEvent, Network};
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

    // let app = Arc::new(Mutex::new(App::new()));

    let daemon_handle = std::thread::spawn(move || {
        start_tokio(rx);
    });

    let frontend_handle = spawn(async move {
        crossterm::run(&tx).await.unwrap();
    });

    // daemon_handle.await.expect("to listen to daemon_handle");
    frontend_handle.await.expect("to listen to frontend_handle");
    daemon_handle.join().unwrap();

    Ok(())
}
