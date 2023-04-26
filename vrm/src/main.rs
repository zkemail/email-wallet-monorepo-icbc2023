mod entry_config;
mod gen_verifier_wrapper;

use anyhow::Result;
use clap::{Parser, Subcommand};
use entry_config::EntryConfig;
use std::env::set_var;
use std::fmt::format;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use tokio::macros::*;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand, Clone)]
enum Commands {
    /// The first setup.
    Setup1 {
        #[arg(short, long, default_value = "./entry_config.json")]
        entry_config_path: String,
        #[arg(short, long, default_value = "./email-wallet-contracts")]
        solidity_project_path: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Setup1 {
            entry_config_path,
            solidity_project_path,
        } => setup1(&entry_config_path, &solidity_project_path).unwrap(),
    }
}

fn setup1(entry_config_path: &str, solidity_project_path: &str) -> Result<()> {
    let entry_config =
        serde_json::from_reader::<File, EntryConfig>(File::open(entry_config_path)?)?;
    entry_config.output_solidity_codes(solidity_project_path)?;
    Ok(())
}
