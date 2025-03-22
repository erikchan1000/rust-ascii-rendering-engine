use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{self, Error, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};
use std::fs;
use std::sync::{Arc, Mutex};
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use rayon::prelude::*;
use std::sync::mpsc;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Terminal
};
use rodio::{Decoder, OutputStream, Sink};
use image::GenericImageView;

const ASCII_CHARS: &str = " .,:;i1tfLCG08@";

pub struct VideoExtractor {
    file_path: String,
    width: Option<u32>,
    height: Option<u32>,
    frame_count: Option<u64>,
    duration: Option<f64>,

    ascii_width: Option<u32>,
    ascii_height: Option<u32>,
    ascii_invert: bool,

    // Audio playback options
    audio_enabled: bool,
    audio_volume: f32,
}

impl VideoExtractor {
    pub fn configure_ascii(&mut self, width: u32, height: u32, invert: bool) {
        self.ascii_width = Some(width);
        self.ascii_height = Some(height);
        self.ascii_invert = invert;
    }

    fn pixel_to_ascii(&self, r: u8, g: u8, b: u8) -> char {
        let brightness = 0.2126 * (r as f32) +
                         0.7152 * (g as f32) +
                         0.0722 * (b as f32);

        let normalized = brightness / 255.0;

        let brightness_index = if self.ascii_invert {
            1.0 - normalized
        } else {
            normalized
        };

        let ascii_index = (brightness_index * (ASCII_CHARS.len() - 1) as f32) as usize;
        ASCII_CHARS.chars().nth(ascii_index).unwrap_or(' ')
    }

