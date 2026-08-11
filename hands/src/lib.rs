use encoding_rs::SHIFT_JIS;

pub type Stroke = Vec<(f32, f32)>;
pub type CharacterStrokes = Vec<Stroke>;

#[derive(Debug, Clone, Default)]
pub struct HandParser {
    native_language: String,
    writing_hand: String,
    dominant_hand: String,
    motivation: String,
    birthday: String,
    first_date: String,
    first_time: String,
    last_date: String,
    last_time: String,
    sex: String,
    occupation: String,
    stroke_data: Vec<(char, CharacterStrokes)>,
    frame_start: (f32, f32),
    frame_step: (f32, f32),
    frame_count: (f32, f32),
    frame_size: (f32, f32),
    display_resolution: (f32, f32),
    input_resolution: (f32, f32),
    frame_index: (f32, f32),
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line_number: usize,
    pub kind: ErrorKind,
}
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ErrorKind {
    UnexpectedSymbol,
    UnexpectedEnd,
    Encoding,
    Integer,
}
/// Parses every file's raw bytes into the combined character/stroke list.
///
/// # Errors
/// Returns a [`ParseError`] if any file contains a malformed header field,
/// a coordinate line that can't be decoded (Shift-JIS) or parsed as a number.
#[inline]
pub fn parse_files(files_content: &[Vec<u8>]) -> Result<Vec<(char, CharacterStrokes)>, ParseError> {
    let mut res = Vec::new();
    let mut parser = HandParser::new();
    for file_content in files_content {
        parser = parser.parse(file_content)?;
        res.append(&mut parser.stroke_data);
        parser = HandParser::new();
    }
    Ok(res)
}

