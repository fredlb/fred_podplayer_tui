use rodio;

pub struct Track {
    pub filepath: String,
}

pub enum PlayState {
    Playing,
    Paused,
    Stopped,
    NotStarted,
}

pub struct Player {
    pub sink: rodio::Sink,
    pub stream_handle: rodio::OutputStreamHandle,
    pub selected_track: Option<Track>,
    pub play_state: PlayState,
}

impl Player {
    pub fn new(sink: rodio::Sink, stream_handle: rodio::OutputStreamHandle) -> Player {
        Player {
            sink,
            stream_handle,
            selected_track: None,
            play_state: PlayState::NotStarted,
        }
    }

    pub fn play(&mut self) {
        if let Some(track) = &self.selected_track {
            let file = std::io::BufReader::new(
                std::fs::File::open(&track.filepath).expect("Failed to open file"),
            );
            let source = rodio::Decoder::new(file).expect("Failed to decode audio file");

            match self.play_state {
                PlayState::NotStarted | PlayState::Stopped | PlayState::Playing => {
                    self.play_state = PlayState::Playing;

                    let sink_try = rodio::Sink::try_new(&self.stream_handle);

                    match sink_try {
                        Ok(sink) => {
                            self.sink = sink;
                            self.sink.append(source);
                        }
                        Err(_) => (),
                    }
                }
                PlayState::Paused => {
                    self.play_state = PlayState::Playing;
                    self.sink.play();
                }
            }
        }
    }

    pub fn pause(&mut self) {
        match self.play_state {
            PlayState::Playing => {
                self.play_state = PlayState::Paused;
                self.sink.pause();
            }
            PlayState::Paused => {
                self.play_state = PlayState::Playing;
                self.sink.play();
            }
            _ => (),
        }
    }

    pub fn stop(&mut self) {
        match &self.play_state {
            PlayState::Playing | PlayState::Paused => {
                self.play_state = PlayState::Stopped;
                self.sink.stop();
            }
            _ => (),
        }
    }
}
