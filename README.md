# AudioPlayer

## Overview

`AudioPlayer` is a simple Rust-based application for playing `.wav` audio files from a specified folder. It supports playback controls such as play/pause, skip to the next track, and return to the previous track using keyboard inputs.

## Features

- **Play `.wav` files:** Automatically scans a folder and loads `.wav` files into a playlist.
- **Playback controls:**
  - `p`: Toggle play/pause.
  - `j`: Play the previous track.
  - `k`: Play the next track.
- **Sample playback:** Uses CPAL for audio output and supports `.wav` files with matching sample rates.
- **Error handling:** Ensures that the files match the expected sample rate and channels.

## Installation

1. Clone this repository or copy the source code into your Rust project.
2. Build the project:
   ```bash
    cargo build --release
   ```
3. Compile and run the application, passing the folder containing .wav files as an argument:
   ```bash
    cargo run --release -- <folder_path>
   ```
