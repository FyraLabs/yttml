pub use aspasia::timing::Moment;
use aspasia::{SubRipSubtitle, WebVttSubtitle, AssSubtitle};
use serde::{Deserialize, Serialize};
use srv3_ttml::BodyElement;
use std::fmt::Write;
use std::str::FromStr;
use hex_color::*;

#[derive(Debug, Serialize, Deserialize)]
struct Paragraph {
    inner: Vec<BodyElement>,
    timestamp: u32,
    duration: u32,
    window_position: Option<u32>,
    window_style: Option<u32>,
}

fn hex_to_ass_color(hex: &HexColor) -> String {
    let hex_str = format!("{:?}", hex);
    if hex_str.contains("r:") && hex_str.contains("g:") && hex_str.contains("b:") {
        let r = hex_str
            .split("r:")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<u8>().ok())
            .unwrap_or(0);

        let g = hex_str
            .split("g:")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<u8>().ok())
            .unwrap_or(0);

        let b = hex_str
            .split("b:")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<u8>().ok())
            .unwrap_or(0);

        // for some reason ASS uses BGR, i dont know who came up with this
        // have them fired immediately
        // also theres a H there
        format!("&H{:02X}{:02X}{:02X}", b, g, r)
    } else {
        String::from("&H000000")
    }
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
            Self::Paragraph(p) => p.text(),
            Self::Span(s) => s.inner.as_ref().map_or(String::new(), |inner| inner.text()),
            Self::Br(_) => "\n".to_string(),
            Self::Div(elements) => elements.text(),
            Self::Window(_) => String::new(),
        }
    }
}

impl ElementExt for Paragraph {
    fn text(&self) -> String {
        self.inner
            .iter()
            .map(|elem| elem.text())
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    }
}

impl ElementExt for Vec<BodyElement> {
    fn text(&self) -> String {
        self.iter()
            .map(|elem| elem.text())
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    }
}

pub fn to_vtt(captions: &srv3_ttml::TimedText) -> std::io::Result<WebVttSubtitle> {
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
    Ok(aspasia::WebVttSubtitle::from_str(&w).unwrap())
}

