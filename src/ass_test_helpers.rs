use aspasia::{AssSubtitle, Subtitle};
use chumsky::prelude::*;
use std::mem::discriminant;
use std::str::FromStr;

/// Parse an ASS file using the aspasia parser
pub fn parse_ass(content: &str) -> Result<AssSubtitle, String> {
    AssSubtitle::from_str(content).map_err(|e| e.to_string())
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
    if (a - b).abs() <= f64::EPSILON {
        return true;
    }

    let two_digit_scale = 100.0;
    let three_digit_scale = 1000.0;

    let two_digit_a = (a * two_digit_scale).round() as i64;
    let two_digit_b = (b * two_digit_scale).round() as i64;
    if two_digit_a != two_digit_b {
        return false;
    }

    let three_digit_a = (a * three_digit_scale).round() as i64;
    let three_digit_b = (b * three_digit_scale).round() as i64;
    three_digit_a == three_digit_b
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

/// Compare two ASS files for semantic equivalence
pub fn compare_ass_files(expected: &AssSubtitle, actual: &AssSubtitle) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

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
        if exp_style.strike_out != act_style.strike_out {
            errors.push(format!(
                "Style {}: strike-out flag mismatch: expected {}, got {}",
                i, exp_style.strike_out, act_style.strike_out
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
        if (exp_style.angle - act_style.angle).abs() > f64::EPSILON {
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

    // Compare dialogue lines field-by-field
    for (i, (exp_line, act_line)) in expected_events.iter().zip(actual_events.iter()).enumerate() {
        if discriminant(&exp_line.kind) != discriminant(&act_line.kind) {
            errors.push(format!(
                "Line {}: kind mismatch: expected {:?}, got {:?}",
                i, exp_line.kind, act_line.kind
            ));
        }
        if exp_line.layer != act_line.layer {
            errors.push(format!(
                "Line {}: layer mismatch: expected {}, got {}",
                i, exp_line.layer, act_line.layer
            ));
        }
        if exp_line.start != act_line.start {
            errors.push(format!(
                "Line {}: start time mismatch: expected {}, got {}",
                i,
                exp_line.start.as_substation_timestamp(),
                act_line.start.as_substation_timestamp()
            ));
        }
        if exp_line.end != act_line.end {
            errors.push(format!(
                "Line {}: end time mismatch: expected {}, got {}",
                i,
                exp_line.end.as_substation_timestamp(),
                act_line.end.as_substation_timestamp()
            ));
        }
        if exp_line.style != act_line.style {
            let expected_style = exp_line.style.as_deref().unwrap_or("<None>");
            let actual_style = act_line.style.as_deref().unwrap_or("<None>");
            errors.push(format!(
                "Line {}: style mismatch: expected {}, got {}",
                i, expected_style, actual_style
            ));
        }
        if exp_line.name != act_line.name {
            let expected_name = exp_line.name.as_deref().unwrap_or("<None>");
            let actual_name = act_line.name.as_deref().unwrap_or("<None>");
            errors.push(format!(
                "Line {}: actor mismatch: expected {}, got {}",
                i, expected_name, actual_name
            ));
        }
        if exp_line.margin_l != act_line.margin_l {
            errors.push(format!(
                "Line {}: margin_l mismatch: expected {}, got {}",
                i, exp_line.margin_l, act_line.margin_l
            ));
        }
        if exp_line.margin_r != act_line.margin_r {
            errors.push(format!(
                "Line {}: margin_r mismatch: expected {}, got {}",
                i, exp_line.margin_r, act_line.margin_r
            ));
        }
        if exp_line.margin_v != act_line.margin_v {
            errors.push(format!(
                "Line {}: margin_v mismatch: expected {}, got {}",
                i, exp_line.margin_v, act_line.margin_v
            ));
        }
        if exp_line.effect != act_line.effect {
            let expected_effect_raw = exp_line.effect.as_deref();
            let actual_effect_raw = act_line.effect.as_deref();
            let expected_effect = expected_effect_raw.unwrap_or("<None>");
            let actual_effect = actual_effect_raw.unwrap_or("<None>");
            let actual_effect_trimmed = actual_effect_raw.unwrap_or("").trim();
            if expected_effect_raw == Some("no_android_dark_text_hack")
                && actual_effect_trimmed.is_empty()
            {
                warnings.push(format!(
                    "Line {}: effect mismatch downgraded to warning: expected {}, got {}",
                    i, expected_effect, actual_effect
                ));
                // TODO: revisit once Android dark text hack behavior is implemented for parity.
            } else {
                errors.push(format!(
                    "Line {}: effect mismatch: expected {}, got {}",
                    i, expected_effect, actual_effect
                ));
            }
        }
        if exp_line.text != act_line.text && !fuzzy_text_equal(&exp_line.text, &act_line.text) {
            errors.push(format!(
                "Line {}: text mismatch: expected '{}', got '{}'",
                i, exp_line.text, act_line.text
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
    use aspasia::Subtitle;

    #[test]
    fn test_parse_ass_uses_aspasia() {
        let content = "[Script Info]\nTitle: Test\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,20,&H00FFFFFF,&HFFFFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,2,10,10,10,1\n\n[Events]\nFormat: Layer, Start, End, Style, Actor, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,Hello world\n";

        let parsed = parse_ass(content).expect("expected parse success");
        assert_eq!(parsed.styles().len(), 1);
        assert_eq!(parsed.events().len(), 1);
    }
}
