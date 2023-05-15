use crossterm::event::KeyCode;
use libp2p::gossipsub::IdentTopic;
use tokio::sync::mpsc::Sender;
use tui::{
    backend::Backend,
    layout::Constraint,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::{
    app::{AppMessage, AppStyle, Page},
    chat_room::ChatRoom,
    GlobalEvent,
};

#[derive(Clone, Debug)]
pub struct TopicList<'a> {
    pub state: TableState,
    pub items: Vec<Vec<&'a str>>,
}

impl<'a> Default for TopicList<'a> {
    fn default() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));

        let items = vec![
            vec!["5", "Rust async"],
            vec!["3", "How to cook better"],
            vec!["8", "Hiking organization"],
            vec!["2", "Secret meeting to rule to world"],
            vec!["1", "Talk about cats"],
        ];
        Self { state, items }
    }
}

impl<'a> TopicList<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn keybindings(
        &mut self,
        k: KeyCode,
        tx: &Sender<AppMessage<'a>>,
        tx_global: &Sender<GlobalEvent>,
    ) {
        match k {
            KeyCode::Char('q') | KeyCode::Esc => tx.send(AppMessage::Quit).await.unwrap(),
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Enter => {
                let topic_index = self.state.selected().unwrap().to_string();
                let topic = IdentTopic::new(topic_index.clone());

                let chat_room = Page::ChatRoom(ChatRoom::new(topic_index));

                tx_global
                    .send(GlobalEvent::Subscribed(topic))
                    .await
                    .unwrap();
                tx.send(AppMessage::ChangePage { page: chat_room })
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, style: &AppStyle) {
        let header_cells = ["Online", "Name"]
            .into_iter()
            .map(|h| Cell::from(h).style(style.normal_style));

        let header = Row::new(header_cells)
            .style(style.normal_style)
            .height(1)
            .bottom_margin(1);

        let rows = self.items.iter().map(|item| {
            let height = item
                .into_iter()
                .map(|content| content.chars().filter(|c| *c == '\n').count())
                .max()
                .unwrap_or(0)
                + 1;
            let cells = item.iter().map(|c| Cell::from(*c));
            Row::new(cells).height(height as u16)
        });

        let t = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Chat Rooms"))
            .highlight_style(style.selected_style)
            .style(style.base_style)
            .widths(&[Constraint::Percentage(10), Constraint::Length(90)]);

        f.render_stateful_widget(t, f.size(), &mut self.state);
    }

    pub fn next(&mut self) {
        let i = self
            .state
            .selected()
            .map_or(0, |v| if v != self.items.len() - 1 { v + 1 } else { 0 });
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = self
            .state
            .selected()
            .map_or(0, |v| if v == 0 { self.items.len() - 1 } else { v - 1 });
        self.state.select(Some(i));
    }
}
