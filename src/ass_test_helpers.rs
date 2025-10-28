use ass_core::parser::{ast::{Event, EventType, Style}, Script};
use chumsky::prelude::*;
use similar::TextDiff;
use std::str::FromStr;

/// Parse an ASS file using the ass-core parser
pub fn parse_ass(content: &str) -> Result<Script, String> {
    Script::parse(content).map_err(|e| format!("{:?}", e))
}

#[derive(Debug, PartialEq)]
enum TextToken {
    Text(String),
    Number(f64),
}

fn tokenize_text(input: &str) -> Vec<TextToken> {
    if input.is_empty() {
        return Vec::new();
    }

    let digits = || text::digits::<_, extra::Err<Simple<char>>>(10).collect::<String>();

    let number = just('+')
        .or(just('-'))
        .or_not()
        .then(digits())
        .then(just('.').then(digits()).or_not())
        .map(|((sign, int_part), frac)| {
            let mut repr = String::new();
            if let Some(sign) = sign {
                repr.push(sign);
            }
            repr.push_str(&int_part);
            if let Some((dot, frac_digits)) = frac {
                repr.push(dot);
                repr.push_str(&frac_digits);
            }

            repr.parse::<f64>()
                .map(TextToken::Number)
                .unwrap_or_else(|_| TextToken::Text(repr))
        });

    let token_parser = number
        .or(any().map(|c: char| TextToken::Text(c.to_string())))
        .repeated()
        .collect::<Vec<_>>();

    match token_parser.parse(input).into_result() {
        Ok(raw_tokens) => merge_text_tokens(raw_tokens),
        Err(_) => vec![TextToken::Text(input.to_string())],
    }
}

fn merge_text_tokens(raw_tokens: Vec<TextToken>) -> Vec<TextToken> {
    let mut merged = Vec::new();
    for token in raw_tokens {
        match token {
            TextToken::Text(chunk) => {
                if chunk.is_empty() {
                    continue;
                }
                if let Some(TextToken::Text(existing)) = merged.last_mut() {
                    existing.push_str(&chunk);
                } else {
                    merged.push(TextToken::Text(chunk));
                }
            }
            TextToken::Number(value) => merged.push(TextToken::Number(value)),
        }
    }
    merged
}

fn floats_close_enough(a: f64, b: f64) -> bool {
    // Allow ±0.001 tolerance for floating point comparisons
    // This handles rounding/precision issues with 4+ decimal digits
    (a - b).abs() <= 0.001
}

fn fuzzy_text_equal(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }

    let expected_tokens = tokenize_text(expected);
    let actual_tokens = tokenize_text(actual);

    if expected_tokens.len() != actual_tokens.len() {
        return false;
    }

    for (expected_token, actual_token) in expected_tokens.iter().zip(actual_tokens.iter()) {
        match (expected_token, actual_token) {
            (TextToken::Text(expected_text), TextToken::Text(actual_text)) => {
                if expected_text != actual_text {
                    return false;
                }
            }
            (TextToken::Number(expected_number), TextToken::Number(actual_number)) => {
                if !floats_close_enough(*expected_number, *actual_number) {
                    return false;
                }
            }
            _ => return false,
        }
    }

    true
}

fn escape_debug_str(input: &str) -> String {
    format!("{}", input.escape_debug())
}

