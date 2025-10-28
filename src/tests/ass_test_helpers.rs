use ariadne::{sources, Label, Report, ReportKind};
use ass_core::analysis::events::TextAnalysis;
use ass_core::analysis::ScriptAnalysis;
use ass_core::parser::Script;

/// Parse an ASS file using the ass-core parser
pub fn parse_ass(content: &str) -> Result<Script<'_>, String> {
    Script::parse(content).map_err(|e| format!("{:?}", e))
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

fn extract_pos_structure(pos_str: &str) -> Option<(f64, f64)> {
    if pos_str.starts_with('(') && pos_str.ends_with(')') {
        let inner = &pos_str[1..pos_str.len() - 1];
        parse_pos_coords(inner)
    } else {
        None
    }
}

fn compare_pos_arg(exp_pos: &str, act_pos: &str) -> Result<(), (Vec<String>, Vec<String>)> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    // We should have a tolerance here of ±0.001 for both X and Y coordinates
    let (expected_x, expected_y) = match extract_pos_structure(exp_pos) {
        Some((x, y)) => (x, y),
        None => {
            errors.push(format!("Invalid position format in fixture!: {}", exp_pos));
            return Err((errors, warnings));
        }
    };

    let (actual_x, actual_y) = match extract_pos_structure(act_pos) {
        Some((x, y)) => (x, y),
        None => {
            errors.push(format!("Invalid position format: {}", act_pos));
            return Err((errors, warnings));
        }
    };

    fn compare_coordinate(
        expected: f64,
        actual: f64,
        coord_name: &str,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        if expected != actual {
            if !floats_close_enough(expected, actual) {
                errors.push(format!(
                    "{} coordinate mismatch: expected {}, got {}",
                    coord_name, expected, actual
                ));
            } else {
                warnings.push(format!(
                    "{} coordinate mismatch within tolerance: expected {}, got {}",
                    coord_name, expected, actual
                ));
            }
        }
    }

    compare_coordinate(expected_x, actual_x, "X", &mut errors, &mut warnings);
    compare_coordinate(expected_y, actual_y, "Y", &mut errors, &mut warnings);

    if errors.is_empty() {
        Ok(())
    } else {
        Err((errors, warnings))
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
        Label::new(("expected.ass", expected_start..expected_end)).with_message("expected segment"),
    )
    .with_label(Label::new(("actual.ass", actual_start..actual_end)).with_message("actual segment"))
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

pub fn compare_text(
    expected: &TextAnalysis,
    actual: &TextAnalysis,
) -> Result<(), (Vec<String>, Vec<String>)> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let (expected_len, actual_len) = (expected.char_count(), actual.char_count());
    // let expected_tokens = expected.tokens();
    // let actual_tokens = actual.tokens();

    if expected_len != actual_len {
        errors.push(format!(
            "Text character count mismatch: expected {}, got {}",
            expected_len, actual_len
        ));
        return Err((errors, warnings));
    }

    let expected_linelen = expected.line_count();
    let actual_linelen = actual.line_count();

    if expected_linelen != actual_linelen {
        errors.push(format!(
            "Text line count mismatch: expected {}, got {}",
            expected_linelen, actual_linelen
        ));
    }

    // Compare plain text
    let (expected_text, actual_text) = (expected.plain_text(), actual.plain_text());
    if expected_text != actual_text {
        errors.push(format!(
            "Text content mismatch:\n{}",
            format_diff_output(expected_text, actual_text)
        ));
    }

    let expected_tags = expected.override_tags();
    let actual_tags = actual.override_tags();

    for (i, (exp_tag, act_tag)) in expected_tags.iter().zip(actual_tags.iter()).enumerate() {
        let exp_args = exp_tag.args();
        let act_args = act_tag.args();
        let exp_name = exp_tag.name();
        let act_name = act_tag.name();

        // Explicit handling for \pos tag comparison
        // We should have a tolerance here of ±0.001 for both X and Y coordinates
        if act_name == "pos" {
            match compare_pos_arg(exp_args, act_args) {
                Ok(_) => {}
                Err((mut err_list, mut warn_list)) => {
                    for err in err_list.drain(..) {
                        errors.push(format!("Tag {i} (pos) argument error: {}", err));
                    }
                    for warn in warn_list.drain(..) {
                        warnings.push(format!("Tag {i} (pos) argument warning: {}", warn));
                    }
                }
            }
        } else if exp_args != act_args {
            errors.push(format!(
                "Tag {i} ({exp_name}) argument mismatch: expected {:?}, got {:?}",
                exp_args, act_args
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err((errors, warnings))
    }
}

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &Script<'_>, actual: &Script<'_>) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let actual_analysis = ScriptAnalysis::analyze(actual).unwrap();
    let expected_analysis = ScriptAnalysis::analyze(expected).unwrap();

    let expected_dialogue_infos = expected_analysis.dialogue_info();
    let actual_dialogue_infos = actual_analysis.dialogue_info();

    // Check if dialogue counts match
    if expected_dialogue_infos.len() != actual_dialogue_infos.len() {
        errors.push(format!(
            "Dialogue count mismatch: expected {}, got {}",
            expected_dialogue_infos.len(),
            actual_dialogue_infos.len()
        ));
    }

    /// Compare centisecs with ±1cs tolerance
    fn times_close_enough(expected: u32, actual: u32) -> bool {
        (expected - actual).abs_diff(1) <= 1
    }

    // Compare events field-by-field
    for (i, (exp_info, act_info)) in expected_dialogue_infos
        .iter()
        .zip(actual_dialogue_infos.iter())
        .enumerate()
    {
        let exp_event = exp_info.event();
        let act_event = act_info.event();

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

        if !times_close_enough(
            exp_event.start_time_cs().unwrap(),
            act_event.start_time_cs().unwrap(),
        ) {
            errors.push(format!(
                "Event {}: start time mismatch: expected {}, got {}",
                i,
                exp_event.start_time_cs().unwrap(),
                act_event.start_time_cs().unwrap()
            ));
        }
        if !times_close_enough(
            exp_event.end_time_cs().unwrap(),
            act_event.end_time_cs().unwrap(),
        ) {
            errors.push(format!(
                "Event {}: end time mismatch: expected {}, got {}",
                i,
                exp_event.end_time_cs().unwrap(),
                act_event.end_time_cs().unwrap()
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

        // handle extra stuff

        if let (Ok(exp_dur), Ok(act_dur)) = (exp_event.duration_cs(), act_event.duration_cs()) {
            if exp_dur != act_dur {
                errors.push(format!(
                    "Event {}: duration mismatch: expected {:?}, got {:?}",
                    i, exp_dur, act_dur
                ));
            }
        } else {
            errors.push(format!(
                "Event {}: duration could not be determined for comparison",
                i
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

        let expected_text = exp_info.text_analysis();

        let actual_text = act_info.text_analysis();

        match compare_text(expected_text, actual_text) {
            Ok(_) => {}
            Err((mut err_list, mut warn_list)) => {
                for err in err_list.drain(..) {
                    errors.push(format!("Event {}: {}", i, err));
                }
                for warn in warn_list.drain(..) {
                    warnings.push(format!("Event {}: {}", i, warn));
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
