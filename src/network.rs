extern crate rss;
use crate::app::App;
use crate::player::TrackFile;
use crate::db::{establish_connection, mark_pod_as_downloaded};
use crate::db::models::{Pod};

use std::sync::Arc;
use std::io::Write;
use std::fs::{File, create_dir_all};
use tokio::sync::Mutex;
use error_chain::error_chain;

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}

pub enum IoEvent {
    GetChannel(Pod),
    DownloadEpisode(String),
}

pub struct Network<'a> {
    pub app: &'a Arc<Mutex<App>>,
}

impl<'a> Network<'a> {
    pub fn new(app: &'a Arc<Mutex<App>>) -> Network {
        Network { app }
    }

    pub async fn handle_network_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetChannel(pod) => {
                self.get_channel(pod).await;
            }
            IoEvent::DownloadEpisode(url) => {
                let _ = self.download_episode(url).await;
            }
        }
        let mut app = self.app.lock().await;
        app.is_loading = false;
        app.is_downloading = false;
    }

    async fn get_channel(&mut self, pod: Pod) {
        let result = reqwest::get(pod.url).await;
        match result {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let conn = establish_connection();
                    match channel {
                        Ok(chan) => {
                            //TODO: Create episodes
                            mark_pod_as_downloaded(&conn, pod.id);
                        },
                        Err(err) => {},
                    }
                    // let mut app = self.app.lock().await;
                    // app.set_pod(channel.unwrap());
                }
                Err(_e) => {}
            },
            Err(_e) => {}
        }
    }

    async fn download_episode(&mut self, url: String) -> Result<()> {
        let result = reqwest::get(url).await?;
        let filename;
        let mut dest = {
            let fname = result
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("tmp.bin");
            let fname = format!("./data/{}", fname);
            filename = String::from(fname.as_str());
            File::create(fname)?
        };
        let content = result.bytes().await?;
        create_dir_all("./data")?;
        dest.write_all(&content)?;
        let mut app = self.app.lock().await;
        app.player.selected_track = Some(TrackFile { filepath: filename, duration: String::from("") });
        app.player.play();
        Ok(())
    }
}
