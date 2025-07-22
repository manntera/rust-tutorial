use anyhow::Result;
use clap::Parser;
use image_dedup::cli::{Cli, Commands};
use image_dedup::cli::commands;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Scan { target_directory, output, threads, force } => {
            commands::execute_scan(target_directory, output, threads, force).await?;
        }
        Commands::FindDups { hash_database, output, threshold } => {
            commands::execute_find_dups(hash_database, output, threshold).await?;
        }
        Commands::Process { duplicate_list, action, dest, no_confirm } => {
            commands::execute_process(duplicate_list, action, dest, no_confirm).await?;
        }
    }
    
    Ok(())
}