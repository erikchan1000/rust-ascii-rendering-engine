// main.rs
mod video_extraction;

use std::path::Path;
use std::io::{self, Write, BufRead};
use std::env;
use video_extraction::VideoExtractor;

fn main() -> Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();
    let mut video_path = "input_video.mp4";
    let mut audio_enabled = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    video_path = &args[i + 1];
                    i += 1;
                }
            },
            "--audio" => {
                audio_enabled = true;
            },
            _ => {
                if !args[i].starts_with("--") {
                    video_path = &args[i];
                }
            }
        }
        i += 1;
    }

    let mut extractor = VideoExtractor::new(video_path, audio_enabled)?;

    match extractor.load_metadata() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error loading video metadata: {}", e);
            eprintln!("Make sure FFmpeg is installed and the video file exists.");
            return Err(e);
        }
    }

    if let Some((width, height)) = extractor.dimensions() {
        println!("Video dimensions: {}x{}", width, height);
    } else {
        println!("Video dimensions: Unknown");
    }

    if let Some(frame_count) = extractor.frame_count() {
        println!("Estimated frame count: {}", frame_count);
    } else {
        println!("Estimated frame count: Unknown");
    }

    if let Some(duration) = extractor.duration() {
        println!("Video duration: {:.2} seconds", duration);
    } else {
        println!("Video duration: Unknown");
    }

    // Display options
    io::stdout().flush()?;

    // Read user input
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    if extractor.duration().is_none() {
        println!("Cannot play as ASCII: Video duration is unknown.");
        return Ok(());
    }

    // Get ASCII dimensions
    print!("Enter ASCII width (characters): ");
    io::stdout().flush()?;
    let mut width_str = String::new();
    handle.read_line(&mut width_str)?;
    let width: u32 = match width_str.trim().parse() {
        Ok(val) => val,
        Err(_) => {
            println!("Invalid width, using default of 80 characters");
            80
        }
    };

    print!("Enter ASCII height (characters): ");
    io::stdout().flush()?;
    let mut height_str = String::new();
    handle.read_line(&mut height_str)?;
    let height: u32 = match height_str.trim().parse() {
        Ok(val) => val,
        Err(_) => {
            println!("Invalid height, using default of 30 characters");
            30
        }
    };

    // Get playback speed
    print!("Enter frame delay in milliseconds (e.g., 100): ");
    io::stdout().flush()?;
    let mut delay_str = String::new();
    handle.read_line(&mut delay_str)?;
    let delay: u64 = match delay_str.trim().parse() {
        Ok(val) => val,
        Err(_) => {
            println!("Invalid delay, using default of 100ms");
            100
        }
    };

    // Ask if brightness should be inverted
    print!("Invert brightness? (y/n): ");
    io::stdout().flush()?;
    let mut invert_str = String::new();
    handle.read_line(&mut invert_str)?;
    let invert = invert_str.trim().to_lowercase() == "y";

    // Configure ASCII rendering in the extractor
    extractor.configure_ascii(width, height, invert);

    // Play the video directly as ASCII
    println!("Playing video as ASCII art (press Ctrl+C to stop)...");
    extractor.play_as_ascii(delay)?;

    Ok(())
}
