use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Split file to open. If not given, takes it from config file (the last one used)
    pub splits: Option<PathBuf>,

    /// Autosplitter file to open. If not given, takes it from config file (the last one used)
    #[arg(short, long, value_name = "FILE")]
    pub autosplitter: Option<PathBuf>,

    /// Layout file to open. If not given, takes it from config file (the last one used)
    #[arg(short, long, value_name = "FILE")]
    pub layout: Option<PathBuf>,
}
