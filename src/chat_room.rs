use crossterm::event::KeyCode;
use libp2p::gossipsub::IdentTopic;
use log::info;
use tokio::sync::mpsc::Sender;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::Modifier,
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    app::{AppMessage, AppStyle, Page},
    topic_list::TopicList,
    NetworkMessage,
};

#[derive(Clone, Debug)]
pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Clone, Debug)]
pub struct ChatRoom {
    pub state: ListState,
    pub items: Vec<String>,
    pub input_mode: InputMode,
    pub input: String,
    pub name: String,
}

impl Default for ChatRoom {
    fn default() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));

        let items = vec!["Esta mensagem ja estava aqui antes".to_string()];

        Self {
            name: "0".to_string(),
            state,
            items,
            input: String::new(),
            input_mode: InputMode::Normal,
        }
    }
}

impl ChatRoom {
    pub fn new(name: String) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));

        let items = vec![];

        Self {
            name,
            state,
            items,
            input: String::new(),
            input_mode: InputMode::Normal,
        }
    }

    pub async fn keybindings<'a>(
        &mut self,
        k: KeyCode,
        tx: &Sender<AppMessage<'a>>,
        tx_network: &Sender<NetworkMessage>,
    ) {
        match &self.input_mode {
            InputMode::Normal => match k {
                KeyCode::Char('i') => self.input_mode = InputMode::Insert,
                KeyCode::Char('q') | KeyCode::Esc => {
                    tx.send(AppMessage::ChangePage {
                        page: Page::TopicList(TopicList::new()),
                    })
                    .await
                    .unwrap();
                }
                _ => {}
            },
            InputMode::Insert => match k {
                KeyCode::Enter => {
                    let topic = IdentTopic::new(self.name.clone());
                    if tx_network
                        .send(NetworkMessage::MessageReceived(topic, self.input.clone()))
                        .await
                        .is_ok()
                    {
                        info!("keycode:enter msg here");
                        self.items.push(self.input.drain(..).collect());
                    }
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                _ => {}
            },
        }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, ui: &AppStyle) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(f.size());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    Span::raw("Press "),
                    Span::styled("q", ui.normal_style.add_modifier(Modifier::BOLD)),
                    Span::raw(" to exit, "),
                    Span::styled("i", ui.normal_style.add_modifier(Modifier::BOLD)),
                    Span::raw(" to enter insert mode."),
                ],
                ui.base_style.add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Insert => (
                vec![
                    Span::raw("Press "),
                    Span::styled("Esc", ui.normal_style.add_modifier(Modifier::BOLD)),
                    Span::raw(" to enter normal mode, "),
                    Span::styled("Enter", ui.normal_style.add_modifier(Modifier::BOLD)),
                    Span::raw(" to send the message"),
                ],
                ui.base_style,
            ),
        };
        let mut text = Text::from(Spans::from(msg));
        text.patch_style(style);
        let help_message = Paragraph::new(text);

        // render help msg
        f.render_widget(help_message, chunks[0]);

        let input = Paragraph::new(self.input.as_ref())
            .style(ui.base_style)
            .block(Block::default().borders(Borders::ALL).title("Message"));

        // render the user input
        f.render_widget(input, chunks[2]);

        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Insert => {
                // Make the cursor visible and ask tui-rs to put it
                // at the specified coordinates after rendering
                f.set_cursor(
                    // Put cursor past the end of the input text
                    chunks[2].x + self.input.len() as u16 + 1,
                    // Move one line down, from the border to the input line
                    chunks[2].y + 1,
                )
            }
        }

        let messages: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(_i, m)| {
                // todo: get the ID of the user and pre-pend to the msg
                // let content = vec![Spans::from(Span::raw(m))];
                let content = Text::from(m.as_str());
                ListItem::new(content)
            })
            .collect();

        let messages = List::new(messages)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .style(ui.base_style);

        // render message list
        f.render_widget(messages, chunks[1]);
    }
}
