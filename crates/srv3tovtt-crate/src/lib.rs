pub use aspasia::timing::Moment;
use aspasia::{SubRipSubtitle, WebVttSubtitle};
use hex_color::*;
use srv3_ttml::{
    AnchorPoint, BodyElement, EdgeType, FontStyle, Head, Paragraph as TimedTextParagraph, Pen,
    TextOffset,
};
use std::fmt::Write;
use std::str::FromStr;

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

        // ASS uses BGR ordering
        format!("&H{:02X}{:02X}{:02X}", b, g, r)
    } else {
        String::from("&H000000")
    }
}

#[derive(Clone, Debug)]
struct StyleDefaults {
    font_name: String,
    font_size: f64,
    primary_color: String,
    primary_alpha: u8,
    outline_color: String,
    outline_alpha: u8,
    back_color: String,
    back_alpha: u8,
    bold: bool,
    italic: bool,
    underline: bool,
}

fn style_defaults(style_name: &str) -> StyleDefaults {
    let mut defaults = StyleDefaults {
        font_name: "Roboto".to_string(),
        font_size: 38.0,
        primary_color: "&HFEFEFE".to_string(),
        primary_alpha: 0x01,
        outline_color: "&H000000".to_string(),
        outline_alpha: 0x01,
        back_color: "&H000000".to_string(),
        back_alpha: 0x01,
        bold: false,
        italic: false,
        underline: false,
    };

    match style_name {
        "YTPlain" => {
            defaults.outline_alpha = 0x00;
            defaults.back_alpha = 0x00;
        }
        "YTPlainBox" => {
            defaults.outline_alpha = 0x01;
            defaults.back_alpha = 0x00;
        }
        "YTGlow" | "YTGlowBox" | "YTSoftShadow" | "YTSoftShadowBox" | "YTHardShadow"
        | "YTHardShadowBox" | "YTBevel" => {
            defaults.outline_alpha = 0x01;
            defaults.back_alpha = 0x01;
        }
        "YTBevelBox" => {
            defaults.outline_alpha = 0x01;
            defaults.back_alpha = 0x00;
        }
        _ => {}
    }

    defaults
}

trait ElementExt {
    fn text(&self) -> String;
    fn text_clean_ass(&self) -> String;
}

impl ElementExt for String {
    fn text(&self) -> String {
        self.clone()
    }

    fn text_clean_ass(&self) -> String {
        self.replace('\u{200B}', "").replace('\n', "\\N")
    }
}

impl ElementExt for BodyElement {
    fn text(&self) -> String {
        match self {
            BodyElement::Text(text) => text.text(),
            BodyElement::Paragraph(paragraph) => paragraph.inner.text(),
            BodyElement::Span(span) => span
                .inner
                .as_ref()
                .map(|inner| inner.text())
                .unwrap_or_default(),
            BodyElement::Br(_) => "\n".to_string(),
            BodyElement::Div(elements) => elements.text(),
            BodyElement::Window(_) => String::new(),
        }
    }

    fn text_clean_ass(&self) -> String {
        match self {
            BodyElement::Text(text) => text.text_clean_ass(),
            BodyElement::Paragraph(paragraph) => paragraph.inner.text_clean_ass(),
            BodyElement::Span(span) => span
                .inner
                .as_ref()
                .map(|inner| inner.text_clean_ass())
                .unwrap_or_default(),
            BodyElement::Br(_) => "\\N".to_string(),
            BodyElement::Div(elements) => elements.text_clean_ass(),
            BodyElement::Window(_) => String::new(),
        }
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

    fn text_clean_ass(&self) -> String {
        self.iter()
            .map(|elem| elem.text_clean_ass())
            .collect::<Vec<_>>()
            .join("")
    }
}

impl ElementExt for TimedTextParagraph {
    fn text(&self) -> String {
        self.inner.text()
    }

