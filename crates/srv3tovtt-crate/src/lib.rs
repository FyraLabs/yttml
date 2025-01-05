use std::fmt::Write;

pub use aspasia::timing::Moment;
use serde::{Deserialize, Serialize};
use srv3_ttml::BodyElement;

#[derive(Debug, Serialize, Deserialize)]
struct Paragraph {
    inner: Vec<BodyElement>,
    timestamp: u32,
    duration: u32,
    window_position: Option<u32>,
    window_style: Option<u32>,
}

trait ElementExt {
    fn text(&self) -> String;
}

impl ElementExt for String {
    fn text(&self) -> String {
        self.clone()
    }
}
impl ElementExt for BodyElement {
    fn text(&self) -> String {
        match self {
            Self::Text(t) => t.text(),
            _ => String::new(),
        }
    }
}
impl ElementExt for Vec<BodyElement> {
    fn text(&self) -> String {
        self.iter()
            .fold(String::new(), |acc, elem| acc + &elem.text() + "\n")
            .trim()
            .to_string()
    }
}

pub fn to_vtt(captions: &srv3_ttml::TimedText) -> std::io::Result<String> {
    let paragraph = &captions.body.elements;
    let mut w = String::new();
    writeln!(&mut w, "WEBVTT").unwrap();
    writeln!(&mut w, "Kind: captions").unwrap();
    writeln!(&mut w, "Language: en").unwrap();
    for element in paragraph {
        writeln!(&mut w, "").unwrap();
        match element {
            BodyElement::Text(_text) => {}
            BodyElement::Paragraph(paragraph) => writeln!(
                &mut w,
                "{} --> {}\n{}",
                aspasia::timing::Moment::as_vtt_timestamp(&aspasia::timing::Moment::from(
                    paragraph.timestamp as i64
                )),
                aspasia::timing::Moment::as_vtt_timestamp(&aspasia::timing::Moment::from(
                    paragraph.timestamp as i64 + paragraph.duration as i64
                )),
                paragraph.inner.text()
            )
            .unwrap(),
            BodyElement::Span(_span) => {}
            BodyElement::Br(_br) => {}
            BodyElement::Div(_div) => {}
            BodyElement::Window(_window) => {}
        }
    }
    Ok(w)
}
