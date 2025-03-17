use crossterm::style::Color;
use crate::video_extraction::Frame;

#[derive(Debug, Clone, Copy)]
pub enum CharacterSet {
    Simple,
    Standard,
    Extended,
}

impl CharacterSet {

    pub fn get_chars(&self) -> &'static str {
        match self {
            CharacterSet::Simple => " .:-=+*#%@",
            CharacterSet::Standard => " .`^\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
            CharacterSet::Extended => " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$█▓▒░▄▀▐▌",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsciiChar {

    pub character: char,


    pub color: Option<(u8, u8, u8)>,
}

#[derive(Debug, Clone)]
pub struct AsciiFrame {

    pub width: u32,

    pub height: u32,

    pub data: Vec<AsciiChar>,

    pub timestamp: f64,
}

pub struct AsciiConverter {

    width: u32,

    height: u32,

    char_set: CharacterSet,

    use_color: bool,

    brightness: f32,

    contrast: f32,
}

impl AsciiConverter {

    pub fn new(width: u32, height: u32, char_set: CharacterSet, use_color: bool) -> Self {
        Self {
            width,
            height,
            char_set,
            use_color,
            brightness: 0.0,
            contrast: 1.0,
        }
    }

    pub fn convert(&self, frame: &Frame) -> AsciiFrame {

        let resized = if frame.width != self.width || frame.height != self.height {
            frame.resize(self.width, self.height * 2)
        } else {
            frame.clone()
        };

        let chars = self.char_set.get_chars();
        let char_count = chars.chars().count();

        let grayscale = resized.to_grayscale();

        let mut ascii_data = Vec::with_capacity((self.width * self.height) as usize);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * resized.width + x) as usize;

                if idx >= grayscale.len() {
                    continue;
                }


                let brightness = grayscale[idx];


                let adjusted = ((brightness as f32 / 255.0 - 0.5) * self.contrast + 0.5 + self.brightness)
                    .clamp(0.0, 1.0);


                let char_idx = ((char_count - 1) as f32 * adjusted) as usize;
                let ascii_char = chars.chars().nth(char_idx).unwrap_or(' ');


                let color = if self.use_color {
                    let rgb_idx = (y * resized.width + x) as usize * 3;
                    if rgb_idx + 2 < resized.data.len() {
                        Some((
                            resized.data[rgb_idx],
                            resized.data[rgb_idx + 1],
                            resized.data[rgb_idx + 2],
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };

                ascii_data.push(AsciiChar {
                    character: ascii_char,
                    color,
                });
            }
        }

        AsciiFrame {
            width: self.width,
            height: self.height,
            data: ascii_data,
            timestamp: frame.timestamp,
        }
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness.clamp(-1.0, 1.0);
    }


    pub fn set_contrast(&mut self, contrast: f32) {
        self.contrast = contrast.clamp(0.0, 2.0);
    }

    pub fn rgb_to_terminal_color(r: u8, g: u8, b: u8) -> Color {

        if r < 30 && g < 30 && b < 30 {
            return Color::Black;
        } else if r > 200 && g > 200 && b > 200 {
            return Color::White;
        } else if r > 200 && g < 100 && b < 100 {
            return Color::Red;
        } else if r < 100 && g > 200 && b < 100 {
            return Color::Green;
        } else if r < 100 && g < 100 && b > 200 {
            return Color::Blue;
        } else if r > 200 && g > 200 && b < 100 {
            return Color::Yellow;
        } else if r > 200 && g < 100 && b > 200 {
            return Color::Magenta;
        } else if r < 100 && g > 200 && b > 200 {
            return Color::Cyan;
        }

        Color::Rgb { r, g, b }
    }
}

pub fn render_to_string(frame: &AsciiFrame) -> String {
    let mut result = String::with_capacity((frame.width * frame.height) as usize + frame.height as usize);

    for y in 0..frame.height {
        for x in 0..frame.width {
            let idx = (y * frame.width + x) as usize;
            if idx < frame.data.len() {
                result.push(frame.data[idx].character);
            } else {
                result.push(' ');
            }
        }
        result.push('\n');
    }

    result
}

pub fn brightness_to_ascii(brightness: u8, char_set: &CharacterSet) -> char {
    let chars = char_set.get_chars();
    let char_count = chars.chars().count();
    let idx = (brightness as usize * (char_count - 1)) / 255;
    chars.chars().nth(idx).unwrap_or(' ')
}

pub fn render_ascii_frame_impl(
    stdout: &mut impl std::io::Write,
    frame: &AsciiFrame,
    use_color: bool,
) -> Result<(), std::io::Error> {
    use crossterm::style::{SetForegroundColor, ResetColor, Print};
    use crossterm::cursor::MoveTo;
    use crossterm::execute;

    execute!(stdout, MoveTo(0, 0))?;

    for y in 0..frame.height {
        for x in 0..frame.width {
            let idx = (y * frame.width + x) as usize;
            if idx >= frame.data.len() {
                continue;
            }

            let ascii_char = &frame.data[idx];

            if use_color && ascii_char.color.is_some() {
                let (r, g, b) = ascii_char.color.unwrap();
                let color = AsciiConverter::rgb_to_terminal_color(r, g, b);
                execute!(stdout, SetForegroundColor(color), Print(ascii_char.character))?;
            } else {
                execute!(stdout, Print(ascii_char.character))?;
            }
        }

        if y < frame.height - 1 {
            execute!(stdout, Print("\n"))?;
        }
    }

    if use_color {
        execute!(stdout, ResetColor)?;
    }

    Ok(())
}
