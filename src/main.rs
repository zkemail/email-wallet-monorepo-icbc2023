mod entry_config;
mod gen_relayer_configs;
mod gen_solidity;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use entry_config::EntryConfig;
use halo2_zk_email::*;
use relayer::{ManipulationDef, ManipulationDefsJson};
use serde_json::Value;
use std::collections::HashMap;
use std::env::{self};
use std::fs::File;
use std::io::Write;
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
    New {
        #[arg(long, default_value = "./")]
        root_path: String,
        #[arg(short, long, default_value = "email-wallet-contracts")]
        solidity_project_path: String,
        #[arg(short, long, default_value = "relayer")]
        relayer_project_path: String,
    },
    AddRule {
        #[arg(long)]
        id: usize,
        #[arg(long, default_value = "./")]
        root_path: String,
        #[arg(short, long, default_value = "email-wallet-contracts")]
        solidity_project_path: String,
        #[arg(short, long, default_value = "relayer")]
        relayer_project_path: String,
    },
    // VerifyProof {
    //     #[arg(long)]
    //     id: usize,
    //     #[arg(long)]
    //     email_index: usize,
    //     #[arg(short, long, default_value = "relayer")]
    //     relayer_project_path: String,
    // },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::New {
            root_path,
            solidity_project_path,
            relayer_project_path,
        } => new(&root_path, &solidity_project_path, &relayer_project_path)
            .await
            .unwrap(),
        Commands::AddRule {
            id,
            root_path,
            solidity_project_path,
            relayer_project_path,
        } => add_rule(
            id,
            &root_path,
            &solidity_project_path,
            &relayer_project_path,
        )
        .await
        .unwrap(),
        // Commands::VerifyProof {
        //     id,
        //     email_index,
        //     relayer_project_path,
        // } => {
        //     let pwd = env::current_dir().unwrap();
        //     let mut relayer_project_path = PathBuf::new().join(relayer_project_path);
        //     if relayer_project_path.is_relative() {
        //         relayer_project_path = pwd.join(&relayer_project_path);
        //     }
        //     let agg_circuit_config_path = pwd
        //         .join("relayer/configs/agg_circuit.config")
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     let app_circuit_config_path = pwd
        //         .join(&format!("relayer/configs/app_circuit_id{}.config", id))
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     let bytecode_path = pwd
        //         .join(&format!("relayer/configs/agg_verifier_id{}.bin", id))
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     let proof_path = pwd
        //         .join(&format!("relayer/emails/proof_{}.hex", email_index))
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     let acc_path = pwd
        //         .join(&format!("relayer/emails/acc_{}.hex", email_index))
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     let public_input_path = pwd
        //         .join(&format!("relayer/emails/public_input_{}.hex", email_index))
        //         .to_str()
        //         .unwrap()
        //         .to_string();
        //     evm_verify_agg(
        //         &app_circuit_config_path,
        //         &agg_circuit_config_path,
        //         &bytecode_path,
        //         &proof_path,
        //         &acc_path,
        //         &public_input_path,
        //     )
        //     .unwrap();
        // }
    }
}

async fn new(
    root_path: &str,
    solidity_project_path: &str,
    relayer_project_path: &str,
) -> Result<()> {
    let pwd = env::current_dir().unwrap();
    let mut root_path = PathBuf::new().join(root_path);
    if root_path.is_relative() {
        root_path = pwd.join(&root_path);
    }
    let mut solidity_project_path = PathBuf::new().join(solidity_project_path);
    if solidity_project_path.is_relative() {
        solidity_project_path = pwd.join(&solidity_project_path);
    }
    let mut relayer_project_path = PathBuf::new().join(relayer_project_path);
    if relayer_project_path.is_relative() {
        relayer_project_path = pwd.join(&relayer_project_path);
    }
    // 1. relayer files.
    // 1.1. .env file.
    let env_template = include_str!("templates/.env.template");
    write!(
        File::create(&relayer_project_path.join(".env"))?,
        "{}",
        env_template
    )?;
    // 1.2. manipulation defs
    let manipulation_defs_path = relayer_project_path
        .join("configs")
        .join("manipulation_defs.json");
    let manipulation_defs_json = ManipulationDefsJson {
        rules: HashMap::<usize, ManipulationDef>::new(),
    };
    write!(
        File::create(&manipulation_defs_path)?,
        "{}",
        serde_json::to_string(&manipulation_defs_json)?
    )?;
    // 1.3. agg_params.bin
    let agg_params_path = relayer_project_path.join("configs").join("agg_params.bin");
    if !agg_params_path.is_file() {
        Command::new("wget")
            .arg("-O")
            .arg(agg_params_path.to_str().unwrap())
            .arg("https://trusted-setup-halo2kzg.s3.eu-central-1.amazonaws.com/perpetual-powers-of-tau-raw-24")
            .output()?;
    }

    // 2. solidity abi files.
    Command::new("forge")
        .arg("build")
        .current_dir(&solidity_project_path)
        .output()?;
    let out_path = solidity_project_path.join("out");
    let config_path = relayer_project_path.join("configs");
    for name in ["EmailWallet", "IERC20", "IManipulator"] {
        let json_path = out_path.join(&format!("{}.sol/{}.json", name, name));
        let json_value: Value = serde_json::from_reader(File::open(&json_path)?)?;
        let abi_value = json_value.get("abi").unwrap();
        let abi_str = serde_json::to_string(&abi_value)?;
        let abi_json_path = config_path.join(&format!("{}.json", name));
        let mut file = File::create(&abi_json_path)?;
        write!(file, "{}", abi_str)?;
        file.flush()?;
    }
    Ok(())
}

