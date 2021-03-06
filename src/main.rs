#[macro_use]
extern crate diesel;

extern crate crossterm;
extern crate rss;
extern crate serde;
extern crate tui;

mod app;
mod db;
mod network;
mod player;

use app::{App, InputField, InputMode, NavigationStack};
use db::{establish_connection, get_pods};
use player::Player;
use unicode_width::UnicodeWidthStr;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use network::{IoEvent, Network};

use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let connection = establish_connection();
    let pods = get_pods(&connection);

    let player = Player::new();

    let tick_rate = Duration::from_millis(250);
    let (sync_io_tx, sync_io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let app = Arc::new(Mutex::new(App::new(sync_io_tx, player, pods)));

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
        terminal.draw(|mut f| ui(&mut f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            let event = event::read()?;
            match app.input_mode {
                InputMode::Normal => match event {
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('q'),
                    }) => match app.navigation_stack {
                        NavigationStack::Main => {
                            app.save_timestamp();
                            return Ok(());
                        }
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
                        NavigationStack::Main => app.handle_enter_pod(),
                        NavigationStack::Episodes => app.handle_enter_episode(),
                    },
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char(' '),
                    }) => app.toggle_playback(),
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('r'),
                    }) => app.refresh_pod(),
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('o'),
                    }) => app.player.jump_forward_10s(),
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('i'),
                    }) => app.player.jump_backward_10s(),
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('n'),
                    }) => app.input_mode = InputMode::Editing,
                    _ => {}
                },
                InputMode::Editing => match event {
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char(c),
                    }) => match app.input_field {
                        InputField::Name => app.input_pod_name.push(c),
                        InputField::Url => app.input_pod_url.push(c),
                    },
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::SHIFT,
                        code: KeyCode::Char(c),
                    }) => match app.input_field {
                        InputField::Name => app.input_pod_name.push(c),
                        InputField::Url => app.input_pod_url.push(c),
                    },
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Esc,
                    }) => app.input_mode = InputMode::Normal,
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Backspace,
                    }) => match app.input_field {
                        InputField::Name => {
                            let _ = app.input_pod_name.pop();
                        }
                        InputField::Url => {
                            let _ = app.input_pod_url.pop();
                        }
                    },
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Tab,
                    }) => match app.input_field {
                        InputField::Name => app.input_field = InputField::Url,
                        InputField::Url => app.input_field = InputField::Name,
                    },
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Enter,
                    }) => app.create_pod(),
                    _ => {}
                },
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(f.size());

    let items: Vec<ListItem> = app
        .pods
        .items
        .iter()
        .map(|i| {
            let lines = vec![Spans::from(i.title.clone())];
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
        for ep in data.items.iter() {
            let mut icon = String::from(" ");
            if let Some(duration) = ep.duration {
                let progress_percent = ep.timestamp / duration as f32;
                icon = match progress_percent {
                    x if (0.0..0.35).contains(&x) => String::from("???"),
                    x if (0.35..0.65).contains(&x) => String::from("???"),
                    x if (0.65..0.90).contains(&x) => String::from("???"),
                    _ => String::from("???"),
                }
            }
            let text = vec![Spans::from(format!("{} {}", icon, &ep.title))];
            episodes_items.push(ListItem::new(text).style(match &ep.downloaded {
                false => Style::default().fg(Color::White),
                true => Style::default().fg(Color::Green),
            }));
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

    let progress = &app.player.get_progress();
    let mut player_spans: Vec<Spans> = Vec::new();
    let mut player_title = String::from("Player");
    match &app.player.selected_track {
        Some(track) => {
            player_spans.push(Spans::from(Span::from(progress.clone())));
            player_title = track.title.clone();
        }
        None => {}
    };
    if app.is_downloading {
        player_spans.push(Spans::from(Span::from("Episode is downloading...")));
    }
    let player = Paragraph::new(player_spans)
        .block(Block::default().title(player_title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(player, main_chunks[1]);

    match &app.navigation_stack {
        NavigationStack::Main => {
            f.render_stateful_widget(pods_items, main_chunks[0], &mut app.pods.state.clone());
        }
        NavigationStack::Episodes => {
            if let Some(episodes) = &app.episodes {
                f.render_stateful_widget(
                    episodes_list,
                    main_chunks[0],
                    &mut episodes.state.clone(),
                );
            }
        }
    }

    if let InputMode::Editing = app.input_mode {
        let block = Block::default().title("New pod").borders(Borders::ALL);
        let area = centered_rect(80, 25, size);
        let area2 = centered_rect(90, 35, size);
        let input_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);
        let name_input_width = input_chunks[0].width;
        let url_input_width = input_chunks[1].width;
        let mut name_scroll_offset = 0;
        let mut url_scroll_offset = 0;
        if app.input_pod_name.width() as u16 >= name_input_width - 2 {
            name_scroll_offset = app.input_pod_name.width() as u16 - (name_input_width - 2);
        }
        if app.input_pod_url.width() as u16 >= name_input_width - 2 {
            url_scroll_offset = app.input_pod_url.width() as u16 - (url_input_width - 2);
        }
        let input1 = Paragraph::new(app.input_pod_name.as_ref())
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("Name"))
            .scroll((0, name_scroll_offset));
        let input2 = Paragraph::new(app.input_pod_url.as_ref())
            .style(Style::default())
            .block(Block::default().borders(Borders::ALL).title("URL"))
            .scroll((0, url_scroll_offset));
        match app.input_field {
            InputField::Name => {
                let mut cursor_pos = app.input_pod_name.width() as u16 + 1;
                if cursor_pos >= name_input_width - 2 {
                    cursor_pos = name_input_width - 2;
                }
                f.set_cursor(input_chunks[0].x + cursor_pos, input_chunks[0].y + 1);
            }
            InputField::Url => {
                let mut cursor_pos = app.input_pod_url.width() as u16 + 1;
                if cursor_pos >= url_input_width - 2 {
                    cursor_pos = url_input_width - 2;
                }
                f.set_cursor(input_chunks[1].x + cursor_pos, input_chunks[1].y + 1);
            }
        }
        f.render_widget(Clear, area2); //this clears out the background
        f.render_widget(block, area2);
        f.render_widget(input1, input_chunks[0]);
        f.render_widget(input2, input_chunks[1]);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
