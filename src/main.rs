use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::VecDeque;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;

use crossterm::event::{self, Event, KeyCode};
use hound::WavReader;
use walkdir::WalkDir;

struct AudioPlayer {
    // Player settings:
    playlist: Vec<String>,
    track_index: usize,
    is_playing: bool,
    channels: usize,
    sample_rate: u32,

    // cpal refs:
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<Stream>,

    // Samples:
    samples: Arc<Mutex<VecDeque<f32>>>, // `VecDeque` allows for fast and constant-time appending at the back and removing from the front
    sample_index: Arc<Mutex<usize>>, // Track the current sample index for pause/resume
}

impl AudioPlayer {
    fn new(playlist: Vec<String>, device: cpal::Device, config: cpal::StreamConfig) -> Self {
        Self {
            playlist,
            track_index: 0,
            is_playing: false,
            channels: config.channels as usize,
            sample_rate: config.sample_rate.0,

            device,
            config: config.clone(),
            stream: None,

            samples: Arc::new(Mutex::new(VecDeque::new())),
            sample_index: Arc::new(Mutex::new(0)), // Start at the beginning
        }
    }

    fn process_samples(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading file: {}", file_path);

        let mut reader = WavReader::open(file_path)?;
        let spec = reader.spec();

        if spec.sample_rate != self.sample_rate {
            eprintln!(
                "Sample rate mismatch. File has {}, expected {}.",
                spec.sample_rate, self.sample_rate
            );
            return Err("Sample rate mismatch".into());
        }

        let mut samples = self.samples.lock().unwrap();
        samples.clear(); 

        for sample in reader.samples::<i16>() {
            let normalized_sample = sample.unwrap() as f32 / i16::MAX as f32;
            samples.push_back(normalized_sample);
        }

        println!("Loaded {} samples.", samples.len());

        Ok(())
    }

    fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.playlist[self.track_index].clone();

        if self.samples.lock().unwrap().is_empty() {
            self.process_samples(&file_path)?;
        }

        let samples = Arc::clone(&self.samples);
        let config = self.config.clone();
        let channels = self.channels;
        let sample_index = Arc::clone(&self.sample_index);

        let stream = self.device.build_output_stream(
            &config,
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut samples = samples.lock().unwrap();
                let mut sample_index = sample_index.lock().unwrap();

                for frame in output.chunks_mut(channels) {
                    for sample in frame {
                        *sample = samples.pop_front().unwrap_or(0.0);
                        *sample_index += 1;
                    }
                }

                *sample_index = *sample_index;
            },
            |err| eprintln!("Error occurred on stream: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        self.is_playing = true;

        println!("Playing file: {}", file_path);
        Ok(())
    }

    fn toggle(&mut self) {
        if self.is_playing {
            println!("Paused at sample index: {}", self.sample_index.lock().unwrap());

            self.stream = None;
            self.is_playing = false;
        } else {
            
            println!("Resumed from sample index: {}", self.sample_index.lock().unwrap());
            if let Err(err) = self.play() {
                eprintln!("Failed to resume playback: {}", err);
            }
        }
    }

    fn next(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.pause().unwrap();
        }

        self.track_index = (self.track_index + 1) % self.playlist.len();

        self.force_play();
    }

    fn previous(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.pause().unwrap();
        }

        if self.track_index == 0 {
            self.track_index = self.playlist.len() - 1;
        } else {
            self.track_index -= 1;
        }
        
        self.force_play();
    }

    fn force_play(&mut self) {
        self.reset_samples();

        if let Err(err) = self.play() {
            eprintln!("Failed to play file: {}", err);
        }
    }

    fn reset_samples(&mut self) {
        *self.sample_index.lock().unwrap() = 0;
        self.samples.lock().unwrap().clear();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <folder_path>", args[0]);
        return Ok(());
    }

    let folder_path = &args[1];
    let mut playlist: Vec<String> = Vec::new();

    for entry in WalkDir::new(folder_path).into_iter().filter_map(Result::ok) {
        if let Some(ext) = entry.path().extension() {
            if ext == "wav" {
                playlist.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    if playlist.is_empty() {
        eprintln!("No audio files found");
        return Ok(());
    }

    println!("Found {} audio files.", playlist.len());

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config()?.config();

    let mut player = AudioPlayer::new(playlist, device, config);

    player.play()?;

    loop {
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('p') => player.toggle(),
                    KeyCode::Char('j') => player.previous(),
                    KeyCode::Char('k') => player.next(),
                    _ => {}
                }
            }
        }
    }

}