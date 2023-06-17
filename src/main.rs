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

use crate::app::StatefulList;
use crate::db::models::Pod;
use db::models::Episode;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::error::Error;
use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tui::layout::Alignment;
use tui::widgets::Wrap;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn run_migrations(
    connection: &mut impl MigrationHarness<diesel::sqlite::Sqlite>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // This will run the necessary migrations.
    //
    // See the documentation for `MigrationHarness` for
    // all available methods.
    connection.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut connection = establish_connection();
    run_migrations(&mut connection).unwrap();

    let pods = get_pods(&mut connection);

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
                        code: KeyCode::Char('?'),
                    }) => app.input_mode = InputMode::Help,
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char('n'),
                    }) => app.input_mode = InputMode::Editing,
                    _ => {}
                },
                InputMode::Help => match event {
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::NONE,
                        code: KeyCode::Char(q),
                    }) => app.input_mode = InputMode::Normal,
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

fn render_pods<B: Backend>(f: &mut Frame<B>, pods: &StatefulList<Pod>, main_chunks: &[Rect]) {
    let items: Vec<ListItem> = pods
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

    f.render_stateful_widget(pods_items, main_chunks[0], &mut pods.state.clone());
}

fn render_episodes<B: Backend>(
    f: &mut Frame<B>,
    episodes: &StatefulList<Episode>,
    main_chunks: &[Rect],
) {
    let mut episodes_items = Vec::<ListItem>::new();
    for ep in episodes.items.iter() {
        let mut icon = String::from(" ");
        if let Some(duration) = ep.duration {
            let progress_percent = ep.timestamp / duration as f32;
            icon = match progress_percent {
                x if (0.0..0.35).contains(&x) => String::from("◔"),
                x if (0.35..0.65).contains(&x) => String::from("◑"),
                x if (0.65..0.90).contains(&x) => String::from("◕"),
                _ => String::from("●"),
            }
        }
        let text = vec![Spans::from(format!("{} {}", icon, &ep.title))];
        episodes_items.push(ListItem::new(text).style(match &ep.downloaded {
            false => Style::default().fg(Color::White),
            true => Style::default().fg(Color::Green),
        }));
    }

    let active_border = Style::default().fg(Color::White);

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

    f.render_stateful_widget(episodes_list, main_chunks[0], &mut episodes.state.clone());
}

fn render_player<B: Backend>(f: &mut Frame<B>, app: &mut App, main_chunks: &[Rect]) {
    let progress = app.player.get_progress();
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
    if app.is_refreshing {
        player_spans.push(Spans::from(Span::from("Checking and fetching new episodes...")));
    }
    let player = Paragraph::new(player_spans)
        .block(Block::default().title(player_title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(player, main_chunks[1]);
}

fn render_input<B: Backend>(f: &mut Frame<B>, app: &App, size: Rect) {
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
    f.render_widget(Clear, area2);
    f.render_widget(
        Block::default().title("New pod").borders(Borders::ALL),
        area2,
    );
    f.render_widget(input1, input_chunks[0]);
    f.render_widget(input2, input_chunks[1]);
}

fn render_help<B: Backend>(f: &mut Frame<B>, app: &App, size: Rect) {
    let area2 = centered_rect(50, 50, size);
    let text = vec![
        Spans::from(Span::from("J/K to navigate up and down")),
        Spans::from(Span::from("Q to navigate back or quit")),
        Spans::from(Span::from("N to create a new pod")),
        Spans::from(Span::from("Space to toggle play/pause")),
        Spans::from(Span::from("R to refresh a podcasts feed/episodes")),
        Spans::from(Span::from("O to seek 100s ahead")),
        Spans::from(Span::from("I to seek 10s back")),
    ];
    let para = Paragraph::new(text)
        .block(Block::default().title("Help").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(Clear, area2);
    f.render_widget(para, area2);
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(f.size());

    match &app.navigation_stack {
        NavigationStack::Main => {
            render_pods(f, &app.pods, &main_chunks);
        }
        NavigationStack::Episodes => {
            if let Some(episodes) = &app.episodes {
                render_episodes(f, episodes, &main_chunks);
            }
        }
    }

    render_player(f, app, &main_chunks);

    if let InputMode::Editing = app.input_mode {
        render_input(f, app, size);
    }
    if let InputMode::Help = app.input_mode {
        render_help(f, app, size);
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
