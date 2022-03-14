extern crate rss;
extern crate tui;

use crate::db::models::{Episode, Pod};
use crate::db::{establish_connection, get_episodes_for_pod, set_timestamp_on_episode};
use crate::{
    network::IoEvent,
    player::{Player, TrackFile},
};
use kira::sound::static_sound::PlaybackState;
use tui::widgets::ListState;

use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
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
    pub pods: StatefulList<Pod>,
    pub episodes: Option<StatefulList<Episode>>,
    io_tx: Option<Sender<IoEvent>>,
    pub is_loading: bool,
    pub is_downloading: bool,
    pub navigation_stack: NavigationStack,
    pub player: Player,
    pub active_pod_id: i32,
}

impl App {
    pub fn new(io_tx: Sender<IoEvent>, player: Player, pods_db: Vec<Pod>) -> App {
        App {
            pods: StatefulList::with_items(pods_db.clone()),
            episodes: None,
            io_tx: Some(io_tx),
            is_loading: false,
            is_downloading: false,
            navigation_stack: NavigationStack::Main,
            player,
            active_pod_id: 0,
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

    pub fn set_active_pod(&mut self, id: i32) {
        self.active_pod_id = id;
        let conn = establish_connection();
        let eps = get_episodes_for_pod(&conn, id);
        self.episodes = Some(StatefulList::with_items(eps));
    }

    pub fn back(&mut self) {
        self.navigation_stack = NavigationStack::Main;
    }

    pub fn toggle_playback(&mut self) {
        match self.player.get_playback_state() {
            PlaybackState::Playing => {
                self.save_timestamp();
                self.player.toggle_playback();
            },
            PlaybackState::Paused | PlaybackState::Pausing => {
                self.player.toggle_playback();
            },
            _ => {},
        }
    }

    pub fn save_timestamp(&mut self) {
        if let Some(selected_track) = &self.player.selected_track {
            let conn = establish_connection();
            set_timestamp_on_episode(&conn, selected_track.id.clone(), self.player.get_current_timestamp());
        }
    }

    pub fn handle_enter_pod(&mut self) {
        self.navigation_stack = NavigationStack::Episodes;
        if let Some(index) = self.pods.state.selected() {
            let pod = &self.pods.items[index];
            if !pod.downloaded {
                return self.dispatch(IoEvent::GetPodEpisodes(self.pods.items[index].clone()));
            }
            let conn = establish_connection();
            self.episodes = Some(StatefulList::with_items(get_episodes_for_pod(
                &conn, pod.id,
            )));
        }
    }

    pub fn handle_enter_episode(&mut self) {
        if let Some(data) = self.episodes.clone() {
            if let Some(index) = data.state.selected() {
                let ep = &data.items[index];
                if !ep.downloaded {
                    self.is_downloading = true;
                    return self.dispatch(IoEvent::DownloadEpisodeAudio(data.items[index].clone()));
                }
                self.player.selected_track = Some(ep.clone());
                self.player.play();
                self.player.seek(ep.timestamp);
            }
        }
    }
}
