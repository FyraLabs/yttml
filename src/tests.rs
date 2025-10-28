use crate::ass_test_helpers::{compare_ass_files, parse_ass};
use aspasia::Subtitle;
use srv3_ttml::TimedText;
#[allow(unused_imports)]
use std::str::FromStr;

fn to_ass(timed_text: &TimedText) -> std::io::Result<String> {
    srv3tovtt_crate::to_ass(timed_text)
}

const STAGE_PARSE_SRV3: &str = "parse-srv3";
const STAGE_CONVERT_TO_ASS: &str = "srv3-to-ass";
const STAGE_PARSE_EXPECTED_ASS: &str = "parse-expected-ass";
const STAGE_PARSE_ACTUAL_ASS: &str = "parse-actual-ass";
const STAGE_COMPARE_ASS: &str = "compare-ass";

// Helper macro to generate test functions for all test categories
// This performs YTT→ASS conversion and compares the result with the expected ASS file
macro_rules! define_ass_test {
    ($test_name:ident, $test_file:expr) => {
        #[test]
        fn $test_name() {
            let input = include_str!(concat!("../tests/ass/", $test_file, ".ytt"));
            let timed_text = match TimedText::from_str(input) {
                Ok(tt) => tt,
                Err(e) => panic!(
                    "[round-trip stage: {}] Failed to parse {}.ytt: {}",
                    STAGE_PARSE_SRV3, $test_file, e
                ),
            };

            let actual_ass = match to_ass(&timed_text) {
                Ok(ass) => ass,
                Err(e) => panic!(
                    "[round-trip stage: {}] Failed to convert {}.ytt to ASS: {}",
                    STAGE_CONVERT_TO_ASS, $test_file, e
                ),
            };

            // Basic validation
            assert!(
                !actual_ass.is_empty(),
                "{}: conversion produced empty output",
                $test_file
            );
            assert!(
                actual_ass.contains("[Events]"),
                "{}: Missing Events section",
                $test_file
            );
            assert!(
                actual_ass.contains("Dialogue:"),
                "{}: Missing Dialogue lines",
                $test_file
            );

            // Parse expected and actual ASS files for semantic comparison
            let expected_content =
                include_str!(concat!("../tests/ass/", $test_file, ".reverse.ass"));

            let expected_parsed = parse_ass(expected_content).unwrap_or_else(|e| {
                panic!(
                    "[round-trip stage: {}] {}: Failed to parse expected ASS file: {}",
                    STAGE_PARSE_EXPECTED_ASS, $test_file, e
                )
            });

            let actual_parsed = parse_ass(&actual_ass).unwrap_or_else(|e| {
                panic!(
                    "[round-trip stage: {}] {}: Failed to parse actual ASS output: {}",
                    STAGE_PARSE_ACTUAL_ASS, $test_file, e
                )
            });

            // Compare and report differences
            if let Err(diffs) = compare_ass_files(&expected_parsed, &actual_parsed) {
                eprintln!(
                    "\n[round-trip stage: {}] {} CONVERSION DIFFERENCES:",
                    STAGE_COMPARE_ASS, $test_file
                );
                for diff in &diffs {
                    eprintln!("  - {}", diff);
                }
                eprintln!(
                    "\nExpected {} dialogue lines, got {}",
                    expected_parsed.events().len(),
                    actual_parsed.events().len()
                );
                eprintln!(
                    "Expected {} styles, got {}",
                    expected_parsed.styles().len(),
                    actual_parsed.styles().len()
                );

                // Fail the test with detailed error message
                panic!(
                    "\n[round-trip stage: {}] {} conversion produced {} differences:\n{}",
                    STAGE_COMPARE_ASS,
                    $test_file,
                    diffs.len(),
                    diffs.join("\n")
                );
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
