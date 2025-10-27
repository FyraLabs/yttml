use clap::Parser;
use std::fs::File;
use std::io::Write;
use std::path::Path;
#[cfg(test)]
mod ass_test_helpers;
#[cfg(test)]
mod test_runner;
#[cfg(test)]
mod tests;

#[derive(Parser)]
#[clap(
    version,
    about,
    long_about = "A tool to parse and process YouTube SRV3 captions"
)]
struct Args {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

/// Serialization format for the output
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum OutputFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// VTT format
    Vtt,
    /// SRT format
    Srt,
    /// ASS format
    Ass,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum SaveLocation {
    /// Print the converted subtitles to stdout
    Stdout,
    /// Save the converted subtitles to a file
    File,
}

// subcommands
#[derive(Parser)]
enum SubCommand {
    /// Parse and attempt to serialize the input file.
    /// Used for verification and debugging.
    Parse {
        // positional
        /// Path to the input file
        input: String,

        /// Output format
        #[clap(short, long, default_value = "json")]
        format: OutputFormat,

        /// Save to file or print to stdout
        #[clap(short, long, default_value = "stdout")]
        save: SaveLocation,

        /// File path to save the output file to
        #[clap(short, long, default_value = "")]
        output: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.subcmd {
        SubCommand::Parse {
            input,
            format,
            save,
            output,
        } => {
            println!("Parsing file: {}", input);

            let file = std::fs::read_to_string(&input)?;

            let captions = srv3_ttml::TimedText::from_str(&file)?;

            // println!("Parsed captions: {:?}", captions);

            let w = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&captions)?,
                OutputFormat::Yaml => serde_yml::to_string(&captions)?,
                OutputFormat::Vtt => srv3tovtt_crate::to_vtt(&captions)?.to_string(),
                OutputFormat::Srt => srv3tovtt_crate::to_srt(&captions)?.to_string(),
                OutputFormat::Ass => srv3tovtt_crate::to_ass(&captions)?,
            };
            match save {
                SaveLocation::Stdout => println!("{}", w),
                SaveLocation::File => {
                    if output.is_empty() {
                        let file_stem = Path::new(&input)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default();
                        let extension = format!("{:?}", format).to_lowercase();
                        let mut outputfile =
                            File::create(format!("./{}.{}", file_stem, extension))?; //if the user doesnt specify the output directory, but wants to save to file, output it with the same name and correct extension to working directory.
                        writeln!(&mut outputfile, "{}", w)?;
                        println!(
                            "Successfully wrote subtitles to ./{}.{}",
                            file_stem, extension
                        );
                    } else {
                        let mut outputfile = File::create(&output)?;
                        writeln!(&mut outputfile, "{}", w)?;
                        println!("Successfully wrote subtitles to {}", output);
                    }
                }
            }
        }
    }
    Ok(())
}