impl HandParser {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Parses one file's raw bytes into `self`, consuming and returning it.
    ///
    /// # Errors
    /// Returns a [`ParseError`] if a line has an invalid encoding, a meta field
    /// or coordinate can't be parsed, or the input is otherwise malformed.
    #[inline]
    pub fn parse(mut self, raw: &[u8]) -> Result<Self, ParseError> {
        for (n, l) in raw.split(|c| *c == b'\n').enumerate() {
            self.parse_line(l, n.saturating_add(1))?;
        }
        Ok(self)
    }
    fn parse_line(&mut self, mut line: &[u8], line_number: usize) -> Result<(), ParseError> {
        line = line.strip_prefix(b"\r\n").unwrap_or(line);
        line = line.strip_prefix(b"\"").unwrap_or(line);

        if line.starts_with(b"native language") {
            self.native_language = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"writing hand") {
            self.writing_hand = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"dominant hand") {
            self.dominant_hand = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"motivation") {
            self.motivation = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"sex") {
            self.sex = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"occupation") {
            self.occupation = Self::parse_meta_string(line, line_number)?;
        } else if line.starts_with(b"birth day") {
            self.birthday = Self::parse_meta_time(line, line_number)?;
        } else if line.starts_with(b"first date") {
            self.first_date = Self::parse_meta_time(line, line_number)?;
        } else if line.starts_with(b"first time") {
            self.first_time = Self::parse_meta_time(line, line_number)?;
        } else if line.starts_with(b"last date") {
            self.last_date = Self::parse_meta_time(line, line_number)?;
        } else if line.starts_with(b"last time") {
            self.last_time = Self::parse_meta_time(line, line_number)?;
        } else if line.starts_with(b"frame start") {
            self.frame_start = Self::parse_meta_int_tuple(line, line_number)?;
        } else if line.starts_with(b"frame step") {
            self.frame_step = Self::parse_meta_int_tuple(line, line_number)?;
        } else if line.starts_with(b"frame count") {
            self.frame_count = Self::parse_meta_int_tuple(line, line_number)?;
            // start at the last frame so that the first char can wrap to first frame
            self.frame_index = self.frame_count;
        } else if line.starts_with(b"frame size") {
            self.frame_size = Self::parse_meta_int_tuple(line, line_number)?;
        } else if line.starts_with(b"display resolution") {
            self.display_resolution = Self::parse_meta_int_tuple(line, line_number)?;
        } else if line.starts_with(b"input resolution") {
            self.input_resolution = Self::parse_meta_int_tuple(line, line_number)?;
        } else if line.starts_with(b"[") {
            let char = Self::parse_char(line, line_number)?;
            self.stroke_data.push((char, Vec::new()));
            self.inc_frame_index();
        } else if line.starts_with(b"2") {
            if let Some((_char, strokes)) = self.stroke_data.last_mut() {
                strokes.push(Vec::new());
            }
            let (x, y) = Self::parse_coord(line, line_number)?;
            self.transform_and_push_point(x, y);
        } else if line.starts_with(b"0") || line.starts_with(b"4") {
            let (x, y) = Self::parse_coord(line, line_number)?;
            self.transform_and_push_point(x, y);
        }
        Ok(())
    }
    fn transform_and_push_point(&mut self, mut x: f32, mut y: f32) {
        if let Some((_char, strokes)) = self.stroke_data.last_mut()
            && let Some(stroke) = strokes.last_mut()
        {
            let frame_origin_x = (self.frame_index.0 * self.frame_step.0 + self.frame_start.0)
                * ((self.input_resolution.0) / (self.display_resolution.0));
            let frame_origin_y = (self.frame_index.1 * self.frame_step.1 + self.frame_start.1)
                * ((self.input_resolution.1) / (self.display_resolution.1));
            let x_width =
                self.frame_size.0 * ((self.input_resolution.0) / (self.display_resolution.0));
            let y_width =
                self.frame_size.1 * ((self.input_resolution.1) / (self.display_resolution.1));

            x = (x - frame_origin_x) / x_width;
            y = (y - frame_origin_y) / y_width;
            stroke.push((x, y));
        }
    }
    fn parse_coord(line: &[u8], line_number: usize) -> Result<(f32, f32), ParseError> {
        let (string, encoding, has_replacement) = SHIFT_JIS.decode(line);
        if encoding != SHIFT_JIS || has_replacement {
            return Err(ParseError {
                line_number,
                kind: ErrorKind::Encoding,
            });
        }
        let mut tuple = string.split_whitespace();
        let _code = tuple.next().ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let x = tuple.next().ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let y = tuple.next().ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;

        let x = x.parse::<f32>().map_err(|_e| ParseError {
            line_number,
            kind: ErrorKind::Integer,
        })?;
        let y = y.parse::<f32>().map_err(|_e| ParseError {
            line_number,
            kind: ErrorKind::Integer,
        })?;

        Ok((x, y))
    }
    fn inc_frame_index(&mut self) {
        self.frame_index.0 += 1.0_f32;
        if self.frame_index.0 >= self.frame_count.0 {
            self.frame_index.0 = 0.0_f32;
            self.frame_index.1 += 1.0_f32;
        }
        if self.frame_index.1 >= self.frame_count.1 {
            self.frame_index.1 = 0.0_f32;
        }
    }
    fn parse_meta_string(line: &[u8], line_number: usize) -> Result<String, ParseError> {
        let slice = line.split(|c| *c == b'\'').nth(1).ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let (string, encoding, has_replacement) = SHIFT_JIS.decode(slice);
        if encoding != SHIFT_JIS || has_replacement {
            return Err(ParseError {
                line_number,
                kind: ErrorKind::Encoding,
            });
        }

        Ok(string.to_string())
    }
    fn parse_meta_time(line: &[u8], line_number: usize) -> Result<String, ParseError> {
        let slice = line.split(|c| *c == b':').nth(1).ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let (string, encoding, has_replacement) = SHIFT_JIS.decode(slice);
        if encoding != SHIFT_JIS || has_replacement {
            return Err(ParseError {
                line_number,
                kind: ErrorKind::Encoding,
            });
        }

        Ok(string.trim().to_owned())
    }
    fn parse_meta_int_tuple(line: &[u8], line_number: usize) -> Result<(f32, f32), ParseError> {
        let slice = line.split(|c| *c == b':').nth(1).ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let (string, encoding, has_replacement) = SHIFT_JIS.decode(slice);
        if encoding != SHIFT_JIS || has_replacement {
            return Err(ParseError {
                line_number,
                kind: ErrorKind::Encoding,
            });
        }
        let (x, y) = string.trim().split_once(' ').ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })?;
        let x = x.trim().parse::<f32>().map_err(|_e| ParseError {
            line_number,
            kind: ErrorKind::Integer,
        })?;
        let y = y.trim().parse::<f32>().map_err(|_e| ParseError {
            line_number,
            kind: ErrorKind::Integer,
        })?;

        Ok((x, y))
    }

    fn parse_char(line: &[u8], line_number: usize) -> Result<char, ParseError> {
        let (string, encoding, has_replacement) = SHIFT_JIS.decode(line);
        if encoding != SHIFT_JIS || has_replacement {
            return Err(ParseError {
                line_number,
                kind: ErrorKind::Encoding,
            });
        }
        string.trim().chars().nth(1).ok_or(ParseError {
            line_number,
            kind: ErrorKind::UnexpectedEnd,
        })
    }

    #[inline]
    #[must_use]
    pub fn get_native_language(&self) -> &str {
        &self.native_language
    }
    #[inline]
    #[must_use]
    pub fn get_writing_hand(&self) -> &str {
        &self.writing_hand
    }
    #[inline]
    #[must_use]
    pub fn get_dominant_hand(&self) -> &str {
        &self.dominant_hand
    }
    #[inline]
    #[must_use]
    pub fn get_motivation(&self) -> &str {
        &self.motivation
    }
    #[inline]
    #[must_use]
    pub fn get_birthday(&self) -> &str {
        &self.birthday
    }
    #[inline]
    #[must_use]
    pub fn get_first_date(&self) -> &str {
        &self.first_date
    }
    #[inline]
    #[must_use]
    pub fn get_first_time(&self) -> &str {
        &self.first_time
    }
    #[inline]
    #[must_use]
    pub fn get_last_date(&self) -> &str {
        &self.last_date
    }
    #[inline]
    #[must_use]
    pub fn get_last_time(&self) -> &str {
        &self.last_time
    }
    #[inline]
    #[must_use]
    pub fn get_sex(&self) -> &str {
        &self.sex
    }
    #[inline]
    #[must_use]
    pub fn get_occupation(&self) -> &str {
        &self.occupation
    }
    #[inline]
    #[must_use]
    pub fn get_stroke_data(&self) -> &[(char, CharacterStrokes)] {
        &self.stroke_data
    }
}

