extern crate rss;
extern crate tui;

use crate::network::IoEvent;
use serde::{Deserialize, Serialize};
use tui::widgets::ListState;
use crate::player::Player;

use std::sync::mpsc::Sender;


#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub feeds: Vec<ConfigFeed>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfigFeed {
    pub name: String,
    pub url: String,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub enum NavigationStack {
    Main,
    Episodes,
}

pub struct App {
    pub pods: StatefulList<ConfigFeed>,
    pub episodes: Option<StatefulList<rss::Item>>,
    io_tx: Option<Sender<IoEvent>>,
    pub is_loading: bool,
    pub is_downloading: bool,
    pub config: Config,
    pub navigation_stack: NavigationStack,
    pub player: Player,
}

impl App {
    pub fn new(config: Config, io_tx: Sender<IoEvent>, player: Player) -> App {
        App {
            config: config.clone(),
            pods: StatefulList::with_items(config.feeds.clone()),
            episodes: None,
            io_tx: Some(io_tx),
            is_loading: false,
            is_downloading: false,
            navigation_stack: NavigationStack::Main,
            player,
        }
    }

    pub fn dispatch(&mut self, action: IoEvent) {
        self.is_loading = true;
        if let Some(io_tx) = &self.io_tx {
            if let Err(e) = io_tx.send(action) {
                self.is_loading = false;
                println!("Error from dispatch {}", e);
            };
        }
    }

    pub fn get_channel(&mut self, url: String) {
        self.dispatch(IoEvent::GetChannel(url));
    }

    pub fn set_pod(&mut self, channel: rss::Channel) {
        self.episodes = Some(StatefulList::with_items(channel.items().to_vec()));
    }

    pub fn view_pod_under_cursor(&mut self) {
        self.navigation_stack = NavigationStack::Episodes;
        if let Some(index) = self.pods.state.selected() {
            self.get_channel(self.pods.items[index].url.clone());
        }
    }

    pub fn back(&mut self) {
        self.navigation_stack = NavigationStack::Main;
    }
    
    // TODO: Play or downlaod? Play if downloaded?
    // FIXME: Chain these if lets somehow with ? operator?
    pub fn download_episode_under_cursor(&mut self) {
        if let Some(data) = self.episodes.clone() {
            if let Some(index) = data.state.selected() {
                if let Some(enc) = &data.items[index].enclosure {
                    self.is_downloading = true;
                    self.dispatch(IoEvent::DownloadEpisode(enc.url.clone()));
                }
            }
        }
    }
}