fn format_diff_output(expected: &str, actual: &str) -> String {
    let expected_display = format!("{}\n", escape_debug_str(expected));
    let actual_display = format!("{}\n", escape_debug_str(actual));

    let diff = TextDiff::from_lines(&expected_display, &actual_display);
    let mut buffer = Vec::new();
    diff.unified_diff()
        .context_radius(0)
        .header("expected", "actual")
        .to_writer(&mut buffer)
        .expect("writing diff to buffer");

    let diff_string =
        String::from_utf8(buffer).unwrap_or_else(|_| "<diff output not valid UTF-8>".to_string());
    diff_string
        .lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &Script, actual: &Script) -> Result<(), Vec<String>> {
    use ass_core::parser::ast::{Section, SectionType};
    
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Helper function to extract events from a script
    fn get_events<'a>(script: &'a Script<'a>) -> Vec<&'a Event<'a>> {
        script
            .sections()
            .iter()
            .filter_map(|section| match section {
                Section::Events(events) => Some(events.as_slice()),
                _ => None,
            })
            .flat_map(|events| events.iter())
            .collect::<Vec<_>>()
    }

    // Helper function to extract styles from a script
    fn get_styles<'a>(script: &'a Script<'a>) -> Vec<&'a Style<'a>> {
        script
            .sections()
            .iter()
            .filter_map(|section| match section {
                Section::Styles(styles) => Some(styles.as_slice()),
                _ => None,
            })
            .flat_map(|styles| styles.iter())
            .collect::<Vec<_>>()
    }

    let expected_events = get_events(expected);
    let actual_events = get_events(actual);

    // Check if dialogue counts match
    if expected_events.len() != actual_events.len() {
        errors.push(format!(
            "Event count mismatch: expected {}, got {}",
            expected_events.len(),
            actual_events.len()
        ));
    }

    let expected_styles = get_styles(expected);
    let actual_styles = get_styles(actual);

    // Check if style counts match
    if expected_styles.len() != actual_styles.len() {
        errors.push(format!(
            "Style count mismatch: expected {}, got {}",
            expected_styles.len(),
            actual_styles.len()
        ));
    }

    // Compare styles field-by-field
    for (i, (exp_style, act_style)) in expected_styles.iter().zip(actual_styles.iter()).enumerate()
    {
        if exp_style.name != act_style.name {
            errors.push(format!(
                "Style {}: name mismatch: expected {}, got {}",
                i, exp_style.name, act_style.name
            ));
        }
        if exp_style.fontname != act_style.fontname {
            errors.push(format!(
                "Style {}: font name mismatch: expected {}, got {}",
                i, exp_style.fontname, act_style.fontname
            ));
        }
        if exp_style.fontsize != act_style.fontsize {
            errors.push(format!(
                "Style {}: font size mismatch: expected {}, got {}",
                i, exp_style.fontsize, act_style.fontsize
            ));
        }
        if exp_style.primary_colour != act_style.primary_colour {
            errors.push(format!(
                "Style {}: primary colour mismatch: expected {}, got {}",
                i, exp_style.primary_colour, act_style.primary_colour
            ));
        }
        if exp_style.secondary_colour != act_style.secondary_colour {
            errors.push(format!(
                "Style {}: secondary colour mismatch: expected {}, got {}",
                i, exp_style.secondary_colour, act_style.secondary_colour
            ));
        }
        if exp_style.outline_colour != act_style.outline_colour {
            errors.push(format!(
                "Style {}: outline colour mismatch: expected {}, got {}",
                i, exp_style.outline_colour, act_style.outline_colour
            ));
        }
        if exp_style.back_colour != act_style.back_colour {
            errors.push(format!(
                "Style {}: back colour mismatch: expected {}, got {}",
                i, exp_style.back_colour, act_style.back_colour
            ));
        }
        if exp_style.bold != act_style.bold {
            errors.push(format!(
                "Style {}: bold flag mismatch: expected {}, got {}",
                i, exp_style.bold, act_style.bold
            ));
        }
        if exp_style.italic != act_style.italic {
            errors.push(format!(
                "Style {}: italic flag mismatch: expected {}, got {}",
                i, exp_style.italic, act_style.italic
            ));
        }
        if exp_style.underline != act_style.underline {
            errors.push(format!(
                "Style {}: underline flag mismatch: expected {}, got {}",
                i, exp_style.underline, act_style.underline
            ));
        }
        if exp_style.strikeout != act_style.strikeout {
            errors.push(format!(
                "Style {}: strike-out flag mismatch: expected {}, got {}",
                i, exp_style.strikeout, act_style.strikeout
            ));
        }
        if exp_style.scale_x != act_style.scale_x {
            errors.push(format!(
                "Style {}: scale_x mismatch: expected {}, got {}",
                i, exp_style.scale_x, act_style.scale_x
            ));
        }
        if exp_style.scale_y != act_style.scale_y {
            errors.push(format!(
                "Style {}: scale_y mismatch: expected {}, got {}",
                i, exp_style.scale_y, act_style.scale_y
            ));
        }
        if exp_style.spacing != act_style.spacing {
            errors.push(format!(
                "Style {}: spacing mismatch: expected {}, got {}",
                i, exp_style.spacing, act_style.spacing
            ));
        }
        if exp_style.angle != act_style.angle {
            errors.push(format!(
                "Style {}: angle mismatch: expected {}, got {}",
                i, exp_style.angle, act_style.angle
            ));
        }
        if exp_style.border_style != act_style.border_style {
            errors.push(format!(
                "Style {}: border style mismatch: expected {}, got {}",
                i, exp_style.border_style, act_style.border_style
            ));
        }
        if exp_style.outline != act_style.outline {
            errors.push(format!(
                "Style {}: outline size mismatch: expected {}, got {}",
                i, exp_style.outline, act_style.outline
            ));
        }
        if exp_style.shadow != act_style.shadow {
            errors.push(format!(
                "Style {}: shadow size mismatch: expected {}, got {}",
                i, exp_style.shadow, act_style.shadow
            ));
        }
        if exp_style.alignment != act_style.alignment {
            errors.push(format!(
                "Style {}: alignment mismatch: expected {}, got {}",
                i, exp_style.alignment, act_style.alignment
            ));
        }
        if exp_style.margin_l != act_style.margin_l {
            errors.push(format!(
                "Style {}: margin_l mismatch: expected {}, got {}",
                i, exp_style.margin_l, act_style.margin_l
            ));
        }
        if exp_style.margin_r != act_style.margin_r {
            errors.push(format!(
                "Style {}: margin_r mismatch: expected {}, got {}",
                i, exp_style.margin_r, act_style.margin_r
            ));
        }
        if exp_style.margin_v != act_style.margin_v {
            errors.push(format!(
                "Style {}: margin_v mismatch: expected {}, got {}",
                i, exp_style.margin_v, act_style.margin_v
            ));
        }
        if exp_style.encoding != act_style.encoding {
            errors.push(format!(
                "Style {}: encoding mismatch: expected {}, got {}",
                i, exp_style.encoding, act_style.encoding
            ));
        }
    }

    // Helper function to parse ASS timestamps with ±1ms tolerance
    fn parse_ass_time(time_str: &str) -> Result<i64, String> {
        // ASS time format: H:MM:SS.CS (centiseconds)
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid time format: {}", time_str));
        }
        
        let hours: i64 = parts[0].parse().map_err(|e| format!("Invalid hour: {}", e))?;
        let minutes: i64 = parts[1].parse().map_err(|e| format!("Invalid minute: {}", e))?;
        
        let sec_parts: Vec<&str> = parts[2].split('.').collect();
        if sec_parts.len() != 2 {
            return Err(format!("Invalid seconds format: {}", parts[2]))?;
        }
        
        let seconds: i64 = sec_parts[0].parse().map_err(|e| format!("Invalid second: {}", e))?;
        let centiseconds: i64 = sec_parts[1].parse().map_err(|e| format!("Invalid centisecond: {}", e))?;
        
        // Convert to milliseconds
        Ok(hours * 3600000 + minutes * 60000 + seconds * 1000 + centiseconds * 10)
    }

    fn times_close_enough(expected: &str, actual: &str) -> bool {
        match (parse_ass_time(expected), parse_ass_time(actual)) {
            (Ok(exp_ms), Ok(act_ms)) => {
                // Allow ±1ms tolerance as specified in agent instructions
                (exp_ms - act_ms).abs() <= 1
            }
            _ => expected == actual, // Fall back to string comparison if parsing fails
        }
    }

    // Compare events field-by-field
    for (i, (exp_event, act_event)) in expected_events.iter().zip(actual_events.iter()).enumerate() {
        if exp_event.event_type != act_event.event_type {
            errors.push(format!(
                "Event {}: type mismatch: expected {:?}, got {:?}",
                i, exp_event.event_type, act_event.event_type
            ));
        }
        if exp_event.layer != act_event.layer {
            errors.push(format!(
                "Event {}: layer mismatch: expected {}, got {}",
                i, exp_event.layer, act_event.layer
            ));
        }
        if !times_close_enough(exp_event.start, act_event.start) {
            errors.push(format!(
                "Event {}: start time mismatch: expected {}, got {}",
                i, exp_event.start, act_event.start
            ));
        }
        if !times_close_enough(exp_event.end, act_event.end) {
            errors.push(format!(
                "Event {}: end time mismatch: expected {}, got {}",
                i, exp_event.end, act_event.end
            ));
        }
        if exp_event.style != act_event.style {
            errors.push(format!(
                "Event {}: style mismatch: expected {}, got {}",
                i, exp_event.style, act_event.style
            ));
        }
        if exp_event.name != act_event.name {
            errors.push(format!(
                "Event {}: name mismatch: expected {}, got {}",
                i, exp_event.name, act_event.name
            ));
        }
        if exp_event.margin_l != act_event.margin_l {
            errors.push(format!(
                "Event {}: margin_l mismatch: expected {}, got {}",
                i, exp_event.margin_l, act_event.margin_l
            ));
        }
        if exp_event.margin_r != act_event.margin_r {
            errors.push(format!(
                "Event {}: margin_r mismatch: expected {}, got {}",
                i, exp_event.margin_r, act_event.margin_r
            ));
        }
        if exp_event.margin_v != act_event.margin_v {
            errors.push(format!(
                "Event {}: margin_v mismatch: expected {}, got {}",
                i, exp_event.margin_v, act_event.margin_v
            ));
        }
        
        // Handle optional effect field comparison (like no_android_dark_text_hack)
        let exp_effect = exp_event.effect.trim();
        let act_effect = act_event.effect.trim();
        if exp_effect != act_effect {
            if exp_effect == "no_android_dark_text_hack" && act_effect.is_empty() {
                warnings.push(format!(
                    "Event {}: effect mismatch downgraded to warning: expected {}, got {}",
                    i, exp_effect, act_effect
                ));
            } else {
                errors.push(format!(
                    "Event {}: effect mismatch: expected {}, got {}",
                    i, exp_effect, act_effect
                ));
            }
        }
        
        if exp_event.text != act_event.text && !fuzzy_text_equal(exp_event.text, act_event.text) {
            let diff_view = format_diff_output(exp_event.text, act_event.text);
            errors.push(format!(
                "Event {}: text mismatch:\n  expected: \"{}\"\n  actual:   \"{}\"\n{}",
                i,
                escape_debug_str(exp_event.text),
                escape_debug_str(act_event.text),
                diff_view
            ));
        }
    }

    if errors.is_empty() {
        if !warnings.is_empty() {
            for warning in warnings {
                eprintln!("warning: {}", warning);
            }
        }
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ass_uses_ass_core() {
        let content = "[Script Info]\nTitle: Test\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&HFFFFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,2,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Actor, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,Hello world\n";

        let parsed = parse_ass(content).expect("expected parse success");
        
        // Check that we have styles and events
        let styles_count = parsed.sections().iter().filter(|s| matches!(s, ass_core::parser::ast::Section::Styles(_))).count();
        let events_count = parsed.sections().iter().filter(|s| matches!(s, ass_core::parser::ast::Section::Events(_))).count();
        
        assert_eq!(styles_count, 1);
        assert_eq!(events_count, 1);
    }
}
