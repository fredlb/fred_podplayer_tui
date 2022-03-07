use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::{
        streaming::{StreamingSoundData, StreamingSoundSettings},
        SoundData, static_sound::PlaybackState,
    },
};

pub struct Track {
    pub filepath: String,
}

pub struct Player {
    manager: AudioManager,
    pub selected_track: Option<Track>,
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
        if let Some(track) = &self.selected_track {
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
            handler.seek_by(5.0).unwrap();
        }
    }

    pub fn jump_backward_10s(&mut self) {
        if let Some(handler) = &mut self.handler {
            handler.seek_by(-5.0).unwrap();
        }
    }

    pub fn get_progress(&mut self) -> f64 {
        if let Some(handler) = &mut self.handler {
            return handler.position();
        }
        return 0.0;
    }
}