    fn text_clean_ass(&self) -> String {
        self.inner.text_clean_ass()
    }
}

#[derive(Debug)]
struct FormattingState {
    font_name: String,
    font_size: f64,
    primary_color: String,
    primary_alpha: u8,
    outline_color: String,
    outline_alpha: u8,
    back_color: String,
    back_alpha: u8,
    bold: bool,
    italic: bool,
    underline: bool,
    text_offset: Option<u8>,
}

impl Clone for FormattingState {
    fn clone(&self) -> Self {
        Self {
            font_name: self.font_name.clone(),
            font_size: self.font_size,
            primary_color: self.primary_color.clone(),
            primary_alpha: self.primary_alpha,
            outline_color: self.outline_color.clone(),
            outline_alpha: self.outline_alpha,
            back_color: self.back_color.clone(),
            back_alpha: self.back_alpha,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            text_offset: self.text_offset,
        }
    }
}

impl FormattingState {
    fn from_defaults(defaults: &StyleDefaults) -> Self {
        Self {
            font_name: defaults.font_name.clone(),
            font_size: defaults.font_size,
            primary_color: defaults.primary_color.clone(),
            primary_alpha: defaults.primary_alpha,
            outline_color: defaults.outline_color.clone(),
            outline_alpha: defaults.outline_alpha,
            back_color: defaults.back_color.clone(),
            back_alpha: defaults.back_alpha,
            bold: defaults.bold,
            italic: defaults.italic,
            underline: defaults.underline,
            text_offset: None,
        }
    }

    fn apply_pen(&mut self, pen: &Pen, defaults: &StyleDefaults) {
        if let Some(font_style) = pen.font_style.as_ref() {
            self.font_name = font_name_for_style(font_style).to_string();
        }

        if let Some(size) = pen.font_size {
            let real_percentage = 100.0 + (size as f64 - 100.0) / 4.0;
            self.font_size = defaults.font_size * real_percentage / 100.0;
        }

        if let Some(value) = pen.bold {
            self.bold = value;
        }

        if let Some(value) = pen.italic {
            self.italic = value;
        }

        if let Some(value) = pen.underline {
            self.underline = value;
        }

        if let Some(color) = pen.foreground_color.as_ref() {
            self.primary_color = hex_to_ass_color(color);
        }

        if let Some(opacity) = pen.foreground_opacity {
            let clamped = (opacity.min(255)) as u8;
            self.primary_alpha = 255u8.saturating_sub(clamped);
        }

        if let Some(color) = pen.edge_color.as_ref() {
            self.outline_color = hex_to_ass_color(color);
        }

        if let Some(opacity) = pen.background_opacity {
            // YouTube encodes box/glow opacity in the outline channel; match YTSubConverter.
            self.outline_alpha = 255u8.saturating_sub(opacity);
        }

        if let Some(color) = pen.background_color.as_ref() {
            self.back_color = hex_to_ass_color(color);
        }

        if let Some(offset) = pen.text_offset.as_ref() {
            self.text_offset = Some(match offset {
                TextOffset::Subscript => 0,
                TextOffset::Superscript | TextOffset::SuperscriptAlt => 1,
            });
        } else {
            self.text_offset = None;
        }
    }
}

fn font_name_for_style(style: &FontStyle) -> &'static str {
    match style {
        FontStyle::Default | FontStyle::ProportionalSans => "Roboto",
        FontStyle::MonoSerif => "Courier New",
        FontStyle::ProportionalSerif => "Times New Roman",
        FontStyle::MonoSans => "Lucida Console",
        FontStyle::Casual => "Comic Sans MS",
        FontStyle::Cursive => "Monotype Corsiva",
        FontStyle::SmallCaps => "Arial",
    }
}

