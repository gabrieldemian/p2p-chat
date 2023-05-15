use std::{
    io::{self, Stdout},
    time::Duration,
};

use crate::{
    chat_room::ChatRoom,
    models::network::NetworkMessage,
    topic_list::*,
    ui::{draw_chat_room, draw_topic_list},
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::Instant,
};
use tui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    Terminal,
};

#[derive(Debug, Clone)]
pub enum Page<'a> {
    TopicList(TopicList<'a>),
    ChatRoom(ChatRoom),
}

pub struct AppStyle {
    pub base_style: Style,
    pub selected_style: Style,
    pub normal_style: Style,
}

impl AppStyle {
    pub fn new() -> Self {
        AppStyle {
            base_style: Style::default().fg(Color::Gray),
            selected_style: Style::default().bg(Color::LightBlue).fg(Color::DarkGray),
            normal_style: Style::default().fg(Color::LightBlue),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppMessage<'a> {
    ChangePage {
        page: Page<'a>,
        // respond_to: oneshot::Sender<Page<'a>>,
    },
    Quit,
    MessageReceived {
        message: String,
    },
}

// actor
pub struct App<'a> {
    pub style: AppStyle,
    pub page: Page<'a>,
    pub should_close: bool,
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    pub rx: Receiver<AppMessage<'a>>,
    pub tx: Sender<AppMessage<'a>>,
    pub tx_network: Sender<NetworkMessage>,
}

// handle
pub struct AppHandle<'a> {
    pub tx: Sender<AppMessage<'a>>,
}

impl<'a> App<'a>
where
    'a: 'static,
{
    pub fn new(
        rx: Receiver<AppMessage<'a>>,
        tx: Sender<AppMessage<'a>>,
        tx_network: Sender<NetworkMessage>,
    ) -> Result<App<'a>, std::io::Error> {
        let style = AppStyle::new();
        let topic_list = TopicList::new();
        let page = Page::TopicList(topic_list);

        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(App {
            style,
            page,
            should_close: false,
            terminal,
            rx,
            tx,
            tx_network,
        })
    }

    pub async fn run(
        &mut self,
        // tx: Sender<AppMessage<'a>>,
        // tx_network: Sender<GlobalEvent>,
    ) -> Result<(), std::io::Error> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            match &mut self.page {
                Page::TopicList(page) => {
                    draw_topic_list(page, &self.tx, &self.tx_network, tick_rate).await;
                }
                Page::ChatRoom(page) => {
                    draw_chat_room(page, &self.tx, &self.tx_network, timeout).await
                }
            };

            // try_recv is non-blocking
            if let Ok(msg) = self.rx.try_recv() {
                self.handle_message(msg).await;
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }

            if self.should_close {
                return Ok(());
            }
        }
    }

    async fn handle_message(&mut self, msg: AppMessage<'a>) {
        match msg {
            AppMessage::Quit => {
                disable_raw_mode().unwrap();
                execute!(
                    self.terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )
                .unwrap();
                self.terminal.show_cursor().unwrap();
                self.should_close = true;
                // send message to `Network`
                let _ = self.tx_network.send(NetworkMessage::Quit).await;
            }
            AppMessage::ChangePage { page } => {
                self.page = page;
            }
            // This message is sent from `Network`
            AppMessage::MessageReceived { message } => {
                if let Page::ChatRoom(page) = &mut self.page {
                    page.items.push(message);
                }
            }
        }
    }
}

impl<'a> AppHandle<'a>
where
    'a: 'static,
{
    pub fn new(
        tx: Sender<AppMessage<'a>>,
        rx: Receiver<AppMessage<'a>>,
        tx_network: Sender<NetworkMessage>,
    ) -> Self {
        let actor = App::new(rx, tx.clone(), tx_network);

        tokio::spawn(async move { actor.unwrap().run().await });

        Self { tx }
    }
}
