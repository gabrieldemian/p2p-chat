use crate::app::{self, AppStyle, Page};
use tui::{backend::Backend, Frame};

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &mut app::App, ui: &AppStyle) {
    match &mut app.page {
        Page::TopicList(page) => {
            page.draw(f, ui);
        }
        Page::ChatRoom(page) => {
            page.draw(f, ui);
        }
    }
}