fn choose_style_for_pen(pen: Option<&Pen>) -> &'static str {
    let default_style = "YTGlow";

    match pen {
        Some(pen) => {
            let has_background = pen.background_opacity.unwrap_or(0) > 0;
            match pen.edge_type {
                Some(EdgeType::HardShadow) => {
                    if has_background {
                        "YTHardShadowBox"
                    } else {
                        "YTHardShadow"
                    }
                }
                Some(EdgeType::Bevel) => {
                    if has_background {
                        "YTBevelBox"
                    } else {
                        "YTBevel"
                    }
                }
                Some(EdgeType::Glow) => {
                    if has_background {
                        "YTGlowBox"
                    } else {
                        "YTGlow"
                    }
                }
                Some(EdgeType::SoftShadow) => {
                    if has_background {
                        "YTSoftShadowBox"
                    } else {
                        "YTSoftShadow"
                    }
                }
                Some(EdgeType::None) | None => {
                    if has_background {
                        "YTPlainBox"
                    } else {
                        "YTPlain"
                    }
                }
            }
        }
        None => default_style,
    }
}

fn format_ass_float(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut s = format!("{:.3}", rounded);
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn floats_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.0005
}

fn trim_ass_edge_whitespace(text: String) -> String {
    text.trim_matches([' ', '\u{200B}']).to_string()
}

fn transition_tags(from: &FormattingState, to: &FormattingState) -> Vec<String> {
    let mut tags = Vec::new();

    if from.font_name != to.font_name {
        tags.push(format!("\\fn{}", to.font_name));
    }

    if !floats_equal(from.font_size, to.font_size) {
        tags.push(format!("\\fs{}", format_ass_float(to.font_size)));
    }

    if from.bold != to.bold {
        tags.push(format!("\\b{}", if to.bold { 1 } else { 0 }));
    }

    if from.italic != to.italic {
        tags.push(format!("\\i{}", if to.italic { 1 } else { 0 }));
    }

    if from.underline != to.underline {
        tags.push(format!("\\u{}", if to.underline { 1 } else { 0 }));
    }

    if from.primary_color != to.primary_color {
        tags.push(format!("\\c{}&", to.primary_color));
    }

    if from.primary_alpha != to.primary_alpha {
        tags.push(format!("\\1a&H{:02X}&", to.primary_alpha));
    }

    if from.outline_color != to.outline_color {
        tags.push(format!("\\3c{}&", to.outline_color));
    }

    if from.outline_alpha != to.outline_alpha {
        tags.push(format!("\\3a&H{:02X}&", to.outline_alpha));
    }

    if from.back_color != to.back_color {
        tags.push(format!("\\4c{}&", to.back_color));
    }

    if from.back_alpha != to.back_alpha {
        tags.push(format!("\\4a&H{:02X}&", to.back_alpha));
    }

    if from.text_offset != to.text_offset {
        let tag = match to.text_offset {
            Some(0) => "\\ytsub",
            Some(1) | Some(2) => "\\ytsup",
            None => "\\ytsur",
            _ => "\\ytsur",
        };
        tags.push(tag.to_string());
    }

    tags
}

fn find_pen(head: &Head, id: u32) -> Option<&Pen> {
    head.pen.iter().find(|pen| pen.id == id)
}

fn first_text_pen<'a>(elements: &'a [BodyElement], head: &'a Head) -> Option<&'a Pen> {
    for element in elements {
        match element {
            BodyElement::Span(span) => {
                if let Some(inner) = &span.inner {
                    let text = inner.text_clean_ass();
                    if !text.trim().is_empty() {
                        if let Some(id) = span.pen {
                            if let Some(pen) = find_pen(head, id) {
                                return Some(pen);
                            }
                        }
                    }

                    if let Some(pen) = first_text_pen(inner, head) {
                        return Some(pen);
                    }
                }
            }
            BodyElement::Div(children) => {
                if let Some(pen) = first_text_pen(children, head) {
                    return Some(pen);
                }
            }
            BodyElement::Paragraph(paragraph) => {
                if let Some(pen) = first_text_pen(&paragraph.inner, head) {
                    return Some(pen);
                }
            }
            _ => {}
        }
    }
    None
}

