use crate::ass_test_helpers::{compare_ass_files, parse_ass};
use srv3_ttml::TimedText;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

fn to_ass(timed_text: &TimedText) -> std::io::Result<String> {
    srv3tovtt_crate::to_ass(timed_text)
}

pub fn run_all_tests_with_output() {
    let test_files = [
        "Alignment",
        "BoldItalicUnderline",
        "Chroma",
        "Colors",
        "Fade",
        "FaultTolerance",
        "Fonts",
        "Karaoke",
        "Move",
        "NoDefaultScale",
        "Offset",
        "Ruby",
        "Shadows",
        "Shake",
        "Simultaneous",
        "SimultaneousReverse",
        "TextDirection",
        "Transform",
    ];

    // Create output directory
    let output_dir = Path::new("test_output");
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    let mut summary = String::new();
    summary.push_str("# YTT to ASS Conversion Test Results\n\n");

    let mut passed = 0;
    let mut failed = 0;

    for test_name in &test_files {
        println!("Testing {}...", test_name);

        match run_single_test(test_name, output_dir) {
            Ok(diffs) => {
                if diffs.is_empty() {
                    passed += 1;
                    summary.push_str(&format!("✅ **{}**: PASSED\n", test_name));
                } else {
                    failed += 1;
                    summary.push_str(&format!(
                        "❌ **{}**: FAILED ({} differences)\n",
                        test_name,
                        diffs.len()
                    ));

                    // Write detailed differences to file
                    let diff_file = output_dir.join(format!("{}_diff.txt", test_name));
                    let mut f = File::create(&diff_file).unwrap();
                    writeln!(f, "Test: {}", test_name).unwrap();
                    writeln!(f, "Differences found: {}\n", diffs.len()).unwrap();
                    for (i, diff) in diffs.iter().enumerate() {
                        writeln!(f, "{}. {}", i + 1, diff).unwrap();
                    }
                    println!(
                        "  ❌ {} differences (see {}_diff.txt)",
                        diffs.len(),
                        test_name
                    );
                }
            }
            Err(e) => {
                failed += 1;
                summary.push_str(&format!("💥 **{}**: ERROR - {}\n", test_name, e));
                println!("  💥 ERROR: {}", e);
            }
        }
    }

    summary.push_str(&format!("\n## Summary\n"));
    summary.push_str(&format!("- **Passed**: {}\n", passed));
    summary.push_str(&format!("- **Failed**: {}\n", failed));
    summary.push_str(&format!("- **Total**: {}\n", test_files.len()));
    summary.push_str(&format!(
        "- **Success Rate**: {:.1}%\n",
        (passed as f64 / test_files.len() as f64) * 100.0
    ));

    // Write summary
    let summary_file = output_dir.join("summary.md");
    fs::write(&summary_file, summary).expect("Failed to write summary");

    println!("\n{} tests passed, {} tests failed", passed, failed);
    println!("Results written to test_output/");
}

fn run_single_test(test_name: &str, output_dir: &Path) -> Result<Vec<String>, String> {
    // Read input YTT file
    let ytt_path = format!("tests/ass/{}.ytt", test_name);
    let ytt_content =
        fs::read_to_string(&ytt_path).map_err(|e| format!("Failed to read {}: {}", ytt_path, e))?;

    // Parse YTT
    let timed_text =
        TimedText::from_str(&ytt_content).map_err(|e| format!("Failed to parse YTT: {}", e))?;

    // Convert to ASS
    let actual_ass = to_ass(&timed_text).map_err(|e| format!("Failed to convert to ASS: {}", e))?;

    // Write actual output
    let actual_file = output_dir.join(format!("{}_actual.ass", test_name));
    fs::write(&actual_file, &actual_ass)
        .map_err(|e| format!("Failed to write actual output: {}", e))?;

    // Read expected ASS file
    let expected_path = format!("tests/ass/{}.ass", test_name);
    let expected_content = fs::read_to_string(&expected_path)
        .map_err(|e| format!("Failed to read {}: {}", expected_path, e))?;

    // Copy expected to output dir for easy comparison
    let expected_file = output_dir.join(format!("{}_expected.ass", test_name));
    fs::write(&expected_file, &expected_content)
        .map_err(|e| format!("Failed to write expected output: {}", e))?;

    // Parse both ASS files
    let expected_parsed =
        parse_ass(&expected_content).map_err(|e| format!("Failed to parse expected ASS: {}", e))?;

    let actual_parsed =
        parse_ass(&actual_ass).map_err(|e| format!("Failed to parse actual ASS: {}", e))?;

    // Compare
    match compare_ass_files(&expected_parsed, &actual_parsed) {
        Ok(_) => Ok(Vec::new()),
        Err(diffs) => Ok(diffs),
    }
}

// #[test]
// fn generate_test_comparison_files() {
//     run_all_tests_with_output();
// }
