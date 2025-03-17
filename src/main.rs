
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use std::thread::sleep;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType}
};

use clap::Parser;

mod video_extraction;
mod ascii_converter;

use video_extraction::{Config, Frame, VideoReader};
use ascii_converter::{AsciiConverter, AsciiFrame, CharacterSet};


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {

    #[clap(short, long)]
    input: String,

    #[clap(short, long, default_value_t = 80)]
    width: u32,

    #[clap(short, long, default_value_t = 40)]
    height: u32,

    #[clap(short, long, default_value = "standard")]
    charset: String,

    #[clap(short, long)]
    color: bool,

    #[clap(short, long)]
    fps: Option<f64>,

    #[clap(long, default_value_t = 1)]
    skip: u32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = Config {
        resolution: Some((args.width * 2, args.height * 2)),
        color: args.color,
        frame_rate: args.fps,
        skip_frames: args.skip,
        ..Default::default()
    };

    let mut reader = VideoReader::new(&args.input, Some(config))?;
    reader.open()?;

    let duration = reader.duration();
    let native_frame_rate = reader.frame_rate();
    let (width, height) = reader.target_dimensions();

    let char_set = match args.charset.as_str() {
        "simple" => CharacterSet::Simple,
        "extended" => CharacterSet::Extended,
        _ => CharacterSet::Standard,
    };

    let converter = AsciiConverter::new(args.width, args.height, char_set, args.color);

    println!("Video: {}", args.input);
    println!("Duration: {:.2} seconds", duration);
    println!("Frame rate: {:.2} fps", native_frame_rate);
    println!("Resolution: {}x{}", width, height);
    println!("ASCII output: {}x{}", args.width, args.height);
    println!("\nPress Space to pause/resume, Q to quit");
    println!("\nStarting playback in 2 seconds...");
    sleep(Duration::from_secs(2));

    setup_terminal()?;

    let result = play_video(&mut reader, &converter, args.fps.unwrap_or(native_frame_rate));

    restore_terminal()?;

    result
}

fn setup_terminal() -> Result<(), std::io::Error> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        Clear(ClearType::All)
    )?;
    Ok(())
}

fn restore_terminal() -> Result<(), std::io::Error> {
    let mut stdout = stdout();
    execute!(
        stdout,
        terminal::LeaveAlternateScreen,
        cursor::Show,
        ResetColor
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn play_video(
    reader: &mut VideoReader,
    converter: &AsciiConverter,
    frame_rate: f64,
) -> anyhow::Result<()> {
    let mut stdout = stdout();
    let frame_duration = Duration::from_secs_f64(1.0 / frame_rate);
    let mut paused = false;
    let mut last_frame_time = Instant::now();

    while let Ok(frame) = reader.next_frame() {

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => paused = !paused,
                    _ => {}
                }
            }
        }

        if paused {

            sleep(Duration::from_millis(100));
            continue;
        }

        let ascii_frame = converter.convert(&frame);

        let elapsed = last_frame_time.elapsed();
        if elapsed < frame_duration {
            sleep(frame_duration - elapsed);
        }

        render_ascii_frame(&mut stdout, &ascii_frame)?;

        last_frame_time = Instant::now();
    }

    Ok(())
}

fn render_ascii_frame(stdout: &mut impl Write, frame: &AsciiFrame) -> Result<(), std::io::Error> {

    ascii_converter::render_ascii_frame_impl(stdout, frame, frame.data.first().and_then(|c| c.color).is_some())
}
