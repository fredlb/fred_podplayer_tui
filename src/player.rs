use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundSettings},
        SoundData, static_sound::PlaybackState,
    },
};

use std::fs::File;
use std::io::Write;
use std::path::Path;

use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::{Error, Result};
use symphonia::core::formats::{Cue, FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::{MediaSource, MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::{ColorMode, MetadataOptions, MetadataRevision, Tag, Value, Visual};
use symphonia::core::probe::{Hint, ProbeResult};
use symphonia::core::units::{Time, TimeBase};

pub struct TrackFile {
    pub filepath: String,
    pub duration: String,
}

pub struct Player {
    manager: AudioManager,
    pub selected_track: Option<TrackFile>,
    handler: Option<<StreamingSoundData as SoundData>::Handle>,
}

impl Player {
    pub fn new() -> Player {
        let manager =
            AudioManager::<CpalBackend>::new(AudioManagerSettings::default()).unwrap();
        Player {
            manager,
            selected_track: None,
            handler: None,
        }
    }

    pub fn play(&mut self) {
        if let Some(track) = &mut self.selected_track {
            // Create a hint to help the format registry guess what format reader is appropriate.
            let mut hint = Hint::new();

            // If the path string is '-' then read from standard input.
            let source = {
                // Othwerise, get a Path from the path string.
                let path = Path::new(&track.filepath);

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
            let format_opts =
                FormatOptions { enable_gapless: false, ..Default::default() };
            match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
                Ok(mut probed) => {
                    let params = &probed.format.default_track().unwrap().codec_params;

                    if let Some(n_frames) = params.n_frames {
                        if let Some(tb) = params.time_base {
                            track.duration = fmt_time(n_frames, tb);
                        }
                    }

                }
                Err(err) => println!("lol"),
            }
            let sound =
                StreamingSoundData::from_file(&track.filepath, StreamingSoundSettings::default())
                    .unwrap();
            self.handler = Some(self.manager.play(sound).unwrap());
        }
    }

    pub fn toggle_playback(&mut self) {
        if let Some(handler) = &mut self.handler {
            match handler.state() {
                PlaybackState::Playing => {
                    handler.pause(kira::tween::Tween::default()).unwrap()
                }
                PlaybackState::Paused | PlaybackState::Pausing => {
                    handler.resume(kira::tween::Tween::default()).unwrap()
                }
                _ => {}
            };
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
