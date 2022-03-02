mod app;
mod network;

extern crate crossterm;
extern crate rss;
extern crate serde;
extern crate tui;

use app::{App, Config, NavigationStack};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use network::{IoEvent, Network};

use std::{
    fs, io,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{Block, Borders, List, ListItem},
    Frame, Terminal,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config_file =
        fs::read_to_string("./config.json").expect("Something went wrong reading config file");

    let config: Config;
    match serde_json::from_str(&config_file) {
        Ok(conf) => config = conf,
        Err(e) => {
            println!("{}", e);
            thread::sleep(Duration::from_secs(5));
            // restore terminal
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;
            return Ok(());
        }
    }

    let tick_rate = Duration::from_millis(250);
    let (sync_io_tx, sync_io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let app = Arc::new(Mutex::new(App::new(config, sync_io_tx)));
    let cloned_app = Arc::clone(&app);
    std::thread::spawn(move || {
        let mut network = Network::new(&app);
        start_tokio(sync_io_rx, &mut network);
    });
    let _res = run_app(&mut terminal, &cloned_app, tick_rate).await?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[tokio::main]
async fn start_tokio<'a>(io_rx: std::sync::mpsc::Receiver<IoEvent>, network: &mut Network) {
    while let Ok(io_event) = io_rx.recv() {
        network.handle_network_event(io_event).await;
    }
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &Arc<Mutex<App>>,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        let mut app = app.lock().await;
        terminal.draw(|mut f| ui(&mut f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            let event = event::read()?;
            match event {
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('q'),
                }) => match app.navigation_stack {
                    NavigationStack::Main => return Ok(()),
                    NavigationStack::Episodes => app.back(),
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('j'),
                }) => match app.navigation_stack {
                    NavigationStack::Main => app.pods.next(),
                    NavigationStack::Episodes => app.episodes.as_mut().unwrap().next(),
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Char('k'),
                }) => match app.navigation_stack {
                    NavigationStack::Main => app.pods.previous(),
                    NavigationStack::Episodes => app.episodes.as_mut().unwrap().previous(),
                },
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Enter,
                }) => match app.navigation_stack {
                    NavigationStack::Main => app.view_pod_under_cursor(),
                    NavigationStack::Episodes => app.download_episode_under_cursor(),
                },
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
        .split(f.size());

    let items: Vec<ListItem> = app
        .pods
        .items
        .iter()
        .map(|i| {
            let lines = vec![Spans::from(i.name.clone())];
            ListItem::new(lines).style(Style::default().fg(Color::White))
        })
        .collect();

    let active_border = Style::default().fg(Color::White);

    let pods_items = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(active_border)
                .title("Pods"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut episodes_items = Vec::<ListItem>::new();
    if let Some(data) = &app.episodes {
        for news in data.items.iter() {
            let text = vec![Spans::from(String::from(news.title().unwrap()))];
            episodes_items.push(ListItem::new(text).style(Style::default().fg(Color::White)));
        }
    };

    let episodes_list = List::new(episodes_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(active_border)
                .title("Episodes"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    match &app.navigation_stack {
        NavigationStack::Main => {
            f.render_stateful_widget(pods_items, main_chunks[0], &mut app.pods.state.clone());
        }
        NavigationStack::Episodes => {
            if let Some(episodes) = &app.episodes {
                f.render_stateful_widget(episodes_list, main_chunks[0], &mut episodes.state.clone());
            }
        }
    }
}
