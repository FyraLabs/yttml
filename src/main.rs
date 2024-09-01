use std::str::FromStr;

use clap::Parser;

#[derive(Parser)]
#[clap(version, about, long_about = "A tool to parse and process YouTube SRV3 captions")]
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
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.subcmd {
        SubCommand::Parse { input, format } => {
            println!("Parsing file: {}", input);
            
            let file = std::fs::read_to_string(input)?;
     
            
            let captions = srv3_ttml::TimedText::from_str(&file)?;
            
            // println!("Parsed captions: {:?}", captions);
            
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&captions)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yml::to_string(&captions)?);
                }
            }
            
        }
    }
    Ok(())
}