pub fn to_ass(captions: &srv3_ttml::TimedText) -> std::io::Result<AssSubtitle> {
    let paragraph = &captions.body.elements;
    let mut w = String::new();

    let has_pens = captions.head.as_ref().map_or(false, |head| !head.pen.is_empty());
    // we check if the file has pens and if it doesnt then dont write any formatting
    // like the black bgcolor or something like that (&H000000)
    // this is to make the resulting file nice and small
    let style = "Default";
    let font = "Roboto";
    let fontsize = "16";
    let primarycolour = "&Hffffff";
    let secondarycolour = "&Hffffff";
    let outlinecolour = "&H0";
    let backcolour = "&H0";
    let bold = 0;
    let italic = 0;
    let underline = 0;
    let strikeout = 0;
    let scalex = 100;
    let scaley = 100;
    let spacing = 0;
    let angle = 0;
    let borderstyle = 1;
    let outline = 1;
    let shadow = 0;
    let alignment = 2;
    let marginl = 10;
    let marginr = 10;
    let marginv = 10;
    let encoding = 1;

    let layer = 0;
    let name = "";
    let effect = "";

    let playresx = 384;
    let playresy = 288;
    // not sure about a good way to get these, YTSC hardcodes them
    // so thats what were doing!!!
    // these are the default for ffmpeg vtt > ass conversion, so we use that

    writeln!(&mut w, "[Script Info]").unwrap();
    writeln!(&mut w, "ScriptType: v4.00+").unwrap();
    writeln!(&mut w, "PlayResX: {}", playresx).unwrap();
    writeln!(&mut w, "PlayResY: {}", playresy).unwrap();
    writeln!(&mut w, "").unwrap();
    writeln!(&mut w, "[V4+ Styles]").unwrap();
    writeln!(&mut w, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").unwrap();
    writeln!(&mut w, "Style: {},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        style, font, fontsize, primarycolour, secondarycolour, outlinecolour, backcolour,
        bold, italic, underline, strikeout, scalex, scaley, spacing, angle,
        borderstyle, outline, shadow, alignment, marginl, marginr, marginv, encoding
    ).unwrap();
    writeln!(&mut w, "").unwrap();
    writeln!(&mut w, "[Events]").unwrap();
    writeln!(&mut w, "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text").unwrap();

    for element in paragraph {
        match element {
            BodyElement::Paragraph(paragraph) => {
                if !has_pens {
                    writeln!(
                        &mut w,
                        "Dialogue: {},{},{},{},{},{},{},{},{},{}{}",
                        layer,
                        aspasia::timing::Moment::as_substation_timestamp(
                            &aspasia::timing::Moment::from(paragraph.timestamp as i64)
                        ),
                        aspasia::timing::Moment::as_substation_timestamp(
                            &aspasia::timing::Moment::from(paragraph.timestamp as i64 + paragraph.duration as i64)
                        ),
                        style,
                        name,
                        marginl,
                        marginr,
                        marginv,
                        effect,
                        "", // if we dont have pens (the color) dont write the color
                            // there is definitely a cleaner way of doing this but this works
                        paragraph.inner.text()
                    ).unwrap();
                } else {
                    if let Some(head) = &captions.head {
                        let text_span_pen_id = paragraph.inner.iter()
                            .filter_map(|elem| {
                                if let BodyElement::Span(span) = elem {
                                    let has_text = span.inner.as_ref()
                                        .map(|inner| {
                                            let text = inner.text();
                                            let binding = text.replace("\u{200b}", "");
                                            let cleaned_text = binding.trim();
                                            !cleaned_text.is_empty()
                                        })
                                        .unwrap_or(false);

                                    if has_text {
                                        span.pen
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .next();

                        if let Some(pen_id) = text_span_pen_id {
                            let pen = head.pen.iter().find(|pen| pen.id == pen_id);

                            let bg_color = pen.and_then(|pen| pen.background_color.as_ref())
                               .map(|color| hex_to_ass_color(color))
                               .unwrap_or_else(|| String::from("&H000000"));

                            let fg_color = pen.and_then(|pen| pen.foreground_color.as_ref())
                               .map(|color| hex_to_ass_color(color))
                               .unwrap_or_else(|| String::from("&HFFFFFF"));

                            writeln!(
                                &mut w,
                                "Dialogue: {},{},{},{},{},{},{},{},{},{{\\3c{}}}{{\\1c{}}}{}",
                                layer,
                                aspasia::timing::Moment::as_substation_timestamp(
                                    &aspasia::timing::Moment::from(paragraph.timestamp as i64)
                                ),
                                aspasia::timing::Moment::as_substation_timestamp(
                                    &aspasia::timing::Moment::from(paragraph.timestamp as i64 + paragraph.duration as i64)
                                ),
                                style,
                                name,
                                marginl,
                                marginr,
                                marginv,
                                effect,
                                bg_color,
                                fg_color,
                                paragraph.inner.text()
                            ).unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    //println!("{}", w);
    Ok(aspasia::AssSubtitle::from_str(&w).unwrap())
}

pub fn to_srt(
    captions: &srv3_ttml::TimedText,
) -> Result<SubRipSubtitle, Box<dyn std::error::Error>> {
    let vtt = to_vtt(captions)?;
    let srt = aspasia::SubRipSubtitle::from(&vtt);
    Ok(srt)
} // Kind of a hacky solution, but because SubRip doesn't offer anything extra over WebVTT, it is plausible

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // test the Hex to ASS color converter
    // cargo test test_hex_to_ass_color
    fn test_hex_to_ass_color() {
        let hex = HexColor::parse("#FF0000").unwrap();
        let result = hex_to_ass_color(&hex);
        assert_eq!(result, "&H0000FF");

        let hex = HexColor::parse("#00FF00").unwrap();
        let result = hex_to_ass_color(&hex);
        assert_eq!(result, "&H00FF00");

        let hex = HexColor::parse("#0000FF").unwrap();
        let result = hex_to_ass_color(&hex);
        assert_eq!(result, "&HFF0000");
    }
}
