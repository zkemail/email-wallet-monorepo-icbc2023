mod entry_config;
mod gen_regex_defs;
mod gen_verifier_wrapper;
mod js_caller;

use anyhow::Result;
use clap::{Parser, Subcommand};
use entry_config::EntryConfig;
use js_caller::get_dfa_json_value;
use std::env::set_var;
use std::fmt::format;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use tokio::macros::*;

use crate::js_caller::{add_graph_nodes, get_accepted_state, get_max_state};

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
        #[arg(short, long, default_value = "./relayer")]
        relayer_project_path: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Setup1 {
            entry_config_path,
            solidity_project_path,
            relayer_project_path,
        } => setup1(
            &entry_config_path,
            &solidity_project_path,
            &relayer_project_path,
        )
        .unwrap(),
    }
}

fn setup1(
    entry_config_path: &str,
    solidity_project_path: &str,
    relayer_project_path: &str,
) -> Result<()> {
    let entry_config =
        serde_json::from_reader::<File, EntryConfig>(File::open(entry_config_path)?)?;
    entry_config.output_solidity_codes(solidity_project_path)?;
    entry_config.gen_regex_files(relayer_project_path)?;
    // let dfa_val = get_dfa_json_value(&format!(r"{}", entry_config.rules[&1][0].regex_def))?;
    // let accepted_nodes = get_accepted_state(&dfa_val).unwrap();
    // println!("accepted {}", accepted_nodes);
    // let mut dag = Dag::new();
    // let max_state = get_max_state(&dfa_val)?;
    // add_dag_nodes(&dfa_val, &mut dag, None, max_state)?;
    // println!("{:?}", dfa_val);
    Ok(())
}
