use crate::{chat_room::ChatRoom, topic_list::*};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tui::{
    style::{Color, Style},
    widgets::StatefulWidget,
};

#[derive(Clone, Debug)]
pub struct PageWidget<W, I>
where
    W: StatefulWidget,
{
    pub state: W::State,
    pub items: I,
}

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

#[derive(Debug, Clone)]
pub enum AppEvent<'a> {
    ChangePage(Page<'a>),
    Quit,
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

pub struct App<'a> {
    pub style: AppStyle,
    pub page: Page<'a>,
    pub should_close: bool,
    pub rx: Receiver<AppEvent<'a>>,
    pub tx: Sender<AppEvent<'a>>,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let style = AppStyle::new();
        let topic_list = TopicList::new();
        let page = Page::TopicList(topic_list);

        let (tx, rx) = mpsc::channel::<AppEvent>(20);

        App {
            style,
            page,
            should_close: false,
            tx,
            rx,
        }
    }

    pub fn change_page(&mut self, page: Page<'a>) {
        self.page = page;
    }
}
