// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![forbid(unsafe_code)]
// Justification: Fields on DecoderOptions and FormatOptions may change at any time, but
// symphonia-play doesn't want to be updated every time those fields change, therefore always fill
// in the remaining fields with default values.

use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::app::App;
use std::{sync::Arc, sync::Mutex};

mod output;

enum PlayStatus {
    Playing,
    Paused,
}

pub enum AudioEvent {
    AbortLoop,
    StartLoop,
}

pub struct AudioPlayer<'a> {
    app: &'a Arc<Mutex<App>>,
    status: PlayStatus,
    io_audio_tx: Option<std::sync::mpsc::Sender<AudioEvent>>,
}

impl<'a> AudioPlayer<'a> {
    pub fn new(app: &'a Arc<Mutex<App>>) -> AudioPlayer {
        AudioPlayer {
            app,
            status: PlayStatus::Paused,
            io_audio_tx: None,
        }
    }

    pub async fn play_loop(&mut self) {
        loop {
            while let Ok(audio_event) = io_audio_rx.recv() {
                match audio_event {
                    AudioEvent::AbortLoop => {
                        println!("Terminating.");
                        break;
                    }
                    AudioEvent::StartLoop => {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                    _ => {}
                }
            }
        }
    }

    // pub async fn handle_audio_event(&mut self, audio_event: AudioEvent) {
    //     match audio_event {
    //         AudioEvent::StartLoop => {
    //             self.status = PlayStatus::Playing;
    //             self.loopiloop().await;
    //         }
    //         AudioEvent::AbortLoop => {
    //             self.status = PlayStatus::Paused;
    //         }
    //     }
    // }

    // fn loopiloop(&mut self) {
    //     let (io_audio_tx, io_audio_rx) = std::sync::mpsc::channel::<AudioEvent>();
    //     self.io_audio_tx = Some(io_audio_tx);
    //     let some_state = 123;
    //     std::thread::spawn(move || loop {
    //         while let Ok(audio_event) = io_audio_rx.recv() {
    //             match audio_event {
    //                 AudioEvent::AbortLoop => {
    //                     println!("Terminating.");
    //                     break;
    //                 },
    //                 AudioEvent::StartLoop => {
    //                     std::thread::sleep(std::time::Duration::from_millis(500));
    //                 },
    //                 _ => {}
    //             }
    //         }
    //     });
    // }

    pub fn play_track(&mut self, path: String) {
        // Open the media source.
        let src = std::fs::File::open(&path).expect("failed to open media");

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let mut hint = Hint::new();
        hint.with_extension("mp3");

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");

        // Get the instantiated format reader.
        let mut format = probed.format;

        // Find the first audio track with a known (decodeable) codec.
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("no supported audio tracks");

        // Use the default options for the decoder.
        let dec_opts: DecoderOptions = Default::default();

        // Create a decoder for the track.
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .expect("unsupported codec");

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;

        let mut audio_output = None;

        // The decode loop.
        loop {
            // Get the next packet from the media format.
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    // The track list has been changed. Re-examine it and create a new set of decoders,
                    // then restart the decode loop. This is an advanced feature and it is not
                    // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                    // for chained OGG physical streams.
                    unimplemented!();
                }
                Err(err) => {
                    // A unrecoverable error occured, halt decoding.
                    panic!("{}", err);
                }
            };

            // Consume any new metadata that has been read since the last packet.
            while !format.metadata().is_latest() {
                // Pop the old head of the metadata queue.
                format.metadata().pop();

                // Consume the new metadata at the head of the metadata queue.
            }

            // If the packet does not belong to the selected track, skip over it.
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet into audio samples.
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // If the audio output is not open, try to open it.
                    if audio_output.is_none() {
                        // Get the audio buffer specification. This is a description of the decoded
                        // audio buffer's sample format and sample rate.
                        let spec = *decoded.spec();

                        // Get the capacity of the decoded buffer. Note that this is capacity, not
                        // length! The capacity of the decoded buffer is constant for the life of the
                        // decoder, but the length is not.
                        let duration = decoded.capacity() as u64;

                        // Try to open the audio output.
                        audio_output.replace(output::try_open(spec, duration).unwrap());
                    } else {
                        // TODO: Check the audio spec. and duration hasn't changed.
                    }

                    if let Some(audio_output) = &mut audio_output {
                        audio_output.write(decoded).unwrap()
                    }
                }
                Err(Error::IoError(_)) => {
                    // The packet failed to decode due to an IO error, skip the packet.
                    continue;
                }
                Err(Error::DecodeError(_)) => {
                    // The packet failed to decode due to invalid data, skip the packet.
                    continue;
                }
                Err(err) => {
                    // An unrecoverable error occured, halt decoding.
                    panic!("{}", err);
                }
            }
        }
    }

    pub fn toggle_playback(&mut self) {
        match self.status {
            PlayStatus::Playing => self.status = PlayStatus::Paused,
            PlayStatus::Paused => self.status = PlayStatus::Playing,
        }
    }
}
