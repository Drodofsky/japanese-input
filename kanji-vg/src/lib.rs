use japanese_input::{kanji_node::KanjiNode, stroke_point::bez_to_point};
use kurbo::{Affine, BezPath};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::Utf8Error,
};
use thiserror::Error;
use wana_kana::IsJapaneseChar;

use quick_xml::{
    Reader,
    events::{BytesStart, Event, attributes::AttrError},
};

pub fn flatten_single_stroke_groups(kanji_node: &mut KanjiNode) {
    if let KanjiNode::Group { children, .. } = kanji_node {
        // First, recurse into each child so deeper levels are flattened first.
        for child in children.iter_mut() {
            flatten_single_stroke_groups(child);
        }

        for child in children.iter_mut() {
            if let KanjiNode::Group {
                children: inner, ..
            } = child
                && inner.len() == 1
                && let KanjiNode::Stroke { .. } = inner[0]
            {
                // Take the inner stroke out and replace the group with it.
                let stroke = inner.remove(0);
                *child = stroke;
            }
        }
    }
}

pub fn parse_xml(path: impl AsRef<Path>) -> Result<GlyphIter<BufReader<File>>, ParseError> {
    let file = File::open(path)?;
    let buf = BufReader::new(file);
    let mut reader = Reader::from_reader(buf);
    reader.config_mut().trim_text(true);

    Ok(GlyphIter {
        reader,
        buf: Vec::new(),
        inner_buf: Vec::new(),
        finished: false,
    })
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("XML error: {0}")]
    XmlAttr(#[from] AttrError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] Utf8Error),
    #[error("Missing Element")]
    MissingElement,
    #[error("Missing Path")]
    MissingPath,
    #[error("Unexpected XML")]
    UnexpectedXml,
    #[error("svg path")]
    Path(#[from] kurbo::SvgParseError),
}

pub struct GlyphIter<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
    inner_buf: Vec<u8>,
    finished: bool,
}

impl<R: BufRead> Iterator for GlyphIter<R> {
    type Item = Result<(char, KanjiNode), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            match self.reader.read_event_into(&mut self.buf) {
                Ok(Event::Start(e)) if e.name().as_ref() == b"kanji" => {
                    let result = parse_one_kanji(&mut self.reader, &mut self.inner_buf);
                    self.buf.clear();
                    return Some(result);
                }
                Ok(Event::Eof) => {
                    self.finished = true;
                    return None;
                }
                Ok(Event::Comment(_)) | Ok(Event::Decl(_)) => {
                    self.buf.clear();
                }
                Ok(Event::Start(e)) if e.name().as_ref() == b"kanjivg" => {
                    self.buf.clear();
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"kanjivg" => {
                    self.buf.clear();
                }
                Ok(d) => {
                    println!("{d:#?}");
                    return Some(Err(ParseError::UnexpectedXml));
                }
                Err(err) => {
                    self.finished = true;
                    return Some(Err(ParseError::Xml(err)));
                }
            }
        }
    }
}

fn parse_one_kanji<R: BufRead>(
    reader: &mut Reader<R>,
    buf: &mut Vec<u8>,
) -> Result<(char, KanjiNode), ParseError> {
    let mut stroke_index = 0;
    let element = loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"g" => {
                break get_element(&e)?.ok_or(ParseError::MissingElement)?;
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"kanji" => {
                return Err(ParseError::MissingElement);
            }
            Ok(_) => continue,
            Err(e) => return Err(ParseError::Xml(e)),
        }
    };

    let children = parse_children(reader, buf, &mut stroke_index, b"kanji")?;
    let mut root = KanjiNode::Group { element, children };
    flatten_single_stroke_groups(&mut root);
    Ok((element, root))
}

fn parse_children<R: BufRead>(
    reader: &mut Reader<R>,
    buf: &mut Vec<u8>,
    stroke_index: &mut usize,
    end_tag: &[u8], // ← b"g" or b"kanji"
) -> Result<Vec<KanjiNode>, ParseError> {
    let mut children = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"g" => {
                let child_element = get_element(&e)?;
                let inner = parse_children(reader, buf, stroke_index, b"g")?;
                match child_element {
                    Some(element) => children.push(KanjiNode::Group {
                        element,
                        children: inner,
                    }),
                    None => children.extend(inner),
                }
            }
            Ok(Event::Empty(e)) if e.name().as_ref() == b"path" => {
                let d = get_attr(&e, b"d")?.ok_or(ParseError::MissingPath)?;
                let path = parse_svg_path(&d)?;
                children.push(KanjiNode::Stroke {
                    index: (*stroke_index).try_into().unwrap_or(u8::MAX),
                    path: bez_to_point(&path),
                });
                *stroke_index += 1;
            }
            Ok(Event::End(e)) if e.name().as_ref() == end_tag => break,
            Ok(_) => continue,
            Err(e) => return Err(ParseError::Xml(e)),
        }
    }
    Ok(children)
}

