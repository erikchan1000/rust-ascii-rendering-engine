use std::path::Path;
use ffmpeg_next as ffmpeg;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg::util::rational::Rational;

use crate::video_extraction::{Config, Error, Frame, Result, ScalingQuality};

pub struct VideoReader {
        path: String,

        config: Config,

        context: Option<ffmpeg::format::context::Input>,

        stream_index: usize,

        scaler: Option<Context>,

        frame_count: u64,

        skip_counter: u32,

        duration: f64,

        frame_rate: f64,

        width: u32,

        height: u32,
}

impl VideoReader {
        pub fn new<P: AsRef<Path>>(path: P, config: Option<Config>) -> Result<Self> {
                ffmpeg::init().map_err(|e| Error::DecodingError(e.to_string()))?;

        Ok(Self {
            path: path.as_ref().to_string_lossy().to_string(),
            config: config.unwrap_or_default(),
            context: None,
            stream_index: 0,
            scaler: None,
            frame_count: 0,
            skip_counter: 0,
            duration: 0.0,
            frame_rate: 0.0,
            width: 0,
            height: 0,
        })
    }

        pub fn open(&mut self) -> Result<()> {
                let context = input(&self.path)?;

                let stream = context.streams()
            .best(Type::Video)
            .ok_or_else(|| Error::FormatError("No video stream found".to_string()))?;

        self.stream_index = stream.index();

                let codec_context = stream.codec();
        let decoder = codec_context.decoder().video()?;

        self.width = decoder.width();
        self.height = decoder.height();

                let frame_rate = stream.avg_frame_rate();
        self.frame_rate = frame_rate.0 as f64 / frame_rate.1.max(1) as f64;

                        let time_base = Rational(1, 1000000);         let duration = context.duration() as f64 * f64::from(time_base.0) / f64::from(time_base.1);
        self.duration = duration;

                self.context = Some(context);

        Ok(())
    }

        pub fn next_frame(&mut self) -> Result<Frame> {
        let context = self.context.as_mut()
            .ok_or_else(|| Error::DecodingError("Video not opened".to_string()))?;

        let stream = context.stream(self.stream_index).unwrap();
        let decoder = stream.codec().decoder().video()?;

        if self.scaler.is_none() {
            let target_width = self.config.resolution.map_or(self.width, |(w, _)| w);
            let target_height = self.config.resolution.map_or(self.height, |(_, h)| h);

            let flags = match self.config.scaling_quality {
                ScalingQuality::Fast => Flags::BILINEAR,
                ScalingQuality::Balanced => Flags::BICUBIC,
                ScalingQuality::Best => Flags::LANCZOS,
            };

            self.scaler = Some(Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::RGB24,
                target_width,
                target_height,
                flags,
            )?);
        }

        let mut decoded = Video::empty();
        let mut rgb_frame = Video::empty();

        let mut decoder = stream.codec().decoder().video()?;
        let time_base = stream.time_base();

        while let Some((stream_index, packet)) = context.packets().next() {
            if stream_index.index() == self.stream_index {
                decoder.send_packet(&packet)?;

                if decoder.receive_frame(&mut decoded).is_ok() {
                                        if self.skip_counter < self.config.skip_frames - 1 {
                        self.skip_counter += 1;
                        continue;
                    }
                    self.skip_counter = 0;

                    let scaler = self.scaler.as_mut().unwrap();
                    scaler.run(&decoded, &mut rgb_frame)?;

                                        let width = rgb_frame.width();
                    let height = rgb_frame.height();
                    let data = rgb_frame.data(0).to_vec();

                                        let pts = decoded.pts().unwrap_or_else(|| packet.pts().unwrap_or(0));
                    let timestamp = pts as f64 * f64::from(time_base.0) / f64::from(time_base.1);

                    self.frame_count += 1;
                    return Ok(Frame::new(width, height, data, timestamp, self.frame_count - 1));
                }
            }
        }

        Err(Error::EndOfStream)
    }

        pub fn extract_frames(&mut self, count: usize) -> Result<Vec<Frame>> {
        let mut frames = Vec::with_capacity(count);

        for _ in 0..count {
            match self.next_frame() {
                Ok(frame) => frames.push(frame),
                Err(Error::EndOfStream) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(frames)
    }

        pub fn seek(&mut self, time_seconds: f64) -> Result<()> {
        let context = self.context.as_mut()
            .ok_or_else(|| Error::DecodingError("Video not opened".to_string()))?;

                        let time_base = Rational(1, 1000000);         let timestamp = (time_seconds * f64::from(time_base.1) / f64::from(time_base.0)) as i64;

                context.seek(timestamp, ..0)?;

                self.frame_count = (time_seconds * self.frame_rate) as u64;

        Ok(())
    }

        pub fn duration(&self) -> f64 {
        self.duration
    }

        pub fn frame_rate(&self) -> f64 {
        self.frame_rate
    }

        pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

        pub fn target_dimensions(&self) -> (u32, u32) {
        self.config.resolution.unwrap_or((self.width, self.height))
    }

        pub fn list_streams(&self) -> Result<Vec<StreamInfo>> {
        let context = self.context.as_ref()
            .ok_or_else(|| Error::DecodingError("Video not opened".to_string()))?;

        let mut streams = Vec::new();

        for (index, stream) in context.streams().enumerate() {
            let codec = stream.codec();
            let medium = codec.medium();

            let stream_type = match medium {
                Type::Video => "video",
                Type::Audio => "audio",
                Type::Subtitle => "subtitle",
                Type::Data => "data",
                _ => "unknown",
            };


            let codec_name = codec.id().name();

            streams.push(StreamInfo {
                index,
                stream_type: stream_type.to_string(),
                codec: codec_name.to_string(),
            });
        }

        Ok(streams)
    }

        pub fn metadata(&self) -> Result<VideoMetadata> {
        let context = self.context.as_ref()
            .ok_or_else(|| Error::DecodingError("Video not opened".to_string()))?;

        let mut metadata = std::collections::HashMap::new();

        for (k, v) in context.metadata().iter() {
            metadata.insert(k.to_string(), v.to_string());
        }

        Ok(VideoMetadata {
            filename: self.path.clone(),
            width: self.width,
            height: self.height,
            duration: self.duration,
            frame_rate: self.frame_rate,
            format: context.format().name().to_string(),
            metadata,
        })
    }

        pub fn close(&mut self) -> Result<()> {
        self.context = None;
        self.scaler = None;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
        pub index: usize,

        pub stream_type: String,

        pub codec: String,
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
        pub filename: String,

        pub width: u32,

        pub height: u32,

        pub duration: f64,

        pub frame_rate: f64,

        pub format: String,

        pub metadata: std::collections::HashMap<String, String>,
}

impl Drop for VideoReader {
    fn drop(&mut self) {
                let _ = self.close();
    }
}