fn render_body_elements(
    elements: &[BodyElement],
    head: &Head,
    defaults: &StyleDefaults,
    current_state: &mut FormattingState,
) -> String {
    let mut output = String::new();

    for element in elements {
        match element {
            BodyElement::Span(span) => {
                let mut target_state = current_state.clone();
                if let Some(pen) = span.pen.and_then(|id| find_pen(head, id)) {
                    target_state.apply_pen(pen, defaults);
                }

                let mut inner_state = target_state.clone();
                let inner_output = span
                    .inner
                    .as_ref()
                    .map(|inner| render_body_elements(inner, head, defaults, &mut inner_state))
                    .unwrap_or_default();

                if inner_output.is_empty() {
                    continue;
                }

                let tags = transition_tags(current_state, &target_state);
                if !tags.is_empty() {
                    output.push_str(&format!("{{{}}}", tags.join("")));
                }

                output.push_str(&inner_output);
                *current_state = inner_state;
            }
            BodyElement::Text(text) => {
                output.push_str(&text.text_clean_ass());
            }
            BodyElement::Br(_) => {
                output.push_str("\\N");
            }
            BodyElement::Div(children) => {
                output.push_str(&render_body_elements(
                    children,
                    head,
                    defaults,
                    current_state,
                ));
            }
            BodyElement::Paragraph(paragraph) => {
                output.push_str(&render_body_elements(
                    &paragraph.inner,
                    head,
                    defaults,
                    current_state,
                ));
            }
            BodyElement::Window(_) => {}
        }
    }

    output
}

fn format_coord(value: f32) -> String {
    if (value - value.floor()).abs() < f32::EPSILON {
        format!("{}", value as i32)
    } else {
        let rounded = (value * 1000.0).round() / 1000.0;
        let mut s = format!("{:.3}", rounded);
        while s.contains('.') && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }
}

