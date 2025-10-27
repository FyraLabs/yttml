pub use aspasia::timing::Moment;
use aspasia::{SubRipSubtitle, WebVttSubtitle, AssSubtitle};
use serde::{Deserialize, Serialize};
use srv3_ttml::{BodyElement, AnchorPoint, FontStyle, EdgeType};
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
    fn text_no_zwsp(&self) -> String {
        self.text().replace('\u{200B}', "")
    }
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

// this will get the anchorpoint (ap) position from the srv3 and
// convert it to coordinates from ass

pub trait AnchorPointExt {
    fn coordinates(&self) -> (i32, i32);
}
impl AnchorPointExt for AnchorPoint {
    fn coordinates(&self) -> (i32, i32) {
        match self {
            AnchorPoint::TopLeft => (0, 0),
            AnchorPoint::TopCenter => (640, 0),     // 1280/2
            AnchorPoint::TopRight => (1280, 0),     // 1280
            AnchorPoint::MiddleLeft => (0, 360),    // 720/2
            AnchorPoint::Center => (640, 360),      // 1280/2, 720/2
            AnchorPoint::MiddleRight => (1280, 360),// 1280, 720/2
            AnchorPoint::BottomLeft => (0, 720),    // 720
            AnchorPoint::BottomCenter => (640, 720),// 1280/2, 720
            AnchorPoint::BottomRight => (1280, 720),// 1280, 720
        }
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

pub fn to_ass(captions: &srv3_ttml::TimedText) -> std::io::Result<String> {
    let paragraph = &captions.body.elements;
    let mut w = String::new();

    let has_pens = captions.head.as_ref().map_or(false, |head| !head.pen.is_empty());
    // we check if the file has pens and if it doesnt then dont write any formatting
    // like the black bgcolor or something like that (&H000000)
    // this is to make the resulting file nice and small

    let default_style = "YTGlow"; // force YTGlow for non-formatted subs
                                  // YTSC forces YTPlainBox but imo that doesn't look very good
    let marginl = 25;
    let marginr = 25;
    let marginv = 15;

    let layer = 0;
    let name = "";
    let effect = "";

    let playresx = 1280;
    let playresy = 720;
    // not sure about a good way to get these, YTSC hardcodes them
    // so thats what were doing!!!
    // these are the default for ffmpeg vtt > ass conversion, so we use that

    writeln!(&mut w, "[Script Info]").unwrap();

    writeln!(&mut w, "; Script generated by YTTML").unwrap();
    writeln!(&mut w, "; https://github.com/FyraLabs/yttml/").unwrap();

    writeln!(&mut w, "ScriptType: v4.00+").unwrap();
    writeln!(&mut w, "WrapStyle: 0").unwrap();
    writeln!(&mut w, "ScaledBorderAndShadow: yes").unwrap();
    writeln!(&mut w, "PlayResX: {}", playresx).unwrap();
    writeln!(&mut w, "PlayResY: {}", playresy).unwrap();
    writeln!(&mut w, "").unwrap();

    writeln!(&mut w, "[V4+ Styles]").unwrap();
    writeln!(&mut w, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").unwrap();
    writeln!(&mut w, "Style: YTPlain,Roboto,38,&H01FEFEFE,&HFF000000,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,2,25,25,15,1\nStyle: YTPlainBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H00000000,0,0,0,0,100,100,0,0,3,0.01,0,2,25,25,15,1\nStyle: YTGlow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,2,0,2,25,25,15,1\nStyle: YTGlowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTSoftShadow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTSoftShadowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTHardShadow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTHardShadowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTBevel,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTBevelBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H00000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1").unwrap();
    writeln!(&mut w, "").unwrap();

    writeln!(&mut w, "[Events]").unwrap();
     writeln!(&mut w, "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text").unwrap();
    for element in paragraph {
        //println!("{:?}", element);
        match element {
            BodyElement::Paragraph(paragraph) => {
                let position_str = if let Some(wp_id) = paragraph.window_position {
                    if let Some(head) = &captions.head {
                        if let Some(wp) = head.wp.iter().find(|wp| wp.id == wp_id) {
                            if let Some(ap) = &wp.anchor_point {
                                let alignment = match ap {
                                    AnchorPoint::TopLeft => 7,
                                    AnchorPoint::TopCenter => 8,
                                    AnchorPoint::TopRight => 9,
                                    AnchorPoint::MiddleLeft => 4,
                                    AnchorPoint::Center => 5,
                                    AnchorPoint::MiddleRight => 6,
                                    AnchorPoint::BottomLeft => 1,
                                    AnchorPoint::BottomCenter => 2,
                                    AnchorPoint::BottomRight => 3,
                                };

                                // Convert percentage coordinates to pixel coordinates
                                let x = if let Some(ah) = wp.horizontal_offset {
                                    let effective_percent = (ah as f32 * 0.96) + 2.0;
                                    effective_percent * 1280.0 / 100.0
                                } else {
                                    640.0  // Center
                                };

                                let y = if let Some(av) = wp.vertical_offset {
                                    let effective_percent = (av as f32 * 0.96) + 2.0;
                                    effective_percent * 720.0 / 100.0
                                } else {
                                    360.0  // Center
                                };

                                format!("\\an{}\\pos({:.3},{:.3})",
                                    if alignment != 2 { alignment.to_string() } else { String::new() },
                                    x,
                                    y
                                )

                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
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
                        default_style,
                        name,
                        marginl,
                        marginr,
                        marginv,
                        effect,
                        "", // if we dont have pens (the color) dont write the color
                            // there is definitely a cleaner way of doing this but this works
                        paragraph.inner.text_no_zwsp()
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

                            let font_family = pen.and_then(|pen| pen.font_style.as_ref())
                                .map(|fs| match fs {
                                    FontStyle::Default | FontStyle::ProportionalSans => "\\fnRoboto",
                                    FontStyle::MonoSerif => "\\fnCourier New",
                                    FontStyle::ProportionalSerif => "\\fnTimes New Roman",
                                    FontStyle::MonoSans => "\\fnLucida Console",
                                    FontStyle::Casual => "\\fnComic Sans MS",
                                    FontStyle::Cursive => "\\fnMonotype Corsiva",
                                    FontStyle::SmallCaps => "\\fnArial",
                                })
                                .unwrap_or_default();

                            let style = if let Some(pen) = pen {
                                match pen.edge_type {
                                    Some(EdgeType::HardShadow) => "YTHardShadow",
                                    Some(EdgeType::Bevel) => "YTBevel",
                                    Some(EdgeType::Glow) => "YTGlow",
                                    Some(EdgeType::SoftShadow) => "YTSoftShadow",
                                    Some(EdgeType::None) | None => "YTGlow"
                                }
                            } else {
                                "YTGlow"
                            };

                            // process each span with its own font size and text offset
                            let formatted_text = paragraph.inner.iter().map(|elem| {
                                if let BodyElement::Span(span) = elem {
                                    if let Some(span_pen_id) = span.pen {
                                        if let Some(span_pen) = head.pen.iter().find(|p| p.id == span_pen_id) {
                                            // get font size for the correct span, only if different from default
                                            let size_tag = span_pen.font_size.and_then(|size| {
                                                if size == 100 {
                                                    None  // Don't add tag for default size
                                                } else {
                                                    let real_percentage = 100.0 + (size as f64 - 100.0) / 4.0;
                                                    let relative_size = (38.0 * real_percentage / 100.0).round() as i32;
                                                    Some(format!("{{\\fs{}}}", relative_size))
                                                }
                                            }).unwrap_or_default();

                                            // get text offset tag (subscript/superscript)
                                            let offset_tag = span_pen.text_offset.as_ref().map(|offset| {
                                                match offset {
                                                    srv3_ttml::TextOffset::Subscript => "{\\ytsub}",
                                                    srv3_ttml::TextOffset::Superscript | srv3_ttml::TextOffset::SuperscriptAlt => "{\\ytsup}",
                                                }
                                            }).unwrap_or_default();

                                            format!("{}{}{}", size_tag, offset_tag, span.inner.as_ref().map_or(String::new(), |inner| inner.text_no_zwsp()))
                                        } else {
                                            span.inner.as_ref().map_or(String::new(), |inner| inner.text_no_zwsp())
                                        }
                                    } else {
                                        span.inner.as_ref().map_or(String::new(), |inner| inner.text_no_zwsp())
                                    }
                                } else {
                                    elem.text_no_zwsp()
                                }
                            }).collect::<String>();

                            writeln!(
                                &mut w,
                                "Dialogue: {},{},{},{},{},{},{},{},{},{{{}\\3c{}\\1c{}{}}}{}",
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
                                position_str,
                                bg_color,
                                fg_color,
                                font_family,
                                formatted_text
                            ).unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    //println!("{}", w);
    Ok(w)
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
