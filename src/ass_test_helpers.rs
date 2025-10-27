use aspasia::{AssSubtitle, Subtitle, TextEvent};
use std::str::FromStr;

/// Parse an ASS file using the aspasia parser
pub fn parse_ass(content: &str) -> Result<AssSubtitle, String> {
    AssSubtitle::from_str(content).map_err(|e| e.to_string())
}

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &AssSubtitle, actual: &AssSubtitle) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    let expected_events = expected.events();
    let actual_events = actual.events();

    // Check if dialogue counts match
    if expected_events.len() != actual_events.len() {
        errors.push(format!(
            "Dialogue count mismatch: expected {}, got {}",
            expected_events.len(),
            actual_events.len()
        ));
    }

    let expected_styles = expected.styles();
    let actual_styles = actual.styles();

    // Check if style counts match
    if expected_styles.len() != actual_styles.len() {
        errors.push(format!(
            "Style count mismatch: expected {}, got {}",
            expected_styles.len(),
            actual_styles.len()
        ));
    }

    // Compare dialogue lines
    for (i, (exp_line, act_line)) in expected_events.iter().zip(actual_events.iter()).enumerate() {
        if exp_line.start != act_line.start {
            errors.push(format!(
                "Line {}: Start time mismatch: expected {}, got {}",
                i,
                exp_line.start.as_substation_timestamp(),
                act_line.start.as_substation_timestamp()
            ));
        }
        if exp_line.end != act_line.end {
            errors.push(format!(
                "Line {}: End time mismatch: expected {}, got {}",
                i,
                exp_line.end.as_substation_timestamp(),
                act_line.end.as_substation_timestamp()
            ));
        }
        if exp_line.style != act_line.style {
            let expected_style = exp_line.style.as_deref().unwrap_or("<None>");
            let actual_style = act_line.style.as_deref().unwrap_or("<None>");
            errors.push(format!(
                "Line {}: Style mismatch: expected {}, got {}",
                i, expected_style, actual_style
            ));
        }
        // Compare text using aspasia's formatting-stripping helpers to avoid false positives
        let expected_plain = exp_line.unformatted_text();
        let actual_plain = act_line.unformatted_text();
        if clean_text(expected_plain.as_ref()) != clean_text(actual_plain.as_ref()) {
            errors.push(format!(
                "Line {}: Text mismatch: expected '{}', got '{}'",
                i, exp_line.text, act_line.text
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Clean text for comparison (remove extra spaces, normalize)
fn clean_text(text: &str) -> String {
    text.trim()
        .replace("\\N", "\n")
        .replace("\\n", "\n")
        .replace("\\h", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspasia::Subtitle;

    #[test]
    fn test_parse_ass_uses_aspasia() {
        let content = "[Script Info]\nTitle: Test\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&HFFFFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,2,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Actor, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,Hello world\n";

        let parsed = parse_ass(content).expect("expected parse success");
        assert_eq!(parsed.styles().len(), 1);
        assert_eq!(parsed.events().len(), 1);
    }
}
