extern crate rss;
use crate::app::App;
use crate::db::models::{Episode, Pod};
use crate::db::{create_episode, establish_connection, mark_pod_as_downloaded, mark_episode_as_downloaded};
use crate::player::TrackFile;

use error_chain::error_chain;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

error_chain! {
     foreign_links {
         Io(std::io::Error);
         HttpRequest(reqwest::Error);
     }
}

pub enum IoEvent {
    GetPodEpisodes(Pod),
    GetPodUpdates(Pod),
    DownloadEpisodeAudio(Episode),
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
            IoEvent::GetPodEpisodes(pod) => {
                self.download_pod_and_episodes(pod).await;
            }
            IoEvent::GetPodUpdates(pod) => {
            }
            IoEvent::DownloadEpisodeAudio(episode) => {
                let _ = self.download_episode_audio(episode).await;
            }
        }
        let mut app = self.app.lock().await;
        app.is_loading = false;
        app.is_downloading = false;
    }

    async fn download_pod_and_episodes(&mut self, pod: Pod) {
        let result = reqwest::get(pod.url).await;
        match result {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let conn = establish_connection();
                    match channel {
                        Ok(chan) => {
                            for item in chan.items().iter() {
                                create_episode(
                                    &conn,
                                    item.guid().unwrap().value(),
                                    pod.id,
                                    item.title().unwrap(),
                                    item.link().unwrap_or(""),
                                    item.enclosure().unwrap().url(),
                                    "",
                                    false,
                                );
                            }
                            mark_pod_as_downloaded(&conn, pod.id);
                        }
                        Err(err) => panic!("failed to download episodes")
                    }
                    let mut app = self.app.lock().await;
                    app.set_active_pod(pod.id);
                }
                Err(_e) => {}
            },
            Err(_e) => {}
        }
    }

    async fn download_episode_audio(&mut self, episode: Episode) -> Result<()> {
        let result = reqwest::get(&episode.audio_url).await?;
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
        let conn = establish_connection();
        let updated_ep = mark_episode_as_downloaded(&conn, &episode, &filename);
        let mut app = self.app.lock().await;
        app.player.selected_track = Some(updated_ep);
        app.player.play();
        Ok(())
    }
}
