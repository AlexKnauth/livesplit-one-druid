use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Split file to open. If not given, opens the last split file used
    pub split_file: Option<PathBuf>,
}
