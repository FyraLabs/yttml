use std::collections::HashMap;

/// Represents a parsed ASS file with its sections
#[derive(Debug, Clone)]
pub struct AssFile {
    #[allow(dead_code)]
    pub script_info: HashMap<String, String>,
    pub styles: Vec<AssStyle>,
    pub events: Vec<AssDialogue>,
}

/// Represents an ASS style definition
#[derive(Debug, Clone, PartialEq)]
pub struct AssStyle {
    pub name: String,
    pub fontname: String,
    pub fontsize: String,
    pub alignment: String,
}

/// Represents an ASS dialogue/event line
#[derive(Debug, Clone, PartialEq)]
pub struct AssDialogue {
    pub start: String,
    pub end: String,
    pub style: String,
    pub text: String,
}

/// Parse an ASS file into structured format
pub fn parse_ass(content: &str) -> Result<AssFile, String> {
    let mut script_info = HashMap::new();
    let mut styles = Vec::new();
    let mut events = Vec::new();

    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments (but not dialogue lines starting with ;)
        if trimmed.is_empty() || (trimmed.starts_with(';') && !trimmed.starts_with("Dialogue:")) {
            continue;
        }

        // Section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed;
            continue;
        }

        // Parse based on section
        match current_section {
            "[Script Info]" => {
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = trimmed[..colon_pos].trim().to_string();
                    let value = trimmed[colon_pos + 1..].trim().to_string();
                    script_info.insert(key, value);
                }
            }
            "[V4+ Styles]" => {
                if trimmed.starts_with("Style:") {
                    if let Some(style) = parse_style_line(trimmed) {
                        styles.push(style);
                    }
                }
            }
            "[Events]" => {
                if trimmed.starts_with("Dialogue:") {
                    if let Some(dialogue) = parse_dialogue_line(trimmed) {
                        events.push(dialogue);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(AssFile {
        script_info,
        styles,
        events,
    })
}

/// Parse a single style line
fn parse_style_line(line: &str) -> Option<AssStyle> {
    let content = line.strip_prefix("Style:")?.trim();
    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();

    if parts.len() < 2 {
        return None;
    }

    // Find alignment (typically at index 17 if all fields present)
    let alignment = if parts.len() > 17 {
        parts[17].to_string()
    } else {
        "2".to_string() // Default alignment
    };

    Some(AssStyle {
        name: parts[0].to_string(),
        fontname: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
        fontsize: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
        alignment,
    })
}

/// Parse a single dialogue line
fn parse_dialogue_line(line: &str) -> Option<AssDialogue> {
    let content = line.strip_prefix("Dialogue:")?.trim();

    // Dialogue format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
    // We need to be careful with commas in the text part
    let parts: Vec<&str> = content.split(',').collect();

    if parts.len() < 10 {
        return None;
    }

    let start = parts[1].trim().to_string();
    let end = parts[2].trim().to_string();
    let style = parts[3].trim().to_string();

    // The text is everything after the 9th comma
    let text_start = content.find(',').and_then(|_| {
        let mut count = 0;
        for (i, ch) in content.chars().enumerate() {
            if ch == ',' {
                count += 1;
                if count == 9 {
                    return Some(i + 1);
                }
            }
        }
        None
    })?;

    let text = content[text_start..].trim().to_string();

    Some(AssDialogue {
        start,
        end,
        style,
        text,
    })
}

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &AssFile, actual: &AssFile) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Check if dialogue counts match
    if expected.events.len() != actual.events.len() {
        errors.push(format!(
            "Dialogue count mismatch: expected {}, got {}",
            expected.events.len(),
            actual.events.len()
        ));
    }

    // Check if style counts match
    if expected.styles.len() != actual.styles.len() {
        errors.push(format!(
            "Style count mismatch: expected {}, got {}",
            expected.styles.len(),
            actual.styles.len()
        ));
    }

    // Compare dialogue lines
    for (i, (exp_line, act_line)) in expected.events.iter().zip(actual.events.iter()).enumerate() {
        if exp_line.start != act_line.start {
            errors.push(format!(
                "Line {}: Start time mismatch: expected {}, got {}",
                i, exp_line.start, act_line.start
            ));
        }
        if exp_line.end != act_line.end {
            errors.push(format!(
                "Line {}: End time mismatch: expected {}, got {}",
                i, exp_line.end, act_line.end
            ));
        }
        if exp_line.style != act_line.style {
            errors.push(format!(
                "Line {}: Style mismatch: expected {}, got {}",
                i, exp_line.style, act_line.style
            ));
        }
        // Text comparison is more lenient due to formatting differences
        if clean_text(&exp_line.text) != clean_text(&act_line.text) {
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

    #[test]
    fn test_parse_style_line() {
        let line = "Style: Default,Arial,20,&H00FFFFFF,&HFFFFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,3,2,0,2,10,10,10,1";
        let style = parse_style_line(line).unwrap();
        assert_eq!(style.name, "Default");
        assert_eq!(style.fontname, "Arial");
        assert_eq!(style.fontsize, "20");
    }

    #[test]
    fn test_parse_dialogue_line() {
        let line = "Dialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,Hello world";
        let dialogue = parse_dialogue_line(line).unwrap();
        assert_eq!(dialogue.start, "0:00:00.00");
        assert_eq!(dialogue.end, "0:00:05.00");
        assert_eq!(dialogue.style, "Default");
        assert_eq!(dialogue.text, "Hello world");
    }
}
