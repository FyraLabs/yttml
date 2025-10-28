pub use aspasia::timing::Moment;
use aspasia::substation::ass::{AssEvent, AssScriptInfo, AssStyle, AssSubtitle};
use aspasia::substation::SubStationEventKind;
use aspasia::{SubRipSubtitle, Subtitle, WebVttSubtitle};
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

fn hex_components(hex: &HexColor) -> Option<(u8, u8, u8)> {
    let hex_str = format!("{:?}", hex);
    if !(hex_str.contains("r:") && hex_str.contains("g:") && hex_str.contains("b:")) {
        return None;
    }

    let r = hex_str
        .split("r:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<u8>().ok())?;
    let g = hex_str
        .split("g:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<u8>().ok())?;
    let b = hex_str
        .split("b:")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<u8>().ok())?;

    Some((r, g, b))
}

fn clamp_u32_to_u8(value: u32) -> u8 {
    if value > u8::MAX as u32 {
        u8::MAX
    } else {
        value as u8
    }
}

const MARKER_NEWLINE: &str = "__YTTML_ASS_NEWLINE__";
const MARKER_LIT_UPPER_N: &str = "__YTTML_ASS_LITERAL_N__";
const MARKER_LIT_LOWER_N: &str = "__YTTML_ASS_LITERAL_n__";
const MARKER_LIT_UPPER_H: &str = "__YTTML_ASS_LITERAL_H__";
const MARKER_LIT_LOWER_H: &str = "__YTTML_ASS_LITERAL_h__";
const MARKER_LIT_LBRACE: &str = "__YTTML_ASS_LITERAL_LBRACE__";
const MARKER_LIT_RBRACE: &str = "__YTTML_ASS_LITERAL_RBRACE__";
const NO_ANDROID_DARK_TEXT_HACK: &str = "no_android_dark_text_hack";

fn paragraph_has_visible_pen(elements: &[BodyElement], head: &Head) -> bool {
    fn helper(elements: &[BodyElement], head: &Head, found: &mut bool) -> bool {
        let mut visible = false;

        for element in elements {
            match element {
                BodyElement::Span(span) => {
                    if let Some(id) = span.pen {
                        if let Some(pen) = find_pen(head, id) {
                            *found = true;
                            let opacity = pen.foreground_opacity;
                            if opacity != Some(0) {
                                visible = true;
                            }
                        }
                    }

                    if let Some(inner) = span.inner.as_deref() {
                        if helper(inner, head, found) {
                            visible = true;
                        }
                    }
                }
                BodyElement::Paragraph(paragraph) => {
                    if helper(&paragraph.inner, head, found) {
                        visible = true;
                    }
                }
                BodyElement::Div(children) => {
                    if helper(children, head, found) {
                        visible = true;
                    }
                }
                _ => {}
            }
        }

        visible
    }

    let mut found_pen = false;
    let visible = helper(elements, head, &mut found_pen);
    if !found_pen {
        true
    } else {
        visible
    }
}

fn prepare_ass_text(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut prev_was_zwsp = false;
    while let Some(ch) = chars.next() {
        if ch == '\u{200B}' {
            prev_was_zwsp = true;
            continue;
        }

        if ch == ' ' && prev_was_zwsp && matches!(chars.peek(), Some('\u{200B}')) {
            // Skip the padding space sandwiched between zero-width spaces.
            prev_was_zwsp = false;
            continue;
        }

        cleaned.push(ch);
        prev_was_zwsp = false;
    }

    let mut output = String::with_capacity(cleaned.len());
    let mut iter = cleaned.chars().peekable();
    while let Some(ch) = iter.next() {
        if ch == '\\' {
            if let Some(next) = iter.peek().copied() {
                match next {
                    'N' => {
                        iter.next();
                        output.push_str(MARKER_LIT_UPPER_N);
                        continue;
                    }
                    'n' => {
                        iter.next();
                        output.push_str(MARKER_LIT_LOWER_N);
                        continue;
                    }
                    'H' => {
                        iter.next();
                        output.push_str(MARKER_LIT_UPPER_H);
                        continue;
                    }
                    'h' => {
                        iter.next();
                        output.push_str(MARKER_LIT_LOWER_H);
                        continue;
                    }
                    '{' => {
                        iter.next();
                        output.push_str(MARKER_LIT_LBRACE);
                        continue;
                    }
                    '}' => {
                        iter.next();
                        output.push_str(MARKER_LIT_RBRACE);
                        continue;
                    }
                    _ => {}
                }
            }
            output.push('\\');
        } else if ch == '\r' {
            continue;
        } else if ch == '\n' {
            output.push_str(MARKER_NEWLINE);
        } else {
            output.push(ch);
        }
    }

    output
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
        prepare_ass_text(self)
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
    has_background: bool,
    edge_type: Option<EdgeType>,
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
            has_background: self.has_background,
            edge_type: self.edge_type,
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
            has_background: false,
            edge_type: None,
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
            let clamped = clamp_u32_to_u8(opacity);
            self.primary_alpha = 255u8.saturating_sub(clamped);
        }

        let effective_edge_type = pen.edge_type.or(self.edge_type);

        if let Some(opacity) = pen.background_opacity {
            let clamped = clamp_u32_to_u8(u32::from(opacity));
            if clamped == 0 {
                self.has_background = false;
                self.outline_alpha = defaults.outline_alpha;
                self.back_alpha = defaults.back_alpha;
            } else {
                let alpha = 255u8.saturating_sub(clamped);
                self.outline_alpha = alpha;
                if matches!(
                    effective_edge_type,
                    Some(EdgeType::Glow)
                        | Some(EdgeType::SoftShadow)
                        | Some(EdgeType::HardShadow)
                        | Some(EdgeType::Bevel)
                ) {
                    self.back_alpha = alpha;
                }
                self.has_background = true;
            }
        }

        let mut edge_color_override = pen.edge_color.as_ref().map(hex_to_ass_color);
        let mut edge_alpha_override: Option<u8> = None;
        if edge_color_override.is_none() && pen.edge_type.or(self.edge_type).is_some() {
            let fallback_alpha = pen.foreground_opacity.map(clamp_u32_to_u8).unwrap_or(254);
            edge_color_override = Some("&H222222".to_string());
            edge_alpha_override = Some(255u8.saturating_sub(fallback_alpha));
        }

        let mut edge_channel_overridden = false;
        if let Some(color) = edge_color_override {
            let effective_edge_type = pen.edge_type.or(self.edge_type);
            match effective_edge_type {
                Some(EdgeType::Glow) => {
                    if self.has_background {
                        self.back_color = color;
                        if let Some(alpha) = edge_alpha_override {
                            self.back_alpha = alpha;
                        }
                    } else {
                        self.outline_color = color;
                        if let Some(alpha) = edge_alpha_override {
                            self.outline_alpha = alpha;
                        }
                    }
                }
                Some(EdgeType::SoftShadow) | Some(EdgeType::HardShadow) | Some(EdgeType::Bevel) => {
                    self.back_color = color;
                    if let Some(alpha) = edge_alpha_override {
                        self.back_alpha = alpha;
                    }
                    edge_channel_overridden = true;
                }
                _ => {
                    self.outline_color = color;
                    if let Some(alpha) = edge_alpha_override {
                        self.outline_alpha = alpha;
                    }
                }
            }
        }

        if let Some(color) = pen.background_color.as_ref() {
            let ass_color = hex_to_ass_color(color);
            self.outline_color = ass_color.clone();
            if matches!(
                effective_edge_type,
                Some(EdgeType::Glow)
                    | Some(EdgeType::SoftShadow)
                    | Some(EdgeType::HardShadow)
                    | Some(EdgeType::Bevel)
            ) && !edge_channel_overridden
            {
                self.back_color = ass_color.clone();
            }
            self.has_background = true;
        }

        if let Some(edge_type) = pen.edge_type {
            self.edge_type = match edge_type {
                EdgeType::None => {
                    self.back_color = defaults.back_color.clone();
                    None
                }
                other => Some(other),
            };
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

fn default_edge_type_for_style(style_name: &str) -> Option<EdgeType> {
    if style_name.contains("Glow") {
        Some(EdgeType::Glow)
    } else if style_name.contains("SoftShadow") {
        Some(EdgeType::SoftShadow)
    } else if style_name.contains("HardShadow") {
        Some(EdgeType::HardShadow)
    } else if style_name.contains("Bevel") {
        Some(EdgeType::Bevel)
    } else {
        None
    }
}

fn format_ass_float(value: f64) -> String {
    let rounded = (value * 10000.0).round() / 10000.0;

    if rounded == 0.0 {
        return "0".to_string();
    }

    let mut buffer = ryu::Buffer::new();
    let repr = buffer.format_finite(rounded);
    let mut s = repr.to_string();

    if s.contains('.') {
        let trimmed_len = {
            let trimmed = s.trim_end_matches('0').trim_end_matches('.');
            trimmed.len()
        };
        if trimmed_len == 0 {
            return "0".to_string();
        }
        s.truncate(trimmed_len);
    }

    s
}

fn floats_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.00005
}

fn trim_ass_edge_whitespace(text: String) -> String {
    text.trim_matches([' ', '\u{200B}']).to_string()
}

fn sanitize_ass_text(mut text: String) -> String {
    if text.is_empty() {
        return text;
    }

    // todo: consider better escape sequence
    text = text.replace("\r\n", "\n");
    text = text.replace('\r', "\n");
    text = text.replace('\u{00A0}', "\\h");
    text = text.replace(MARKER_NEWLINE, "\\N");
    text = text.replace(MARKER_LIT_UPPER_N, "\\{}N");
    text = text.replace(MARKER_LIT_LOWER_N, "\\{}n");
    text = text.replace(MARKER_LIT_UPPER_H, "\\{}H");
    text = text.replace(MARKER_LIT_LOWER_H, "\\{}h");
    text = text.replace(MARKER_LIT_LBRACE, "\\{");
    text = text.replace(MARKER_LIT_RBRACE, "\\}");

    const CONTROL_SEQS: [&str; 6] = ["\\N", "\\n", "\\{}N", "\\{}n", "\\{}H", "\\{}h"];
    if text.contains(' ') {
        for seq in CONTROL_SEQS {
            let prefix_pattern = format!(" {}", seq);
            while let Some(pos) = text.find(&prefix_pattern) {
                text.replace_range(pos..pos + 1, "");
            }

            let suffix_pattern = format!("{} ", seq);
            while let Some(pos) = text.find(&suffix_pattern) {
                let remove_start = pos + seq.len();
                text.replace_range(remove_start..remove_start + 1, "");
            }
        }
    }

    fn strip_redundant_tags(text: &mut String, newline: &str) {
        let pattern = format!("}}{}", newline);
        while let Some(pos) = text.find(&pattern) {
            if let Some(start) = text[..pos].rfind('{') {
                text.replace_range(start..pos + 1, "");
            } else {
                break;
            }
        }
    }

    strip_redundant_tags(&mut text, "\\N");
    strip_redundant_tags(&mut text, "\\n");

    text
}

fn ass_placeholders_to_plain_text(text: &str) -> String {
    text.replace(MARKER_NEWLINE, "\n")
        .replace(MARKER_LIT_UPPER_N, "\\N")
        .replace(MARKER_LIT_LOWER_N, "\\n")
        .replace(MARKER_LIT_UPPER_H, "\\H")
        .replace(MARKER_LIT_LOWER_H, "\\h")
        .replace(MARKER_LIT_LBRACE, "{")
        .replace(MARKER_LIT_RBRACE, "}")
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
        tags.push(format!("\\1a&H{:X}&", to.primary_alpha));
    }

    if from.outline_color != to.outline_color {
        tags.push(format!("\\3c{}&", to.outline_color));
    }

    if from.outline_alpha != to.outline_alpha {
        tags.push(format!("\\3a&H{:X}&", to.outline_alpha));
    }

    if from.back_color != to.back_color {
        tags.push(format!("\\4c{}&", to.back_color));
    }

    if from.back_alpha != to.back_alpha {
        tags.push(format!("\\4a&H{:X}&", to.back_alpha));
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

#[derive(Clone, Copy, Debug)]
struct PenUsage {
    is_dark: bool,
    fore_alpha: u8,
    back_alpha: u8,
    has_shadow: bool,
}

fn collect_pen_usage(
    elements: &[BodyElement],
    head: &Head,
    inherited: Option<&Pen>,
    usage: &mut Vec<PenUsage>,
) {
    for element in elements {
        match element {
            BodyElement::Span(span) => {
                let current_pen = span.pen.and_then(|id| find_pen(head, id)).or(inherited);

                if let Some(inner) = span.inner.as_deref() {
                    collect_pen_usage(inner, head, current_pen, usage);
                } else if let Some(current_pen) = current_pen {
                    let props = pen_usage_from_pen(current_pen);
                    usage.push(props);
                }
            }
            BodyElement::Text(text) => {
                let has_visible_char = text
                    .chars()
                    .any(|ch| ch != '\u{200B}' && !ch.is_whitespace());
                if !has_visible_char {
                    continue;
                }

                if let Some(pen) = inherited {
                    usage.push(pen_usage_from_pen(pen));
                } else {
                    usage.push(default_pen_usage());
                }
            }
            BodyElement::Paragraph(paragraph) => {
                collect_pen_usage(&paragraph.inner, head, inherited, usage);
            }
            BodyElement::Div(children) => {
                collect_pen_usage(children, head, inherited, usage);
            }
            BodyElement::Br(_) => {}
            BodyElement::Window(_) => {}
        }
    }
}

fn pen_usage_from_pen(pen: &Pen) -> PenUsage {
    let (r, g, b) = pen
        .foreground_color
        .as_ref()
        .and_then(hex_components)
        .unwrap_or((255, 255, 255));
    let is_dark = r.max(g).max(b) < 128;

    let fore_alpha = pen.foreground_opacity.map(clamp_u32_to_u8).unwrap_or(254);
    let back_alpha = pen
        .background_opacity
        .map(|value| clamp_u32_to_u8(value as u32))
        .unwrap_or(0);
    let has_shadow = pen.edge_type.is_some();

    PenUsage {
        is_dark,
        fore_alpha,
        back_alpha,
        has_shadow,
    }
}

fn default_pen_usage() -> PenUsage {
    PenUsage {
        is_dark: false,
        fore_alpha: 254,
        back_alpha: 0,
        has_shadow: false,
    }
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
                let original_state = current_state.clone();
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

                let plain_text = ass_placeholders_to_plain_text(&inner_output);
                let has_visible_text = plain_text.chars().any(|ch| !ch.is_whitespace());

                let tags = transition_tags(current_state, &target_state);
                if !tags.is_empty() {
                    output.push_str(&format!("{{{}}}", tags.join("")));
                }

                output.push_str(&inner_output);
                if has_visible_text {
                    *current_state = inner_state;
                } else {
                    *current_state = original_state;
                }
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
        let rounded = (value * 10000.0).round() / 10000.0;
        let mut s = format!("{:.4}", rounded);
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

fn create_youtube_styles() -> Vec<AssStyle> {
    vec![
        AssStyle {
            name: "YTPlain".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H00000000".to_string(),
            back_colour: "&H00000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 1,
            outline: 0,
            shadow: 0,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTPlainBox".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H00000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 3,
            outline: 0,
            shadow: 0,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTGlow".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 1,
            outline: 2,
            shadow: 0,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTGlowBox".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 3,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTSoftShadow".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 1,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTSoftShadowBox".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 3,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTHardShadow".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 1,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTHardShadowBox".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 3,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTBevel".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H01000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 1,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
        AssStyle {
            name: "YTBevelBox".to_string(),
            fontname: "Roboto".to_string(),
            fontsize: 38,
            primary_colour: "&H01FEFEFE".to_string(),
            secondary_colour: "&HFF000000".to_string(),
            outline_colour: "&H01000000".to_string(),
            back_colour: "&H00000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike_out: false,
            scale_x: 100,
            scale_y: 100,
            spacing: 0,
            angle: 0.0,
            border_style: 3,
            outline: 0,
            shadow: 4,
            alignment: 2,
            margin_l: 25,
            margin_r: 25,
            margin_v: 15,
            encoding: 1,
        },
    ]
}

pub fn to_ass(captions: &srv3_ttml::TimedText) -> std::io::Result<String> {
    const MARGIN_L: i64 = 0;
    const MARGIN_R: i64 = 0;
    const MARGIN_V: i64 = 0;
    const LAYER: i64 = 0;
    const PLAY_RES_X: &str = "1280";
    const PLAY_RES_Y: &str = "720";

    let head = captions.head.as_ref();
    let has_pens = head.is_some_and(|h| !h.pen.is_empty());
    let mut awaiting_android_hack = false;
    let mut previous_line_info: Option<(Moment, Moment, String)> = None;

    // Create script info
    let script_info = AssScriptInfo::builder()
        .title("Generated by YTTML")
        .script_type("v4.00+")
        .wrap_style("0")
        .play_res_x(PLAY_RES_X)
        .play_res_y(PLAY_RES_Y)
        .collisions("Normal")
        .build();

    // Create styles
    let styles = create_youtube_styles();

    // Create dialogue events
    let mut dialogue_events = Vec::new();

    for element in &captions.body.elements {
        if let BodyElement::Paragraph(paragraph) = element {
            let start = Moment::from(paragraph.timestamp as i64);
            let end = Moment::from((paragraph.timestamp + paragraph.duration) as i64);
            let start_ts = Moment::as_substation_timestamp(&start);
            let _end_ts = Moment::as_substation_timestamp(&end);

            let mut style_name: &'static str = "YTGlow";
            let mut effect: Option<String> = None;
            let mut skip_line = false;
            let text: String;

            if let Some(head) = head {
                let mut usage = Vec::new();
                collect_pen_usage(&paragraph.inner, head, None, &mut usage);
                #[cfg(debug_assertions)]
                if paragraph.timestamp == 10060 {
                    for (idx, props) in usage.iter().enumerate() {
                        eprintln!(
                            "usage[{idx}] -> is_dark={} fore_alpha={} back_alpha={} has_shadow={}",
                            props.is_dark, props.fore_alpha, props.back_alpha, props.has_shadow
                        );
                    }
                }
                let line_has_dark_text = usage
                    .iter()
                    .any(|props| props.is_dark && props.fore_alpha > 0);
                let line_is_android_fallback = !usage.is_empty()
                    && usage.iter().all(|props| {
                        props.fore_alpha == 0 && props.back_alpha == 0 && !props.has_shadow
                    });

                #[cfg(debug_assertions)]
                if paragraph.timestamp == 10060 {
                    eprintln!(
                        "pre-check: start={} fallback={} visible={} usage={}",
                        start_ts,
                        line_is_android_fallback,
                        paragraph_has_visible_pen(&paragraph.inner, head),
                        usage.len()
                    );
                }

                if !line_is_android_fallback && !paragraph_has_visible_pen(&paragraph.inner, head) {
                    continue;
                }

                let raw_text = paragraph.inner.text_clean_ass();
                let plain_text = ass_placeholders_to_plain_text(&raw_text);

                #[cfg(debug_assertions)]
                if paragraph.timestamp == 10060 {
                    eprintln!(
                        "debug android fallback: start={} fallback={} dark={} usage={} text={}",
                        start_ts,
                        line_is_android_fallback,
                        line_has_dark_text,
                        usage.len(),
                        plain_text
                    );
                }

                if usage.is_empty() && plain_text.trim().is_empty() {
                    continue;
                }

                if line_has_dark_text {
                    awaiting_android_hack = true;
                } else if awaiting_android_hack {
                    if let Some((prev_start, prev_end, prev_text)) = previous_line_info.as_ref() {
                        if line_is_android_fallback
                            && prev_start == &start
                            && prev_end == &end
                            && prev_text == &plain_text
                        {
                            skip_line = true;
                        } else {
                            effect = Some(NO_ANDROID_DARK_TEXT_HACK.to_string());
                        }
                    }
                    awaiting_android_hack = false;
                }

                if skip_line {
                    continue;
                }

                previous_line_info = Some((start, end, plain_text.clone()));
            }

            if has_pens {
                if let Some(head) = head {
                    let base_pen = first_text_pen(&paragraph.inner, head);
                    style_name = choose_style_for_pen(base_pen);
                    let defaults = style_defaults(style_name);
                    let mut default_state = FormattingState::from_defaults(&defaults);
                    default_state.has_background = style_name.ends_with("Box");
                    default_state.edge_type = default_edge_type_for_style(style_name);
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
                    let body_text = sanitize_ass_text(body_text);
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
                    let body_text = sanitize_ass_text(paragraph.inner.text_clean_ass());
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
                let body_text = sanitize_ass_text(paragraph.inner.text_clean_ass());
                let body_text = trim_ass_edge_whitespace(body_text);
                text = format!("{}{}", prefix, body_text);
            }

            // Create AssEvent
            dialogue_events.push(AssEvent {
                kind: SubStationEventKind::Dialogue,
                layer: LAYER,
                start,
                end,
                style: Some(style_name.to_string()),
                name: None,
                margin_l: MARGIN_L,
                margin_r: MARGIN_R,
                margin_v: MARGIN_V,
                effect,
                text,
            });
        }
    }

    // Build the AssSubtitle
    let subtitle = AssSubtitle::builder()
        .script_info(script_info)
        .styles(styles)
        .dialogue(dialogue_events)
        .build();

    // We can't use aspasia's Display directly due to bugs:
    // 1. Missing "Style: " prefix
    // 2. Boolean serialization is "true"/"false" instead of 0/1
    // 3. Missing "ScaledBorderAndShadow" field
    // So we'll manually format the ASS output
    
    let mut output = String::new();
    
    // Write Script Info
    writeln!(&mut output, "[Script Info]").unwrap();
    writeln!(&mut output, "; Script generated by YTTML").unwrap();
    writeln!(&mut output, "; https://github.com/FyraLabs/yttml/").unwrap();
    writeln!(&mut output, "ScriptType: v4.00+").unwrap();
    writeln!(&mut output, "WrapStyle: 0").unwrap();
    writeln!(&mut output, "ScaledBorderAndShadow: yes").unwrap();
    writeln!(&mut output, "PlayResX: {}", PLAY_RES_X).unwrap();
    writeln!(&mut output, "PlayResY: {}", PLAY_RES_Y).unwrap();
    output.push('\n');
    
    // Write Styles
    writeln!(&mut output, "[V4+ Styles]").unwrap();
    writeln!(&mut output, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").unwrap();
    
    for style in subtitle.styles() {
        // Work around aspasia limitation: Box styles need outline=0.01 for border_style=3
        // but outline field is i64, so we manually format it
        let outline_str = if style.border_style == 3 && style.outline == 0 {
            "0.01".to_string()
        } else {
            style.outline.to_string()
        };
        
        writeln!(
            &mut output,
            "Style: {},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            style.name,
            style.fontname,
            style.fontsize,
            style.primary_colour,
            style.secondary_colour,
            style.outline_colour,
            style.back_colour,
            if style.bold { 1 } else { 0 },
            if style.italic { 1 } else { 0 },
            if style.underline { 1 } else { 0 },
            if style.strike_out { 1 } else { 0 },
            style.scale_x,
            style.scale_y,
            style.spacing,
            style.angle,
            style.border_style,
            outline_str,
            style.shadow,
            style.alignment,
            style.margin_l,
            style.margin_r,
            style.margin_v,
            style.encoding
        ).unwrap();
    }
    output.push('\n');
    
    // Write Events
    writeln!(&mut output, "[Events]").unwrap();
    writeln!(&mut output, "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text").unwrap();
    
    for event in subtitle.events() {
        writeln!(
            &mut output,
            "{}: {},{},{},{},{},{},{},{},{},{}",
            event.kind,
            event.layer,
            event.start.as_substation_timestamp(),
            event.end.as_substation_timestamp(),
            event.style.as_deref().unwrap_or(""),
            event.name.as_deref().unwrap_or(""),
            event.margin_l,
            event.margin_r,
            event.margin_v,
            event.effect.as_deref().unwrap_or(""),
            event.text
        ).unwrap();
    }
    
    Ok(output)
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
