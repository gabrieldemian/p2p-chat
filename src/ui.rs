use std::{io, time::Duration};

use crate::{
    app::{AppMessage, AppStyle},
    chat_room::ChatRoom,
    models::network::NetworkMessage,
    topic_list::TopicList,
};
use crossterm::event::{self, Event};
use tokio::sync::mpsc::Sender;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn draw_topic_list<'a>(
    page: &mut TopicList<'a>,
    tx: &Sender<AppMessage<'a>>,
    tx_network: &Sender<NetworkMessage>,
    timeout: Duration,
) {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let style = AppStyle::new();

    terminal.draw(|f| page.draw(f, &style)).unwrap();

    if event::poll(timeout).unwrap() {
        if let Event::Key(k) = event::read().unwrap() {
            page.keybindings(k.code, &tx, tx_network).await;
        }
    }
}

pub async fn draw_chat_room<'a>(
    page: &mut ChatRoom,
    tx: &Sender<AppMessage<'a>>,
    tx_network: &Sender<NetworkMessage>,
    timeout: Duration,
) {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let style = AppStyle::new();

    terminal.draw(|f| page.draw(f, &style)).unwrap();

    if event::poll(timeout).unwrap() {
        if let Event::Key(k) = event::read().unwrap() {
            page.keybindings(k.code, &tx, tx_network).await;
        }
    }
}
