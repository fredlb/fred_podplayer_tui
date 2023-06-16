extern crate rss;
use crate::app::App;
use crate::db::models::{Episode, Pod};
use crate::db::{
    create_episode, establish_connection, get_episodes_for_pod, mark_episode_as_downloaded,
    mark_pod_as_downloaded,
};

use chrono::DateTime;
use reqwest::header::USER_AGENT;

use error_chain::error_chain;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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
                self.download_pod_updates(pod).await;
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
        let client = reqwest::Client::new();
        let res = client
            .get(&pod.url)
            .header(USER_AGENT, "Mah podplayah")
            .send()
            .await;
        match res {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let mut conn = establish_connection();
                    match channel {
                        Ok(chan) => {
                            for item in chan.items().iter() {
                                let dt = item.pub_date().unwrap();
                                let dt2 = DateTime::parse_from_rfc2822(dt).unwrap();
                                create_episode(
                                    &mut conn,
                                    item.guid().unwrap().value(),
                                    pod.id,
                                    item.title().unwrap(),
                                    item.link().unwrap_or(""),
                                    item.enclosure().unwrap().url(),
                                    "",
                                    dt2.timestamp() as i32,
                                    false,
                                );
                            }
                            mark_pod_as_downloaded(&mut conn, pod.id);
                        }
                        Err(err) => panic!("failed to download episodes: {}", err),
                    }
                    let mut app = self.app.lock().await;
                    app.set_active_pod(pod.id);
                }
                Err(_e) => {}
            },
            Err(_e) => {}
        }
    }

    async fn download_pod_updates(&mut self, pod: Pod) {
        let mut conn = establish_connection();
        let existing_episodes = get_episodes_for_pod(&mut conn, pod.id);
        let uids: Vec<String> = existing_episodes.iter().map(|ep| ep.uid.clone()).collect();

        let client = reqwest::Client::new();
        let res = client
            .get(&pod.url)
            .header(USER_AGENT, "Mah podplayah")
            .send()
            .await;
        match res {
            Ok(result) => match result.bytes().await {
                Ok(result) => {
                    let channel = rss::Channel::read_from(&result[..]);
                    let mut conn = establish_connection();
                    match channel {
                        Ok(chan) => {
                            for item in chan.items().iter() {
                                if !uids.contains(&item.guid().unwrap().value().to_string()) {
                                    let dt = item.pub_date().unwrap();
                                    let dt2 = DateTime::parse_from_rfc2822(dt).unwrap();
                                    create_episode(
                                        &mut conn,
                                        item.guid().unwrap().value(),
                                        pod.id,
                                        item.title().unwrap(),
                                        item.link().unwrap_or(""),
                                        item.enclosure().unwrap().url(),
                                        "",
                                        dt2.timestamp() as i32,
                                        false,
                                    );
                                }
                            }
                        }
                        Err(err) => panic!("failed to download episodes: {}", err),
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
        let duration = self.read_metadata_from_file(&filename);
        let mut conn = establish_connection();
        let updated_ep = mark_episode_as_downloaded(&mut conn, &episode, &filename, duration as i32);
        let mut app = self.app.lock().await;
        app.play_episode(updated_ep);
        Ok(())
    }

    fn read_metadata_from_file(&mut self, filepath: &String) -> u64 {
        let mut hint = Hint::new();
        let source = {
            let path = Path::new(filepath);

            if let Some(extension) = path.extension() {
                if let Some(extension_str) = extension.to_str() {
                    hint.with_extension(extension_str);
                }
            }

            Box::new(File::open(path).unwrap())
        };
        let mss = MediaSourceStream::new(source, Default::default());
        let metadata_opts: MetadataOptions = Default::default();
        let format_opts = FormatOptions {
            enable_gapless: false,
            ..Default::default()
        };
        match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
            Ok(probed) => {
                let params = &probed.format.default_track().unwrap().codec_params;

                if let Some(n_frames) = params.n_frames {
                    if let Some(tb) = params.time_base {
                        let time = tb.calc_time(n_frames);
                        return time.seconds;
                    } else {
                        return 0;
                    }
                } else {
                    return 0;
                }
            }
            Err(_) => panic!("could not probe audio for metadata"),
        }
    }
}
