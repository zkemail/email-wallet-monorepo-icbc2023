mod entry_config;
mod gen_regex_defs;
mod gen_relayer_configs;
mod gen_solidity;
mod js_caller;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use entry_config::EntryConfig;
use halo2_zk_email::*;
use std::env::{self};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        #[arg(short, long, default_value = "entry_config.json")]
        entry_config_path: String,
        #[arg(short, long, default_value = "email-wallet-contracts")]
        solidity_project_path: String,
        #[arg(short, long, default_value = "relayer")]
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
        .await
        .unwrap(),
    }
}

async fn setup1(
    entry_config_path: &str,
    solidity_project_path: &str,
    relayer_project_path: &str,
) -> Result<()> {
    let pwd = env::current_dir().unwrap();
    let mut entry_config_path = PathBuf::new().join(entry_config_path);
    if entry_config_path.is_relative() {
        entry_config_path = pwd.join(&entry_config_path);
    }
    let mut solidity_project_path = PathBuf::new().join(solidity_project_path);
    if solidity_project_path.is_relative() {
        solidity_project_path = pwd.join(&solidity_project_path);
    }
    let mut relayer_project_path = PathBuf::new().join(relayer_project_path);
    if relayer_project_path.is_relative() {
        relayer_project_path = pwd.join(&relayer_project_path);
    }

    let entry_config =
        serde_json::from_reader::<File, EntryConfig>(File::open(entry_config_path)?)?;
    entry_config.gen_solidity_codes(&solidity_project_path)?;
    entry_config.gen_regex_files(&relayer_project_path)?;
    entry_config.gen_config_files(&relayer_project_path)?;
    Command::new("bash").arg("./vrm/src/setup.sh").output()?;
    let app_param_path = pwd
        .join("relayer/configs/app_params.bin")
        .to_str()
        .unwrap()
        .to_string();
    let agg_param_path = pwd
        .join("relayer/configs/agg_params.bin")
        .to_str()
        .unwrap()
        .to_string();
    let agg_circuit_config_path = pwd
        .join("relayer/configs/agg_circuit.config")
        .to_str()
        .unwrap()
        .to_string();
    if !Path::new(&app_param_path).is_file() {
        downsize_param(&agg_param_path, &app_param_path, 20)?;
    }
    for idx in entry_config.rules.keys() {
        let app_circuit_config_path = pwd
            .join(&format!("relayer/configs/app_circuit_id{}.config", idx))
            .to_str()
            .unwrap()
            .to_string();
        let email_path = pwd
            .join(&format!("emails/email_id{}.eml", idx))
            .to_str()
            .unwrap()
            .to_string();
        let app_pk_path = pwd
            .join(&format!("relayer/configs/app_{}.pk", idx))
            .to_str()
            .unwrap()
            .to_string();
        let agg_pk_path = pwd
            .join(&format!("relayer/configs/agg_{}.pk", idx))
            .to_str()
            .unwrap()
            .to_string();
        let app_vk_path = pwd
            .join(&format!("relayer/configs/app_{}.vk", idx))
            .to_str()
            .unwrap()
            .to_string();
        let agg_vk_path = pwd
            .join(&format!("relayer/configs/agg_{}.vk", idx))
            .to_str()
            .unwrap()
            .to_string();
        let bytecode_path = pwd
            .join(&format!("relayer/configs/agg_verifier_id{}.bin", idx))
            .to_str()
            .unwrap()
            .to_string();
        let solidity_path = pwd
            .join(&format!(
                "email-wallet-contracts/src/rule{}/Verifier.sol",
                idx
            ))
            .to_str()
            .unwrap()
            .to_string();
        gen_app_key(
            &app_param_path,
            &app_circuit_config_path,
            &email_path,
            &app_pk_path,
            &app_vk_path,
        )
        .await?;
        gen_agg_key(
            &app_param_path,
            &agg_param_path,
            &app_circuit_config_path,
            &agg_circuit_config_path,
            &email_path,
            &app_pk_path,
            &agg_pk_path,
            &agg_vk_path,
        )
        .await?;
        gen_agg_evm_verifier(
            &agg_param_path,
            &app_circuit_config_path,
            &agg_circuit_config_path,
            &agg_vk_path,
            &bytecode_path,
            &solidity_path,
        )
        .await?;
    }
    entry_config.replace_verifier_names(&solidity_project_path)?;
    Command::new("forge")
        .arg("build")
        .current_dir("email-wallet-contracts")
        .output()?;
    entry_config.copy_abi_files(&solidity_project_path, &relayer_project_path)?;
    Ok(())
}
