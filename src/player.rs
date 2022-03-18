use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::{
        static_sound::PlaybackState,
        streaming::{StreamingSoundData, StreamingSoundSettings},
        SoundData,
    }, Volume, tween::Tween,
};

use symphonia::core::units::Time;

use crate::db::models::Episode;

pub struct Player {
    manager: AudioManager,
    pub selected_track: Option<Episode>,
    handler: Option<<StreamingSoundData as SoundData>::Handle>,
}

impl Player {
    pub fn new() -> Player {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default()).unwrap();
        Player {
            manager,
            selected_track: None,
            handler: None,
        }
    }

    pub fn play(&mut self) {
        if let Some(track) = &mut self.selected_track {
            if let Some(handler) = &mut self.handler {
                let _ = handler.stop(kira::tween::Tween::default());
            }

            let sound =
                StreamingSoundData::from_file(track.audio_filepath.as_ref().unwrap(), StreamingSoundSettings::default())
                    .unwrap();
            self.handler = Some(self.manager.play(sound).unwrap());
            if let Some(handler) = &mut self.handler {
                handler.set_volume(Volume::Amplitude(0.5), Tween::default());
            }
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
            handler.seek_by(100.0).unwrap();
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

    pub fn increase_volume(&mut self) {
        if let Some(handler) = &mut self.handler {
            handler.set_volume(Volume::Amplitude(1.0), Tween::default());
        }
    }

    pub fn decrease_volume(&mut self) {
        if let Some(handler) = &mut self.handler {
            handler.set_volume(Volume::Amplitude(0.5), Tween::default());
        }
    }

    pub fn get_progress(&mut self) -> String {
        if let Some(handler) = &mut self.handler {
            let pos = Time::from(handler.position());
            let cur_dur = self.fmt_time(pos.seconds);
            let dur = self.selected_track.as_ref().unwrap().duration.unwrap();
            let tot_dur = self.fmt_time(dur as u64);
            return format!("{} / {}", cur_dur, tot_dur);
        }
        return String::from("");
    }

    fn fmt_time(&mut self, secs: u64) -> String {
        let hours = secs / (60 * 60);
        let mins = (secs % (60 * 60)) / 60;
        let secs = (secs % 60) as u32;

        return format!("{}:{:0>2}:{:0>2}", hours, mins, secs);
    }
}
