extern crate rss;
use crate::app::App;
use crate::player::TrackFile;

use std::sync::Arc;
use std::io::Write;
use std::fs::File;
use tokio::sync::Mutex;
use error_chain::error_chain;

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}

pub enum IoEvent {
    GetChannel(String),
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
            IoEvent::GetChannel(url) => {
                self.get_channel(url).await;
            }
            IoEvent::DownloadEpisode(url) => {
                let _ = self.download_episode(url).await;
            }
        }
        let mut app = self.app.lock().await;
        app.is_loading = false;
        app.is_downloading = false;
    }

    async fn get_channel(&mut self, url: String) {
        let result = reqwest::get(url).await;
        match result {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let mut app = self.app.lock().await;
                    app.set_pod(channel.unwrap());
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
        dest.write_all(&content)?;
        let mut app = self.app.lock().await;
        app.player.selected_track = Some(TrackFile { filepath: filename });
        app.player.play();
        Ok(())
    }
}
