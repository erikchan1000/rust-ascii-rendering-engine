// video_extraction.rs
use std::path::Path;
use std::process::Command;
use std::io::{self, Error, ErrorKind, Write};
use std::fs;
use rayon::prelude::*;
use std::sync::Arc;

use image::GenericImageView;

/// ASCII intensity ramp (from darkest to brightest)
const ASCII_CHARS: &str = " .,:;i1tfLCG08@";

/// A struct that represents a video file to be processed
pub struct VideoExtractor {
    file_path: String,
    width: Option<u32>,
    height: Option<u32>,
    frame_count: Option<u64>,
    duration: Option<f64>,
    // ASCII rendering options
    ascii_width: Option<u32>,
    ascii_height: Option<u32>,
    ascii_invert: bool,
}

impl VideoExtractor {
    /// Configure ASCII rendering options
    pub fn configure_ascii(&mut self, width: u32, height: u32, invert: bool) {
        self.ascii_width = Some(width);
        self.ascii_height = Some(height);
        self.ascii_invert = invert;
    }

    /// Convert pixel brightness to an ASCII character
    fn pixel_to_ascii(&self, r: u8, g: u8, b: u8) -> char {
        // Calculate brightness using perceptual weights
        let brightness = 0.2126 * (r as f32) +
                         0.7152 * (g as f32) +
                         0.0722 * (b as f32);

        // Normalize to 0.0 - 1.0
        let normalized = brightness / 255.0;

        // Invert if needed
        let brightness_index = if self.ascii_invert {
            1.0 - normalized
        } else {
            normalized
        };

        // Map to ASCII character
        let ascii_index = (brightness_index * (ASCII_CHARS.len() - 1) as f32) as usize;
        ASCII_CHARS.chars().nth(ascii_index).unwrap_or(' ')
    }

    /// Convert an image to ASCII art and return it as a string
    pub fn image_to_ascii<P: AsRef<Path>>(&self, image_path: P) -> Result<String, Error> {
        if self.ascii_width.is_none() || self.ascii_height.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ASCII rendering not configured. Call configure_ascii() first."
            ));
        }

        // Use the 'image' crate to load and process the image
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(e) => return Err(Error::new(ErrorKind::Other, format!("Failed to open image: {}", e))),
        };

        // Resize image to ASCII dimensions
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
    /// Create a new VideoExtractor instance for the given video file
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
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
        };

        // Validate that the file exists
        if !Path::new(path_str).exists() {
            return Err(Error::new(ErrorKind::NotFound, "Video file not found"));
        }

        Ok(extractor)
    }

    /// Load video metadata using FFmpeg
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

    /// Get estimated frame count
    pub fn frame_count(&self) -> Option<u64> {
        self.frame_count
    }

    /// Get video duration in seconds
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }

    /// Extract a single frame at the specified timestamp and display as ASCII (without saving image)
    pub fn extract_frame_as_ascii(&self, timestamp: f64) -> Result<String, Error> {
        if self.ascii_width.is_none() || self.ascii_height.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "ASCII rendering not configured. Call configure_ascii() first."
            ));
        }

        // Create a temporary file for the frame
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("frame_{}.jpg", timestamp));
        let temp_path = temp_file.to_str().ok_or_else(|| {
            Error::new(ErrorKind::Other, "Failed to create temporary file path")
        })?;

        // Extract the frame
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

        // Convert to ASCII
        let ascii_art = self.image_to_ascii(&temp_file)?;

        // Clean up temp file
        if let Err(e) = fs::remove_file(&temp_file) {
            eprintln!("Warning: Failed to remove temporary file: {}", e);
        }

        Ok(ascii_art)
    }

/// Play the video directly as ASCII art without saving frames
pub fn play_as_ascii(&self, frame_delay_ms: u64) -> Result<(), Error> {
    if self.ascii_width.is_none() || self.ascii_height.is_none() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "ASCII rendering not configured. Call configure_ascii() first."
        ));
    }

    let duration = self.duration
        .ok_or_else(|| Error::new(ErrorKind::Other, "Video duration unknown. Call load_metadata() first."))?;

    // Create a temporary directory for extracted frames
    let temp_dir = std::env::temp_dir().join("ascii_video_frames");
    fs::create_dir_all(&temp_dir)?;
    let temp_dir_path = temp_dir.to_str().ok_or_else(|| {
        Error::new(ErrorKind::Other, "Failed to create temporary directory path")
    })?;

    // Calculate frame rate based on delay
    let fps = (1000.0 / frame_delay_ms as f64).ceil() as u32;
    // Prevent too high FPS that would create too many files
    let fps = std::cmp::min(fps, 15);

    println!("Extracting frames at {} FPS...", fps);

    // Extract all frames at once using FFmpeg with downscaling for better performance
    let status = Command::new("ffmpeg")
        .args(&[
            "-hide_banner",
            "-loglevel", "error",
            "-i", &self.file_path,
            "-vf", &format!("scale=320:-1,fps={}", fps), // Scale video down first
            "-q:v", "5", // Lower quality for faster processing
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

    // Get all frame files
    let mut frame_paths: Vec<_> = fs::read_dir(&temp_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|ext| ext == "jpg")
                .unwrap_or(false)
        })
        .collect();

    // Sort frames by name to ensure correct order
    frame_paths.sort();

    println!("Converting {} frames to ASCII (this may take a moment)...", frame_paths.len());

    // Convert frames to ASCII in parallel using rayon
    let ascii_frames: Vec<String> = frame_paths
        .par_iter() // Parallel iterator from rayon
        .map(|path| {
            match self.image_to_ascii(path) {
                Ok(ascii) => ascii,
                Err(_) => String::from("Error converting frame to ASCII")
            }
        })
        .collect();

    println!("ASCII conversion complete. Starting playback...");

    // ANSI escape code for clearing the screen
    let clear_code = "\x1B[2J\x1B[H";

    // Play the frames from memory (much faster than processing during playback)
    for ascii_frame in &ascii_frames {
        // Clear screen and display the frame
        print!("{}", clear_code);
        println!("{}", ascii_frame);
        io::stdout().flush()?;

        // Wait for the specified delay
        std::thread::sleep(std::time::Duration::from_millis(frame_delay_ms));
    }

    // Clean up temporary directory
    println!("Playback complete. Cleaning up temporary files...");
    fs::remove_dir_all(temp_dir)?;

    Ok(())
}
}
