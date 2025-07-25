use anyhow::Result;
use clap::Parser;
use image_dedup::cli::commands;
use image_dedup::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            target_directory,
            output,
            threads,
            force,
            algorithm,
            hash_size,
            config_preset,
        } => {
            commands::execute_scan(
                target_directory,
                output,
                threads,
                force,
                algorithm,
                hash_size,
                config_preset,
            )
            .await?;
        }
        Commands::FindDups {
            hash_database,
            output,
            threshold,
        } => {
            commands::execute_find_dups(hash_database, output, threshold).await?;
        }
        Commands::FilterDuplicates {
            input_json,
            min_distance,
        } => {
            commands::execute_filter_duplicates(input_json, min_distance).await?;
        }
        Commands::Process {
            duplicate_list,
            action,
            dest,
            no_confirm,
        } => {
            commands::execute_process(duplicate_list, action, dest, no_confirm).await?;
        }
    }

    Ok(())
}
