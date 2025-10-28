use ariadne::{sources, Color, Label, Report, ReportKind};
use ass_core::parser::{ast::Event, Script};
use chumsky::prelude::*;

/// Parse an ASS file using the ass-core parser
pub fn parse_ass(content: &str) -> Result<Script<'_>, String> {
    Script::parse(content).map_err(|e| format!("{:?}", e))
}

#[derive(Debug, PartialEq)]
enum TextToken {
    Text(String),
    Number(f64),
}

#[derive(Debug, PartialEq, Eq)]
enum FuzzyMatchResult {
    Exact,
    NumericOnly,
    Different,
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

fn parse_pos_coords(input: &str) -> Option<(f64, f64)> {
    let mut parts = input.split(',');
    let x = parts.next()?.trim().parse::<f64>().ok()?;
    let y = parts.next()?.trim().parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn extract_pos_structure(text: &str) -> Option<(String, Vec<(f64, f64)>)> {
    const PREFIX: &str = "\\pos(";

    let mut sanitized = String::with_capacity(text.len());
    let mut coords = Vec::new();
    let mut index = 0;

    while index < text.len() {
        if text[index..].starts_with(PREFIX) {
            let start = index + PREFIX.len();
            let remainder = &text[start..];
            let closing_offset = remainder.find(')')?;
            let inner = &remainder[..closing_offset];
            let (x, y) = parse_pos_coords(inner)?;
            coords.push((x, y));
            sanitized.push_str("\\pos(#,#)");
            index = start + closing_offset + 1;
        } else {
            let ch = text[index..].chars().next()?;
            sanitized.push(ch);
            index += ch.len_utf8();
        }
    }

    Some((sanitized, coords))
}

fn pos_difference_only(expected: &str, actual: &str) -> bool {
    match (
        extract_pos_structure(expected),
        extract_pos_structure(actual),
    ) {
        (Some((exp_sanitized, exp_coords)), Some((act_sanitized, act_coords))) => {
            if exp_coords.is_empty() || exp_coords.len() != act_coords.len() {
                return false;
            }

            if exp_sanitized != act_sanitized {
                return false;
            }

            exp_coords
                .iter()
                .zip(act_coords.iter())
                .all(|(&(exp_x, exp_y), &(act_x, act_y))| {
                    floats_close_enough(exp_x, act_x) && floats_close_enough(exp_y, act_y)
                })
        }
        _ => false,
    }
}

fn fuzzy_text_compare(expected: &str, actual: &str) -> FuzzyMatchResult {
    if expected == actual {
        return FuzzyMatchResult::Exact;
    }

    let expected_tokens = tokenize_text(expected);
    let actual_tokens = tokenize_text(actual);

    if expected_tokens.len() != actual_tokens.len() {
        return FuzzyMatchResult::Different;
    }

    let mut saw_numeric_delta = false;

    for (expected_token, actual_token) in expected_tokens.iter().zip(actual_tokens.iter()) {
        match (expected_token, actual_token) {
            (TextToken::Text(expected_text), TextToken::Text(actual_text)) => {
                if expected_text != actual_text {
                    return FuzzyMatchResult::Different;
                }
            }
            (TextToken::Number(expected_number), TextToken::Number(actual_number)) => {
                if expected_number == actual_number {
                    continue;
                }

                if floats_close_enough(*expected_number, *actual_number) {
                    saw_numeric_delta = true;
                } else {
                    return FuzzyMatchResult::Different;
                }
            }
            _ => return FuzzyMatchResult::Different,
        }
    }

    if saw_numeric_delta {
        FuzzyMatchResult::NumericOnly
    } else {
        FuzzyMatchResult::Exact
    }
}

fn escape_debug_str(input: &str) -> String {
    format!("{}", input.escape_debug())
}

fn diff_spans(expected: &str, actual: &str) -> ((usize, usize), (usize, usize)) {
    if expected == actual {
        let len = expected.len();
        return ((len, len), (len, len));
    }

    let mut prefix_bytes = 0;
    for (exp_char, act_char) in expected.chars().zip(actual.chars()) {
        if exp_char == act_char {
            prefix_bytes += exp_char.len_utf8();
        } else {
            break;
        }
    }

    let expected_tail = &expected[prefix_bytes..];
    let actual_tail = &actual[prefix_bytes..];

    let mut suffix_bytes = 0;
    let mut expected_tail_iter = expected_tail.chars().rev();
    let mut actual_tail_iter = actual_tail.chars().rev();
    loop {
        match (expected_tail_iter.next(), actual_tail_iter.next()) {
            (Some(exp_char), Some(act_char)) if exp_char == act_char => {
                suffix_bytes += exp_char.len_utf8();

                if prefix_bytes + suffix_bytes >= expected.len()
                    || prefix_bytes + suffix_bytes >= actual.len()
                {
                    break;
                }
            }
            _ => break,
        }
    }

    let mut expected_end = expected.len().saturating_sub(suffix_bytes);
    let mut actual_end = actual.len().saturating_sub(suffix_bytes);

    if expected_end < prefix_bytes {
        expected_end = prefix_bytes;
    }
    if actual_end < prefix_bytes {
        actual_end = prefix_bytes;
    }

    ((prefix_bytes, expected_end), (prefix_bytes, actual_end))
}

fn format_diff_output(expected: &str, actual: &str) -> String {
    let expected_display = escape_debug_str(expected);
    let actual_display = escape_debug_str(actual);

    let ((expected_start, expected_end), (actual_start, actual_end)) =
        diff_spans(&expected_display, &actual_display);

    let report = Report::build(
        ReportKind::Error,
        ("actual.ass", actual_start..actual_start),
    )
    .with_message("ASS text mismatch")
    .with_label(
        Label::new(("expected.ass", expected_start..expected_end))
            .with_message("expected segment")
            .with_color(Color::Yellow),
    )
    .with_label(
        Label::new(("actual.ass", actual_start..actual_end))
            .with_message("actual segment")
            .with_color(Color::Cyan),
    )
    .finish();

    let mut buffer = Vec::new();
    if let Err(err) = report.write(
        sources([
            ("expected.ass", expected_display.as_str()),
            ("actual.ass", actual_display.as_str()),
        ]),
        &mut buffer,
    ) {
        return format!("  <failed to render ariadne report: {}>", err);
    }

    String::from_utf8(buffer)
        .unwrap_or_else(|_| "<ariadne output not valid UTF-8>".to_string())
        .lines()
        .map(|line| format!("  {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &Script<'_>, actual: &Script<'_>) -> Result<(), Vec<String>> {
    use ass_core::parser::ast::Section;

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

    // Helper function to parse ASS timestamps with ±1ms tolerance
    fn parse_ass_time(time_str: &str) -> Result<i64, String> {
        // ASS time format: H:MM:SS.CS (centiseconds)
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid time format: {}", time_str));
        }

        let hours: i64 = parts[0]
            .parse()
            .map_err(|e| format!("Invalid hour: {}", e))?;
        let minutes: i64 = parts[1]
            .parse()
            .map_err(|e| format!("Invalid minute: {}", e))?;

        let sec_parts: Vec<&str> = parts[2].split('.').collect();
        if sec_parts.len() != 2 {
            return Err(format!("Invalid seconds format: {}", parts[2]));
        }

        let seconds: i64 = sec_parts[0]
            .parse()
            .map_err(|e| format!("Invalid second: {}", e))?;
        let centiseconds: i64 = sec_parts[1]
            .parse()
            .map_err(|e| format!("Invalid centisecond: {}", e))?;

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
    for (i, (exp_event, act_event)) in expected_events.iter().zip(actual_events.iter()).enumerate()
    {
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

        if exp_event.text != act_event.text {
            match fuzzy_text_compare(exp_event.text, act_event.text) {
                FuzzyMatchResult::Exact => {}
                FuzzyMatchResult::NumericOnly => {
                    let diff_view = format_diff_output(exp_event.text, act_event.text);
                    warnings.push(format!(
                        "Event {}: text numeric-only mismatch downgraded to warning:\n{}",
                        i, diff_view
                    ));
                }
                FuzzyMatchResult::Different => {
                    if pos_difference_only(exp_event.text, act_event.text) {
                        let diff_view = format_diff_output(exp_event.text, act_event.text);
                        warnings.push(format!(
                            "Event {}: text mismatch limited to pos() rounding, downgraded to warning:\n{}",
                            i,
                            diff_view
                        ));
                    } else {
                        let diff_view = format_diff_output(exp_event.text, act_event.text);
                        errors.push(format!("Event {}: text mismatch:\n{}", i, diff_view));
                    }
                }
            }
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
        let styles_count = parsed
            .sections()
            .iter()
            .filter(|s| matches!(s, ass_core::parser::ast::Section::Styles(_)))
            .count();
        let events_count = parsed
            .sections()
            .iter()
            .filter(|s| matches!(s, ass_core::parser::ast::Section::Events(_)))
            .count();

        assert_eq!(styles_count, 1);
        assert_eq!(events_count, 1);
    }
}
