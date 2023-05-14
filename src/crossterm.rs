use std::io;
use std::time::{Duration, Instant};

use crate::app::{self, AppEvent};
use crate::app::{AppStyle, Page};
use crate::{ui, GlobalEvent};

use crossterm::event;
use crossterm::event::Event;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tokio::sync::mpsc::Sender;
use tui::backend::Backend;
use tui::{backend::CrosstermBackend, Terminal};

pub async fn run(tx: &Sender<GlobalEvent>) -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = app::App::new();
    let res = run_app(&mut terminal, app, tx).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

pub async fn run_app<'a, B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: app::App<'a>,
    tx_global: &Sender<GlobalEvent>,
) -> io::Result<()> {
    let style = AppStyle::new();
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::ui(f, &mut app, &style))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        match &mut app.page {
            Page::ChatRoom(page) => {
                if event::poll(timeout)? {
                    if let Event::Key(k) = event::read()? {
                        page.keybindings(k.code, &app.tx, tx_global).await;
                    }
                }
            }
            Page::TopicList(page) => {
                if event::poll(timeout)? {
                    if let Event::Key(k) = event::read()? {
                        page.keybindings(k.code, &app.tx, tx_global).await;
                    }
                }
            }
        };

        if let Ok(e) = app.rx.try_recv() {
            match e {
                AppEvent::Quit => app.should_close = true,
                AppEvent::ChangePage(page) => app.change_page(page),
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_close {
            tx_global.send(GlobalEvent::Quit).await.unwrap();
            return Ok(());
        }
    }
}
