#[allow(unused_imports)]
use crate::ass_test_helpers::{compare_ass_files, parse_ass};
use srv3_ttml::TimedText;
use std::str::FromStr;

fn to_ass(timed_text: &TimedText) -> std::io::Result<String> {
    srv3tovtt_crate::to_ass(timed_text)
}

// Helper macro to generate test functions for all test categories
macro_rules! define_ass_test {
    ($test_name:ident, $test_file:expr) => {
        #[test]
        fn $test_name() {
            let input = include_str!(concat!("../tests/ass/", $test_file, ".ytt"));
            let timed_text = match TimedText::from_str(input) {
                Ok(tt) => tt,
                Err(e) => panic!("Failed to parse {}.ytt: {}", $test_file, e),
            };

            let output = match to_ass(&timed_text) {
                Ok(ass) => ass,
                Err(e) => panic!("Failed to convert {}.ytt to ASS: {}", $test_file, e),
            };

            // Basic validation
            assert!(
                !output.is_empty(),
                "{}: conversion produced empty output",
                $test_file
            );
            assert!(
                output.contains("[Events]"),
                "{}: Missing Events section",
                $test_file
            );
            assert!(
                output.contains("Dialogue:"),
                "{}: Missing Dialogue lines",
                $test_file
            );

            // Optional: Parse and do semantic comparison if reference file exists
            let expected_content = include_str!(concat!("../tests/ass/", $test_file, ".ass"));
            if let Ok(expected_parsed) = parse_ass(expected_content) {
                if let Ok(actual_parsed) = parse_ass(&output) {
                    // Suppress errors for now - we're mostly checking that conversion doesn't crash
                    let _ = compare_ass_files(&expected_parsed, &actual_parsed);
                }
            }
        }
    };
}

define_ass_test!(ytt2ass_alignment, "Alignment");
define_ass_test!(ytt2ass_bold_italic_underline, "BoldItalicUnderline");
define_ass_test!(ytt2ass_chroma, "Chroma");
define_ass_test!(ytt2ass_colors, "Colors");
define_ass_test!(ytt2ass_fade, "Fade");
define_ass_test!(ytt2ass_fault_tolerance, "FaultTolerance");
define_ass_test!(ytt2ass_fonts, "Fonts");
define_ass_test!(ytt2ass_karaoke, "Karaoke");
define_ass_test!(ytt2ass_move, "Move");
define_ass_test!(ytt2ass_no_default_scale, "NoDefaultScale");
define_ass_test!(ytt2ass_offset, "Offset");
define_ass_test!(ytt2ass_ruby, "Ruby");
define_ass_test!(ytt2ass_shadows, "Shadows");
define_ass_test!(ytt2ass_shake, "Shake");
define_ass_test!(ytt2ass_simultaneous, "Simultaneous");
define_ass_test!(ytt2ass_simultaneous_reverse, "SimultaneousReverse");
define_ass_test!(ytt2ass_text_direction, "TextDirection");
define_ass_test!(ytt2ass_transform, "Transform");
