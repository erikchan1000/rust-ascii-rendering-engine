use std::fmt;
use image::{ImageBuffer, Rgb, RgbImage};

#[derive(Clone)]
pub struct Frame {
        pub width: u32,

        pub height: u32,

        pub data: Vec<u8>,

        pub timestamp: f64,

        pub index: u64,
}

impl Frame {
        pub fn new(width: u32, height: u32, data: Vec<u8>, timestamp: f64, index: u64) -> Self {
        Self {
            width,
            height,
            data,
            timestamp,
            index,
        }
    }

        pub fn empty(width: u32, height: u32) -> Self {
        let size = width as usize * height as usize * 3;         Self {
            width,
            height,
            data: vec![0; size],
            timestamp: 0.0,
            index: 0,
        }
    }

        pub fn get_pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let idx = (y * self.width + x) as usize * 3;
        if idx + 2 < self.data.len() {
            Some((
                self.data[idx],                     self.data[idx + 1],                 self.data[idx + 2],             ))
        } else {
            None
        }
    }

        pub fn to_grayscale(&self) -> Vec<u8> {
                let img = self.to_image();

                let gray_img = image::imageops::grayscale(&img);

                gray_img.into_raw()
    }

        pub fn to_image(&self) -> RgbImage {
                let expected_len = (self.width * self.height * 3) as usize;
        let data = if self.data.len() < expected_len {
            let mut padded = self.data.clone();
            padded.resize(expected_len, 0);
            padded
        } else if self.data.len() > expected_len {
            self.data[0..expected_len].to_vec()
        } else {
            self.data.clone()
        };

                ImageBuffer::from_fn(self.width, self.height, |x, y| {
            let idx = ((y * self.width + x) * 3) as usize;
            Rgb([data[idx], data[idx + 1], data[idx + 2]])
        })
    }

        pub fn from_image(img: &RgbImage, timestamp: f64, index: u64) -> Self {
        Self {
            width: img.width(),
            height: img.height(),
            data: img.as_raw().clone(),
            timestamp,
            index,
        }
    }

        pub fn resize(&self, new_width: u32, new_height: u32) -> Frame {
        let img = self.to_image();

                let resized = image::imageops::resize(
            &img,
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3
        );

        Frame::from_image(&resized, self.timestamp, self.index)
    }

        pub fn adjust_brightness_contrast(&self, brightness: f32, contrast: f32) -> Frame {
        let img = self.to_image();

                let adjusted = ImageBuffer::from_fn(self.width, self.height, |x, y| {
            let pixel = img.get_pixel(x, y);

                        let apply_to_channel = |c: u8| -> u8 {
                let c_f32 = c as f32 / 255.0;
                let adjusted = (c_f32 - 0.5) * contrast + 0.5 + brightness;
                let clamped = adjusted.clamp(0.0, 1.0);
                (clamped * 255.0) as u8
            };

            Rgb([
                apply_to_channel(pixel[0]),
                apply_to_channel(pixel[1]),
                apply_to_channel(pixel[2]),
            ])
        });

        Frame::from_image(&adjusted, self.timestamp, self.index)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("timestamp", &self.timestamp)
            .field("index", &self.index)
            .field("data_size", &self.data.len())
            .finish()
    }
}

pub struct FrameBuffer {
        frames: Vec<Frame>,

        capacity: usize,

        position: usize,
}

impl FrameBuffer {
        pub fn new(capacity: usize) -> Self {
        Self {
            frames: Vec::with_capacity(capacity),
            capacity,
            position: 0,
        }
    }

        pub fn push(&mut self, frame: Frame) {
        if self.frames.len() >= self.capacity {
            self.frames.remove(0);
            if self.position > 0 {
                self.position -= 1;
            }
        }
        self.frames.push(frame);
    }

        pub fn get(&self, index: usize) -> Option<&Frame> {
        self.frames.get(index)
    }

        pub fn latest(&self) -> Option<&Frame> {
        self.frames.last()
    }

        pub fn len(&self) -> usize {
        self.frames.len()
    }

        pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

        pub fn clear(&mut self) {
        self.frames.clear();
        self.position = 0;
    }

        pub fn next(&mut self) -> Option<&Frame> {
        if self.position < self.frames.len() {
            let frame = &self.frames[self.position];
            self.position += 1;
            Some(frame)
        } else {
            None
        }
    }

        pub fn previous(&mut self) -> Option<&Frame> {
        if self.position > 0 {
            self.position -= 1;
            Some(&self.frames[self.position])
        } else {
            None
        }
    }

        pub fn rewind(&mut self) {
        self.position = 0;
    }
}
