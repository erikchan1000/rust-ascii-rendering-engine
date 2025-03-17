pub mod frame;
pub mod reader;

pub use frame::Frame;
pub use reader::VideoReader;

#[derive(Debug)]
pub enum Error {
        IoError(std::io::Error),

        DecodingError(String),

        FormatError(String),

        EndOfStream,

        FFmpegError(ffmpeg_next::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::DecodingError(msg) => write!(f, "Decoding error: {}", msg),
            Error::FormatError(msg) => write!(f, "Format error: {}", msg),
            Error::EndOfStream => write!(f, "End of stream reached"),
            Error::FFmpegError(e) => write!(f, "FFmpeg error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(e) => Some(e),
            Error::FFmpegError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<ffmpeg_next::Error> for Error {
    fn from(error: ffmpeg_next::Error) -> Self {
        Error::FFmpegError(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Config {
        pub frame_rate: Option<f64>,

        pub resolution: Option<(u32, u32)>,

        pub color: bool,

        pub skip_frames: u32,

        pub scaling_quality: ScalingQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingQuality {
        Fast,
        Balanced,
        Best,
}

impl Default for ScalingQuality {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            frame_rate: None,
            resolution: None,
            color: false,
            skip_frames: 1,
            scaling_quality: ScalingQuality::default(),
        }
    }
}