async fn add_rule(
    id: usize,
    root_path: &str,
    solidity_project_path: &str,
    relayer_project_path: &str,
) -> Result<()> {
    let pwd = env::current_dir().unwrap();
    let mut root = PathBuf::new().join(root_path);
    if root.is_relative() {
        root = pwd.join(&root);
    }
    let mut solidity_project_path = PathBuf::new().join(solidity_project_path);
    if solidity_project_path.is_relative() {
        solidity_project_path = pwd.join(&solidity_project_path);
    }
    let mut relayer_project_path = PathBuf::new().join(relayer_project_path);
    if relayer_project_path.is_relative() {
        relayer_project_path = pwd.join(&relayer_project_path);
    }

    let entry_config_path = root.join("entry_config.json");
    let entry_config =
        serde_json::from_reader::<File, EntryConfig>(File::open(entry_config_path)?)?;
    entry_config.gen_solidity_codes(&solidity_project_path, id)?;
    // entry_config.gen_regex_files(&relayer_project_path, id)?;
    entry_config.gen_config_files(&relayer_project_path, id)?;
    // Command::new("bash").arg("./vrm/src/setup.sh").output()?;
    let app_param_path = relayer_project_path
        .join("configs/app_params.bin")
        .to_str()
        .unwrap()
        .to_string();
    let agg_param_path = relayer_project_path
        .join("configs/agg_params.bin")
        .to_str()
        .unwrap()
        .to_string();
    let agg_circuit_config_path = relayer_project_path
        .join("configs/agg_circuit.config")
        .to_str()
        .unwrap()
        .to_string();
    let app_circuit_config_path = relayer_project_path
        .join(&format!("configs/app_circuit_id{}.config", id))
        .to_str()
        .unwrap()
        .to_string();
    let email_path = root
        .join(&format!("example_emails/email_id{}.eml", id))
        .to_str()
        .unwrap()
        .to_string();
    let app_pk_path = relayer_project_path
        .join(&format!("relayer/configs/app_{}.pk", id))
        .to_str()
        .unwrap()
        .to_string();
    let agg_pk_path = relayer_project_path
        .join(&format!("configs/agg_{}.pk", id))
        .to_str()
        .unwrap()
        .to_string();
    let app_vk_path = relayer_project_path
        .join(&format!("configs/app_{}.vk", id))
        .to_str()
        .unwrap()
        .to_string();
    let agg_vk_path = relayer_project_path
        .join(&format!("configs/agg_{}.vk", id))
        .to_str()
        .unwrap()
        .to_string();
    let bytecode_path = relayer_project_path
        .join(&format!("configs/agg_verifier_id{}.bin", id))
        .to_str()
        .unwrap()
        .to_string();
    let solidity_path = solidity_project_path
        .join(&format!("src/rule{}/Verifier.sol", id))
        .to_str()
        .unwrap()
        .to_string();
    if !Path::new(&app_param_path).is_file() {
        downsize_param(&agg_param_path, &app_param_path, 20)?;
    }
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
    entry_config.replace_verifier_names(&solidity_project_path, id)?;
    Ok(())
}