fn position_override(paragraph: &TimedTextParagraph, head: Option<&Head>) -> Option<String> {
    let wp_id = paragraph.window_position?;
    let head = head?;
    let window_position = head.wp.iter().find(|wp| wp.id == wp_id)?;
    let anchor_point = window_position.anchor_point.as_ref()?;

    let alignment = match *anchor_point {
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

    let x = window_position
        .horizontal_offset
        .map(|ah| ((ah as f32) * 0.96 + 2.0) * 1280.0 / 100.0)
        .unwrap_or(640.0);

    let y = window_position
        .vertical_offset
        .map(|av| ((av as f32) * 0.96 + 2.0) * 720.0 / 100.0)
        .unwrap_or(360.0);

    let mut tag = String::new();
    if alignment != 2 {
        tag.push_str(&format!("\\an{}", alignment));
    }
    tag.push_str(&format!("\\pos({},{})", format_coord(x), format_coord(y)));

    Some(tag)
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
            AnchorPoint::TopCenter => (640, 0),      // 1280/2
            AnchorPoint::TopRight => (1280, 0),      // 1280
            AnchorPoint::MiddleLeft => (0, 360),     // 720/2
            AnchorPoint::Center => (640, 360),       // 1280/2, 720/2
            AnchorPoint::MiddleRight => (1280, 360), // 1280, 720/2
            AnchorPoint::BottomLeft => (0, 720),     // 720
            AnchorPoint::BottomCenter => (640, 720), // 1280/2, 720
            AnchorPoint::BottomRight => (1280, 720), // 1280, 720
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
        writeln!(&mut w).unwrap();
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
    const DEFAULT_STYLE: &str = "YTGlow";
    const MARGIN_L: i32 = 0;
    const MARGIN_R: i32 = 0;
    const MARGIN_V: i32 = 0;
    const LAYER: i32 = 0;
    const PLAY_RES_X: i32 = 1280;
    const PLAY_RES_Y: i32 = 720;

    let mut w = String::new();
    let head = captions.head.as_ref();
    let has_pens = head.is_some_and(|h| !h.pen.is_empty());

    writeln!(&mut w, "[Script Info]").unwrap();
    writeln!(&mut w, "; Script generated by YTTML").unwrap();
    writeln!(&mut w, "; https://github.com/FyraLabs/yttml/").unwrap();
    writeln!(&mut w, "ScriptType: v4.00+").unwrap();
    writeln!(&mut w, "WrapStyle: 0").unwrap();
    writeln!(&mut w, "ScaledBorderAndShadow: yes").unwrap();
    writeln!(&mut w, "PlayResX: {}", PLAY_RES_X).unwrap();
    writeln!(&mut w, "PlayResY: {}", PLAY_RES_Y).unwrap();
    w.push('\n');

    writeln!(&mut w, "[V4+ Styles]").unwrap();
    writeln!(&mut w, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").unwrap();
    writeln!(&mut w, "Style: YTPlain,Roboto,38,&H01FEFEFE,&HFF000000,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,2,25,25,15,1\nStyle: YTPlainBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H00000000,0,0,0,0,100,100,0,0,3,0.01,0,2,25,25,15,1\nStyle: YTGlow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,2,0,2,25,25,15,1\nStyle: YTGlowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTSoftShadow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTSoftShadowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTHardShadow,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTHardShadowBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1\nStyle: YTBevel,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H01000000,0,0,0,0,100,100,0,0,1,0,4,2,25,25,15,1\nStyle: YTBevelBox,Roboto,38,&H01FEFEFE,&HFF000000,&H01000000,&H00000000,0,0,0,0,100,100,0,0,3,0.01,4,2,25,25,15,1").unwrap();
    w.push('\n');

    writeln!(&mut w, "[Events]").unwrap();
    writeln!(
        &mut w,
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"
    )
    .unwrap();

    for element in &captions.body.elements {
        if let BodyElement::Paragraph(paragraph) = element {
            let start = Moment::from(paragraph.timestamp as i64);
            let end = Moment::from((paragraph.timestamp + paragraph.duration) as i64);
            let start_ts = Moment::as_substation_timestamp(&start);
            let end_ts = Moment::as_substation_timestamp(&end);

            let mut style_name: &'static str = DEFAULT_STYLE;
            let text: String;

            if has_pens {
                if let Some(head) = head {
                    let base_pen = first_text_pen(&paragraph.inner, head);
                    style_name = choose_style_for_pen(base_pen);
                    let defaults = style_defaults(style_name);
                    let default_state = FormattingState::from_defaults(&defaults);
                    let mut line_state = default_state.clone();
                    if let Some(pen) = base_pen {
                        line_state.apply_pen(pen, &defaults);
                    }

                    let mut override_tags = Vec::new();
                    if let Some(position_tag) = position_override(paragraph, Some(head)) {
                        override_tags.push(position_tag);
                    }
                    override_tags.extend(transition_tags(&default_state, &line_state));

                    let prefix = if !override_tags.is_empty() {
                        format!("{{{}}}", override_tags.join(""))
                    } else {
                        String::new()
                    };

                    let mut current_state = line_state.clone();
                    let body_text =
                        render_body_elements(&paragraph.inner, head, &defaults, &mut current_state);
                    let body_text = trim_ass_edge_whitespace(body_text);
                    text = format!("{}{}", prefix, body_text);
                } else {
                    let mut override_tags = Vec::new();
                    if let Some(position_tag) = position_override(paragraph, None) {
                        override_tags.push(position_tag);
                    }
                    let prefix = if !override_tags.is_empty() {
                        format!("{{{}}}", override_tags.join(""))
                    } else {
                        String::new()
                    };
                    let body_text = paragraph.inner.text_clean_ass();
                    let body_text = trim_ass_edge_whitespace(body_text);
                    text = format!("{}{}", prefix, body_text);
                }
            } else {
                let mut override_tags = Vec::new();
                if let Some(position_tag) = position_override(paragraph, head) {
                    override_tags.push(position_tag);
                }
                let prefix = if !override_tags.is_empty() {
                    format!("{{{}}}", override_tags.join(""))
                } else {
                    String::new()
                };
                let body_text = paragraph.inner.text_clean_ass();
                let body_text = trim_ass_edge_whitespace(body_text);
                text = format!("{}{}", prefix, body_text);
            }

            writeln!(
                &mut w,
                "Dialogue: {},{},{},{},,{},{},{},,{}",
                LAYER, start_ts, end_ts, style_name, MARGIN_L, MARGIN_R, MARGIN_V, text
            )
            .unwrap();
        }
    }

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