    pub fn image_to_ascii<P: AsRef<Path>>(&self, image_path: P) -> Result<String, Error> {
        if self.ascii_width.is_none() || self.ascii_height.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ASCII rendering not configured. Call configure_ascii() first."
            ));
        }

        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(e) => return Err(Error::new(ErrorKind::Other, format!("Failed to open image: {}", e))),
        };

        let ascii_width = self.ascii_width.unwrap();
        let ascii_height = self.ascii_height.unwrap();

        let resized = img.resize_exact(
            ascii_width,
            ascii_height,
            image::imageops::FilterType::Lanczos3
        );

        let mut ascii_art = String::new();

        for y in 0..ascii_height {
            for x in 0..ascii_width {
                let pixel = resized.get_pixel(x, y);
                let ascii_char = self.pixel_to_ascii(pixel[0], pixel[1], pixel[2]);
                ascii_art.push(ascii_char);
            }
            ascii_art.push('\n');
        }

        Ok(ascii_art)
    }

    pub fn new<P: AsRef<Path>>(file_path: P, audio: bool) -> Result<Self, Error> {
        let path_str = file_path.as_ref()
            .to_str()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid file path"))?;

        let extractor = VideoExtractor {
            file_path: path_str.to_string(),
            width: None,
            height: None,
            frame_count: None,
            duration: None,
            ascii_width: None,
            ascii_height: None,
            ascii_invert: false,
            audio_enabled: audio,
            audio_volume: 0.5,
        };

        if !Path::new(path_str).exists() {
            return Err(Error::new(ErrorKind::NotFound, "Video file not found"));
        }

        Ok(extractor)
    }

    pub fn load_metadata(&mut self) -> Result<(), Error> {
        let output = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=width,height,nb_frames,duration",
                "-of", "csv=p=0",
                &self.file_path
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to execute ffprobe: {}",
                    String::from_utf8_lossy(&output.stderr))
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = output_str.trim().split(',').collect();

        if parts.len() >= 2 {
            self.width = parts[0].parse::<u32>().ok();
            self.height = parts[1].parse::<u32>().ok();

            if parts.len() >= 3 {
                self.frame_count = parts[2].parse::<u64>().ok();
            }

            if parts.len() >= 4 {
                self.duration = parts[3].parse::<f64>().ok();
            }
        }

        Ok(())
    }

    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    pub fn frame_count(&self) -> Option<u64> {
        self.frame_count
    }

    pub fn duration(&self) -> Option<f64> {
        self.duration
    }

    pub fn extract_frame_as_ascii(&self, timestamp: f64) -> Result<String, Error> {
        if self.ascii_width.is_none() || self.ascii_height.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ASCII rendering not configured. Call configure_ascii() first."
            ));
        }

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("frame_{}.jpg", timestamp));
        let temp_path = temp_file.to_str().ok_or_else(|| {
            Error::new(ErrorKind::Other, "Failed to create temporary file path")
        })?;

        let status = Command::new("ffmpeg")
            .args(&[
                "-hide_banner",
                "-loglevel", "error",
                "-ss", &timestamp.to_string(),
                "-i", &self.file_path,
                "-vframes", "1",
                "-q:v", "2",
                temp_path
            ])
            .status()?;

        if !status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to extract frame at specified timestamp"
            ));
        }

        let ascii_art = self.image_to_ascii(&temp_file)?;

        if let Err(e) = fs::remove_file(&temp_file) {
            eprintln!("Warning: Failed to remove temporary file: {}", e);
        }

        Ok(ascii_art)
    }

    fn extract_audio(&self, temp_dir: &Path) -> Result<String, Error> {
        let audio_file = temp_dir.join("audio.wav");
        let audio_path = audio_file.to_str().ok_or_else(|| {
            Error::new(ErrorKind::Other, "Failed to create audio file path")
        })?;

        // Check if ffmpeg is available
        match Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).status() {
            Ok(_) => {
                // ffmpeg is available, proceed with extraction
                let status = Command::new("ffmpeg")
                    .args(&[
                        "-hide_banner",
                        "-loglevel", "error",
                        "-i", &self.file_path,
                        "-vn", // No video
                        "-acodec", "pcm_s16le", // Convert to WAV
                        "-ar", "44100", // 44.1kHz sample rate
                        "-ac", "2", // Stereo
                        audio_path
                    ])
                    .status()?;

                if !status.success() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Failed to extract audio"
                    ));
                }

                Ok(audio_path.to_string())
            },
            Err(_) => {
                eprintln!("Warning: ffmpeg not found, audio extraction skipped");
                Err(Error::new(
                    ErrorKind::NotFound,
                    "ffmpeg not found"
                ))
            }
        }
    }

    pub fn play_as_ascii(&self, frame_delay_ms: u64) -> Result<(), Error> {
        if self.ascii_width.is_none() || self.ascii_height.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ASCII rendering not configured. Call configure_ascii() first."
            ));
        }

        let temp_dir = std::env::temp_dir().join("ascii_video_frames");
        fs::create_dir_all(&temp_dir)?;
        let temp_dir_path = temp_dir.to_str().ok_or_else(|| {
            Error::new(ErrorKind::Other, "Failed to create temporary directory path")
        })?;

        let fps = (1000.0 / frame_delay_ms as f64).ceil() as u32;
        let fps = std::cmp::min(fps, 15);

        println!("Extracting frames at {} FPS...", fps);

        let status = Command::new("ffmpeg")
            .args(&[
                "-hide_banner",
                "-loglevel", "error",
                "-i", &self.file_path,
                "-vf", &format!("scale=320:-1,fps={}", fps),
                "-q:v", "5",
                &format!("{}/frame_%04d.jpg", temp_dir_path)
            ])
            .status()?;

        if !status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to extract frames"
            ));
        }

        println!("Frames extracted. Processing to ASCII...");

        let mut frame_paths: Vec<_> = fs::read_dir(&temp_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext == "jpg")
                    .unwrap_or(false)
            })
            .collect();

        frame_paths.sort();

        println!("Converting {} frames to ASCII (this may take a moment)...", frame_paths.len());

        let ascii_frames: Vec<String> = frame_paths
            .par_iter()
            .map(|path| {
                match self.image_to_ascii(path) {
                    Ok(ascii) => ascii,
                    Err(_) => String::from("Error converting frame to ASCII")
                }
            })
            .collect();

        // Extract audio if enabled
        let audio_path = if self.audio_enabled {
            println!("Extracting audio...");
            match self.extract_audio(&temp_dir) {
                Ok(path) => {
                    println!("Audio extracted successfully");
                    Some(path)
                },
                Err(e) => {
                    eprintln!("Warning: Failed to extract audio: {}", e);
                    None
                }
            }
        } else {
            None
        };

        println!("ASCII conversion complete. Starting playback...");
        println!("Press 'q' to quit, 'p' to pause/play, arrow keys to adjust speed, 'm' to mute/unmute, '+'/'-' to adjust volume");

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let (tx, rx) = mpsc::channel();
        let mut paused = false;
        let mut current_frame = 0;
        let mut current_delay = frame_delay_ms;
        let total_frames = ascii_frames.len();
        let video_name = Path::new(&self.file_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("video"))
            .to_string_lossy();

        // Setup audio playback - make it optional
        let mut audio_setup_success = false;
        let (_stream, _, sink) = match OutputStream::try_default() {
            Ok((stream, handle)) => {
                match Sink::try_new(&handle) {
                    Ok(sink) => {
                        audio_setup_success = true;
                        (stream, handle, Some(sink))
                    },
                    Err(e) => {
                        eprintln!("Warning: Failed to create audio sink: {}", e);
                        (stream, handle, None)
                    }
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to initialize audio output: {}", e);
                // Create dummy values that won't be used
                let (stream, handle) = match OutputStream::try_default() {
                    Ok(result) => result,
                    Err(_) => {
                        eprintln!("Warning: Failed to create dummy audio output");
                        OutputStream::try_default().unwrap()
                    }
                };
                (stream, handle, None)
            }
        };

        // Set up audio variables
        let mut audio_muted = false;
        let mut current_volume = self.audio_volume;

        // Set initial volume and start audio playback if possible
        let sink_arc = if let Some(sink) = sink {
            sink.set_volume(self.audio_volume);

            // Start audio playback if enabled and audio setup succeeded
            if audio_setup_success && self.audio_enabled {
                if let Some(audio_path) = &audio_path {
                    match fs::File::open(audio_path) {
                        Ok(file) => {
                            match Decoder::new(file) {
                                Ok(source) => {
                                    sink.append(source);
                                    if paused {
                                        sink.pause();
                                    } else {
                                        sink.play();
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Warning: Failed to decode audio: {}", e);
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Warning: Failed to open audio file: {}", e);
                        }
                    }
                }
            }

            Some(Arc::new(Mutex::new(sink)))
        } else {
            None
        };

        // Shared audio control state
        let paused_state = Arc::new(Mutex::new(paused));

        thread::spawn(move || {
            loop {
                if event::poll(Duration::from_millis(100)).unwrap() {
                    if let Event::Key(key) = event::read().unwrap() {
                        tx.send(key.code).unwrap();
                    }
                }
            }
        });

        let mut last_frame_time = Instant::now();
        loop {
            while let Ok(key_code) = rx.try_recv() {
                match key_code {
                    KeyCode::Char('q') => {
                        // Stop audio before exiting
                        if let Some(sink_arc) = &sink_arc {
                            if let Ok(sink) = sink_arc.lock() {
                                sink.stop();
                            }
                        }

                        disable_raw_mode()?;
                        execute!(
                            terminal.backend_mut(),
                            LeaveAlternateScreen,
                            DisableMouseCapture
                        )?;
                        terminal.show_cursor()?;

                        println!("Playback complete. Cleaning up temporary files...");
                        fs::remove_dir_all(temp_dir)?;

                        return Ok(());
                    },
                    KeyCode::Char('p') => {
                        paused = !paused;

                        // Update audio playback state
                        if let Ok(mut paused_guard) = paused_state.lock() {
                            *paused_guard = paused;
                        }

                        if let Some(sink_arc) = &sink_arc {
                            if let Ok(sink) = sink_arc.lock() {
                                if paused {
                                    sink.pause();
                                } else {
                                    sink.play();
                                }
                            }
                        }
                    },
                    KeyCode::Char('m') => {
                        // Toggle mute
                        audio_muted = !audio_muted;
                        if let Some(sink_arc) = &sink_arc {
                            if let Ok(sink) = sink_arc.lock() {
                                if audio_muted {
                                    sink.set_volume(0.0);
                                } else {
                                    sink.set_volume(current_volume);
                                }
                            }
                        }
                    },
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        // Increase volume
                        current_volume = (current_volume + 0.1).min(1.0);
                        if !audio_muted {
                            if let Some(sink_arc) = &sink_arc {
                                if let Ok(sink) = sink_arc.lock() {
                                    sink.set_volume(current_volume);
                                }
                            }
                        }
                    },
                    KeyCode::Char('-') => {
                        // Decrease volume
                        current_volume = (current_volume - 0.1).max(0.0);
                        if !audio_muted {
                            if let Some(sink_arc) = &sink_arc {
                                if let Ok(sink) = sink_arc.lock() {
                                    sink.set_volume(current_volume);
                                }
                            }
                        }
                    },
                    KeyCode::Left => {
                        current_delay = (current_delay as f64 * 1.2).min(500.0) as u64;
                    },
                    KeyCode::Right => {
                        current_delay = (current_delay as f64 * 0.8).max(16.0) as u64;
                    },
                    KeyCode::Up => {
                        if current_frame >= 10 {
                            current_frame -= 10;
                        } else {
                            current_frame = 0;
                        }
                    },
                    KeyCode::Down => {
                        if current_frame + 10 < total_frames {
                            current_frame += 10;
                        }
                    },
                    _ => {}
                }
            }

            let now = Instant::now();
            let elapsed = now.duration_since(last_frame_time);

            if !paused && elapsed >= Duration::from_millis(current_delay) {
                last_frame_time = now;
                current_frame = (current_frame + 1) % total_frames;
            }

            terminal.draw(|f| {
                let size = f.area();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(3),
                    ])
                    .split(size);

                let volume_status = if !audio_setup_success {
                    "NO AUDIO"
                } else if audio_muted {
                    "MUTED"
                } else {
                    match (current_volume * 10.0).round() as i32 {
                        0 => "VOL: 0%",
                        1 => "VOL: 10%",
                        2 => "VOL: 20%",
                        3 => "VOL: 30%",
                        4 => "VOL: 40%",
                        5 => "VOL: 50%",
                        6 => "VOL: 60%",
                        7 => "VOL: 70%",
                        8 => "VOL: 80%",
                        9 => "VOL: 90%",
                        _ => "VOL: 100%",
                    }
                };

                let status = format!(
                    "Playing: {} | Frame: {}/{} | FPS: {:.1} | {} | {}",
                    video_name,
                    current_frame + 1,
                    total_frames,
                    1000.0 / current_delay as f64,
                    if paused { "PAUSED" } else { "PLAYING" },
                    volume_status
                );

                let status_widget = Paragraph::new(status)
                    .block(Block::default().borders(Borders::ALL).title("ASCII Video Player"))
                    .alignment(Alignment::Center)
                    .style(Style::default());

                let ascii_content = &ascii_frames[current_frame];
                let ascii_widget = Paragraph::new(ascii_content.to_string())
                    .style(Style::default());

                let controls = "Controls: q - Quit | p - Pause/Play | m - Mute/Unmute | +/- - Volume | ← → - Change Speed | ↑ ↓ - Skip 10 Frames";
                let controls_widget = Paragraph::new(controls)
                    .block(Block::default().borders(Borders::ALL))
                    .alignment(Alignment::Center)
                    .style(Style::default());

                f.render_widget(status_widget, chunks[0]);
                f.render_widget(ascii_widget, chunks[1]);
                f.render_widget(controls_widget, chunks[2]);
            })?;

            thread::sleep(Duration::from_millis(10));
        }
    }
}
