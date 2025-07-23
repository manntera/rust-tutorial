use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "image_dedup")]
#[command(about = "A tool for finding and managing duplicate images")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan a directory for images and generate hash database
    Scan {
        /// Target directory to scan
        target_directory: PathBuf,

        /// Output file path for hash database
        #[arg(short, long, default_value = "hashes.json")]
        output: PathBuf,

        /// Number of threads to use for parallel processing
        #[arg(short, long)]
        threads: Option<usize>,

        /// Force overwrite existing output file without warning
        #[arg(short, long)]
        force: bool,

        /// Hash algorithm to use
        #[arg(short = 'a', long, default_value = "dct")]
        algorithm: String,

        /// Hash size (typically 8, 16, or 32)
        #[arg(long, default_value = "8")]
        hash_size: u32,

        /// Configuration file path (JSON format)
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,
    },

    /// Find duplicate images using hash database
    FindDups {
        /// Hash database file
        #[arg(default_value = "hashes.json")]
        hash_database: PathBuf,

        /// Output file path for duplicate list
        #[arg(short, long, default_value = "duplicates.json")]
        output: PathBuf,

        /// Maximum Hamming distance for duplicates
        #[arg(short, long, default_value = "5")]
        threshold: u32,
    },

    /// Process duplicate images (move or delete)
    Process {
        /// Duplicate list file
        #[arg(default_value = "duplicates.json")]
        duplicate_list: PathBuf,

        /// Action to perform: move or delete
        #[arg(short, long, default_value = "move")]
        action: ProcessAction,

        /// Destination directory for moved files
        #[arg(short, long, default_value = "./duplicates")]
        dest: PathBuf,

        /// Skip confirmation prompt
        #[arg(long)]
        no_confirm: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ProcessAction {
    Move,
    Delete,
}
