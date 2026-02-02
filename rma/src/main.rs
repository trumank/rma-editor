use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rma::AppMode;

mod app;

#[derive(Parser)]
#[command(name = "rma")]
#[command(about = "RMA Editor - edit and convert Deep Rock Galactic room assets")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to .uasset or .json file to edit
    #[arg(value_name = "FILE")]
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the editor with a .uasset or .json file
    Edit {
        /// Path to file
        path: PathBuf,
    },
    /// Convert a .uasset to JSON
    ToJson {
        /// Input .uasset file
        input: PathBuf,
        /// Output .json file (defaults to input with .json extension)
        output: Option<PathBuf>,
    },
    /// Convert a JSON file back to .uasset
    FromJson {
        /// Input .json file
        input: PathBuf,
        /// Output .uasset file (defaults to input with .uasset extension)
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Edit { path }) => app::run(AppMode::Editor {
            path: path.to_string_lossy().into_owned(),
        }),
        Some(Commands::ToJson { input, output }) => {
            let output = output.unwrap_or_else(|| input.with_extension("json"));
            let room = rma::load_room(&input)?;
            rma::save_room(&output, &room)?;
            eprintln!("Wrote {}", output.display());
            Ok(())
        }
        Some(Commands::FromJson { input, output }) => {
            let output = output.unwrap_or_else(|| input.with_extension("uasset"));
            let room = rma::load_room(&input)?;
            rma::save_room(&output, &room)?;
            eprintln!("Wrote {}", output.display());
            Ok(())
        }
        None => {
            if let Some(path) = cli.path {
                app::run(AppMode::Editor {
                    path: path.to_string_lossy().into_owned(),
                })
            } else {
                eprintln!("Usage: rma <FILE> or rma <COMMAND>");
                eprintln!("Try 'rma --help' for more information.");
                std::process::exit(1);
            }
        }
    }
}