#[cfg(test)]
mod tests {
    use crate::HandParser;

    #[test]
    fn native_language() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_native_language(), "ドイツ");
    }
    #[test]
    fn writing_hand() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_writing_hand(), "l");
    }

    #[test]
    fn dominant_hand() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_dominant_hand(), "l");
    }
    #[test]
    fn motivation() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_motivation(), "");
    }
    #[test]
    fn birthday() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_birthday(), "2002 7 12");
    }
    #[test]
    fn first_date() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_first_date(), "2025 12 11");
    }
    #[test]
    fn first_time() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_first_time(), "11 58 5");
    }
    #[test]
    fn last_date() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_last_date(), "2026 1 14");
    }
    #[test]
    fn last_time() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_last_time(), "14 8 30");
    }
    #[test]
    fn sex() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_sex(), "m");
    }
    #[test]
    fn occupation() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.get_occupation(), "学生");
    }
    #[test]
    fn frame_start() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.frame_start, (19.0_f32, 39.0_f32));
    }
    #[test]
    fn frame_step() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.frame_step, (65.0_f32, 103.0_f32));
    }
    #[test]
    fn frame_count() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.frame_count, (9.0_f32, 4.0_f32));
    }
    #[test]
    fn frame_size() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.frame_size, (60.0_f32, 60.0_f32));
    }
    #[test]
    fn display_resolution() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.display_resolution, (640.0_f32, 480.0_f32));
    }
    #[test]
    fn input_resolution() {
        let raw = include_bytes!("../../data/test/HANDS_TEST.TXT");
        let parser = HandParser::new().parse(raw).unwrap();
        assert_eq!(parser.input_resolution, (8300.0_f32, 6240.0_f32));
    }
}
