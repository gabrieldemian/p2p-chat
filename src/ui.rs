use crate::app::{self, Page};
use tui::{backend::Backend, Frame};

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &mut app::App) {
    match &mut app.page {
        Page::TopicList(page) => {
            page.draw(f, &app.style);
        }
        Page::ChatRoom(page) => {
            page.draw(f, &app.style);
        }
    }
}
