use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::{
        static_sound::PlaybackState,
        streaming::{StreamingSoundData, StreamingSoundSettings},
        SoundData,
    },
};

use std::fs::File;
use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Time, TimeBase};

use crate::db::models::Episode;

pub struct Player {
    manager: AudioManager,
    pub selected_track: Option<Episode>,
    handler: Option<<StreamingSoundData as SoundData>::Handle>,
    pub duration_str: String,
}

impl Player {
    pub fn new() -> Player {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default()).unwrap();
        Player {
            manager,
            selected_track: None,
            handler: None,
            duration_str: String::from(""),
        }
    }

    pub fn play(&mut self) {
        if let Some(track) = &mut self.selected_track {
            if let Some(handler) = &mut self.handler {
                let _ = handler.stop(kira::tween::Tween::default());
            }
            // Create a hint to help the format registry guess what format reader is appropriate.
            let mut hint = Hint::new();

            // If the path string is '-' then read from standard input.
            let source = {
                // Othwerise, get a Path from the path string.
                let path = Path::new(track.audio_filepath.as_ref().unwrap());

                // Provide the file extension as a hint.
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
                            self.duration_str = fmt_time(n_frames, tb);
                        }
                    }
                }
                Err(_) => panic!("could not probe audio for metadata"),
            }
            let sound =
                StreamingSoundData::from_file(track.audio_filepath.as_ref().unwrap(), StreamingSoundSettings::default())
                    .unwrap();
            self.handler = Some(self.manager.play(sound).unwrap());
        }
    }

    pub fn toggle_playback(&mut self) {
        if let Some(handler) = &mut self.handler {
            match handler.state() {
                PlaybackState::Playing => handler.pause(kira::tween::Tween::default()).unwrap(),
                PlaybackState::Paused | PlaybackState::Pausing => {
                    handler.resume(kira::tween::Tween::default()).unwrap()
                }
                _ => {}
            };
        }
    }

    pub fn get_playback_state(&mut self) -> PlaybackState {
        if let Some(handler) = &mut self.handler {
            return handler.state();
        } else {
            return PlaybackState::Stopped;
        }
    }

    pub fn get_current_timestamp(&mut self) -> f32 {
        match &self.handler {
            Some(handler) => return handler.position().clone() as f32,
            None => return 0.0,
        }
    }

    pub fn jump_forward_10s(&mut self) {
        if let Some(handler) = &mut self.handler {
            handler.seek_by(10.0).unwrap();
        }
    }

    pub fn jump_backward_10s(&mut self) {
        if let Some(handler) = &mut self.handler {
            handler.seek_by(-10.0).unwrap();
        }
    }

    pub fn seek(&mut self, ts: f32) {
        if let Some(handler) = &mut self.handler {
            handler.seek_to(ts as f64).unwrap();
        }
    }

    pub fn get_progress(&mut self) -> String {
        if let Some(handler) = &mut self.handler {
            let pos = Time::from(handler.position());
            let hours = pos.seconds / (60 * 60);
            let mins = (pos.seconds % (60 * 60)) / 60;
            let secs = (pos.seconds % 60) as u32;

            return format!("{}:{:0>2}:{:0>2}", hours, mins, secs);
        }
        return String::from("");
    }
}

fn fmt_time(ts: u64, tb: TimeBase) -> String {
    let time = tb.calc_time(ts);

    let hours = time.seconds / (60 * 60);
    let mins = (time.seconds % (60 * 60)) / 60;
    let secs = (time.seconds % 60) as u32;

    format!("{}:{:0>2}:{:0>2}", hours, mins, secs)
}
