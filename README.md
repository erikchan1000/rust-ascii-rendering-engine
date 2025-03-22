# ASCII Video Player

A Rust application that converts videos to ASCII art and plays them in your terminal with audio support.

![ASCII Video Player Demo](https://example.com/demo.gif)

## Features

- Convert and play videos as ASCII art in real-time
- Audio playback support (optional)
- Adjustable playback speed
- Customizable ASCII dimensions
- Brightness inversion option
- Playback controls (pause/play, skip frames, speed adjustment)
- Volume control

## Prerequisites

- Rust and Cargo (installation via [rustup](https://rustup.rs/))
- FFmpeg (for video frame and audio extraction)
- ALSA development libraries (Linux) or PulseAudio (WSL)

### System-specific requirements

#### Linux
```bash
sudo apt install ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libasound2-dev
```

#### macOS
```bash
brew install ffmpeg
```

#### Windows
- Install FFmpeg from the [official website](https://ffmpeg.org/download.html) and add it to your PATH
- For WSL, follow the PulseAudio setup instructions in the "Audio in WSL" section

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/ascii-video-player.git
cd ascii-video-player
```

2. Build the application:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release -- --input path/to/your/video.mp4 --audio
```

## Usage

### Command-line options

- `--input <file>`: Specify the input video file path
- `--audio`: Enable audio playback (optional)

Examples:
```bash
# Play video with audio
cargo run --release -- --input myvideo.mp4 --audio

# Play video without audio
cargo run --release -- --input myvideo.mp4

# Legacy mode (first argument is the video path)
cargo run --release -- myvideo.mp4
```

### Interactive setup

After starting the application, you'll be prompted to:

1. Enter the ASCII width (in characters)
2. Enter the ASCII height (in characters)
3. Enter the frame delay in milliseconds (controls playback speed)
4. Choose whether to invert brightness

### Playback controls

Once playback begins, you can use the following controls:

- `q`: Quit the application
- `p`: Pause/Play
- `←` `→`: Decrease/Increase playback speed
- `↑` `↓`: Skip backward/forward 10 frames
- `m`: Mute/Unmute audio
- `+` `-`: Increase/Decrease volume

## Audio in WSL

If you're running in Windows Subsystem for Linux (WSL), audio support requires additional setup:

1. Install PulseAudio on WSL:
```bash
sudo apt update
sudo apt install pulseaudio
```

2. Install PulseAudio on Windows from the [official releases](https://www.freedesktop.org/software/pulseaudio/releases/)

3. Configure PulseAudio to work between Windows and WSL (see full instructions in the PulseAudio documentation)

4. Start the PulseAudio server on Windows before running the application

## Project Structure

- `main.rs`: Application entry point and argument parsing
- `video_extraction.rs`: Core functionality for ASCII conversion and playback

## Dependencies

- `image`: For processing video frames
- `ffmpeg` (external): For extracting frames and audio
- `crossterm`: For terminal handling
- `ratatui`: For terminal UI
- `rodio`: For audio playback
- `rayon`: For parallel processing

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The ASCII art conversion technique is inspired by various text-based art projects
- Thanks to the Rust community for the excellent crates that made this project possible