fn get_element(e: &BytesStart) -> Result<Option<char>, ParseError> {
    let mut phon = None;
    for attr in e.attributes() {
        let attr = attr?;
        match attr.key.as_ref() {
            b"kvg:element" => {
                if let Some(c) = std::str::from_utf8(&attr.value)?.chars().next() {
                    return Ok(Some(c));
                }
            }
            b"kvg:phon" => {
                // only accept a bare single kanji; rejects "豸+艮", "矦V", "3+6+1"
                let mut cs = std::str::from_utf8(&attr.value)?.chars();
                phon = match (cs.next(), cs.next()) {
                    (Some(c), None) if c.is_kanji() => Some(c),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    Ok(phon)
}

fn get_attr(e: &BytesStart, name: &[u8]) -> Result<Option<String>, ParseError> {
    for attr in e.attributes() {
        let attr = attr?;
        if attr.key.as_ref() == name {
            return Ok(Some(std::str::from_utf8(&attr.value)?.to_string()));
        }
    }
    Ok(None)
}

fn parse_svg_path(d: &str) -> Result<BezPath, ParseError> {
    let mut path = BezPath::from_svg(d)?;
    let scale = 1.0 / 109.0;
    path.apply_affine(Affine::scale(scale));
    Ok(path)
}
#[cfg(test)]
mod tests {
    use japanese_input::KanjiMap;
    use wana_kana::IsJapaneseChar;

    use crate::parse_xml;
    #[test]
    fn generate_reference_data() {
        let path = "../data/raw/kanjivg.xml";
        let kanji_entries: KanjiMap = parse_xml(path)
            .unwrap()
            .into_iter()
            .map(|k| k.unwrap())
            .filter(|k| k.0.is_japanese())
            .collect();
        let data = postcard::to_allocvec(&kanji_entries).expect("Failed to serialize");
        std::fs::create_dir_all("../data/generated").unwrap();

        std::fs::write("../data/generated/reference_data.bin", &data).expect("Failed to write");
        assert_eq!(kanji_entries.len(), 6604);
        println!(
            "Wrote {} entries ({} bytes)",
            kanji_entries.len(),
            data.len()
        );
    }

    #[test]
    fn generate_kanji() {
        let path = "../data/raw/kanjivg.xml";
        let kanji_entries: KanjiMap = parse_xml(path)
            .unwrap()
            .into_iter()
            .map(|k| k.unwrap())
            .filter(|k| k.0.is_kanji())
            .collect();
        let data = postcard::to_allocvec(&kanji_entries).expect("Failed to serialize");
        std::fs::create_dir_all("../data/generated").unwrap();

        std::fs::write("../data/generated/kanji.bin", &data).expect("Failed to write");
        assert_eq!(kanji_entries.len(), 6412);
        println!(
            "Wrote {} entries ({} bytes)",
            kanji_entries.len(),
            data.len()
        );
    }
    #[test]
    fn generate_hiragana() {
        let path = "../data/raw/kanjivg.xml";
        let kanji_entries: KanjiMap = parse_xml(path)
            .unwrap()
            .into_iter()
            .map(|k| k.unwrap())
            .filter(|k| k.0.is_hiragana() & !"ぁぃぅぇぉっゃゅょゎゕゖゐゑゔー".contains(k.0))
            .collect();
        let data = postcard::to_allocvec(&kanji_entries).expect("Failed to serialize");
        std::fs::create_dir_all("../data/generated").unwrap();
        std::fs::write("../data/generated/hiragana.bin", &data).expect("Failed to write");
        assert_eq!(kanji_entries.len(), 71);
        //println!("{:?}",kanji_entries.iter().map(|s|s.0).collect::<Vec<_>>());
        println!(
            "Wrote {} entries ({} bytes)",
            kanji_entries.len(),
            data.len()
        );
    }
    #[test]
    fn generate_katakana() {
        let path = "../data/raw/kanjivg.xml";
        let kanji_entries: KanjiMap = parse_xml(path)
            .unwrap()
            .into_iter()
            .map(|k| k.unwrap())
            .filter(|k| k.0.is_katakana() & !"ァィゥェォッャュョヮヵヶ・ヷ ヸ ヹ ヺ".contains(k.0))
            .collect();
        let data = postcard::to_allocvec(&kanji_entries).expect("Failed to serialize");
        std::fs::create_dir_all("../data/generated").unwrap();

        std::fs::write("../data/generated/katakana.bin", &data).expect("Failed to write");
        assert_eq!(kanji_entries.len(), 75);
        // println!("{:?}",kanji_entries.iter().map(|s|s.0).collect::<Vec<_>>());
        println!(
            "Wrote {} entries ({} bytes)",
            kanji_entries.len(),
            data.len()
        );
    }
}
