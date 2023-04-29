use crate::entry_config::{BodyPartConfig, EntryConfig};
use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use itertools::Itertools;
use relayer::RegexType as SoldityType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::{collections::HashMap, fs::File, path::Path};

impl EntryConfig {
    const STRING_PARAM_DEFS: &'static str =
        "        uint substr<%ID%>Start;\n        string substr<%ID%>String;\n";

    const UINT_PARAM_DEFS: &'static str =
        "        uint substr<%ID%>Start;\n        uint substr<%ID%>Uint;\n";

    const DECIMAL_PARAM_DEFS: &'static str = "        uint substr<%ID%>Start;\n        uint substr<%ID%>IntPart;\n        uint substr<%ID%>DecNumZero;\n        uint substr<%ID%>DecimalPart;\n";

    const STRING_ENCODE_PART: &'static str = r"
        bytes memory substr<%ID%>Bytes = bytes(param.substr<%ID%>String);
        for (uint i = 0; i < substr<%ID%>Bytes.length; i++) {
            maskedStrPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = substr<%ID%>Bytes[i];
            substrIdsPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = bytes1(uint8(<%ID+1%>));
        }

    ";

    const UINT_ENCODE_PART: &'static str = r"
        bytes memory substr<%ID%>Bytes = bytes(param.substr<%ID%>Uint.toString());
        for (uint i = 0; i < substr<%ID%>Bytes.length; i++) {
            maskedStrPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = substr<%ID%>Bytes[i];
            substrIdsPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = bytes1(uint8(<%ID+1%>));
        }

    ";

    const DECIMAL_ENCODE_PART: &'static str = r"
        bytes memory substr<%ID%>Bytes = bytes(
            decString(
                param.substr<%ID%>IntPart,
                param.substr<%ID%>DecNumZero,
                param.substr<%ID%>DecimalPart
            )
        );
        for (uint i = 0; i < substr<%ID%>Bytes.length; i++) {
            maskedStrPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = substr<%ID%>Bytes[i];
            substrIdsPart[
                HEADER_MAX_BYTE_SIZE + param.substr<%ID%>Start + i
            ] = bytes1(uint8(<%ID+1%>));
        }

    ";

    pub fn gen_solidity_codes(&self, solidity_project_path: &PathBuf, id: usize) -> Result<()> {
        let src_path = solidity_project_path.join("src");
        let dir_path = src_path.join(format!("rule{}", id.to_string()));
        if dir_path.is_dir() {
            fs::remove_dir_all(&dir_path)?;
        }
        fs::create_dir(&dir_path)?;
        let verifier = self.gen_verifier_for_one_rule(id)?;
        let mut verifier_file = File::create(dir_path.join("VerifierWrapper.sol"))?;
        write!(verifier_file, "{}", verifier)?;
        verifier_file.flush()?;
        let manipulator = self.gen_manipulator_for_one_rule(id)?;
        let mut manipulator_file = File::create(dir_path.join("Manipulator.sol"))?;
        write!(manipulator_file, "{}", manipulator)?;
        manipulator_file.flush()?;
        let script_path = solidity_project_path.join("script");
        let mut deployer = self.gen_deploy_script_for_one_rule(id)?;
        let mut deployer_file =
            File::create(script_path.join(format!("DeployManipulator{}.s.sol", id)))?;
        write!(deployer_file, "{}", deployer)?;
        deployer_file.flush()?;

        // for id in self.rules.keys() {
        //     let dir_path = src_path.join(format!("rule{}", id.to_string()));
        //     if dir_path.is_dir() {
        //         fs::remove_dir_all(&dir_path)?;
        //     }
        //     fs::create_dir(&dir_path)?;
        //     let verifier = self.gen_verifier_for_one_rule(*id)?;
        //     let mut verifier_file = File::create(dir_path.join("VerifierWrapper.sol"))?;
        //     write!(verifier_file, "{}", verifier)?;
        //     verifier_file.flush()?;
        //     let manipulator = self.gen_manipulator_for_one_rule(*id)?;
        //     let mut manipulator_file = File::create(dir_path.join("Manipulator.sol"))?;
        //     write!(manipulator_file, "{}", manipulator)?;
        //     manipulator_file.flush()?;
        // }
        Ok(())
    }

    fn gen_verifier_for_one_rule(&self, id: usize) -> Result<String> {
        let mut template = include_str!("VerifierWrapper.sol.template").to_string();
        template = template.replace("<%RULE_INDEX%>", format!("Rule{}", id.to_string()).as_str());
        template = template.replace(
            "<%HEADER_MAX_BYTE_SIZE%>",
            self.max_header_size.to_string().as_str(),
        );
        template = template.replace(
            "<%BODY_MAX_BYTE_SIZE%>",
            self.max_body_size.to_string().as_str(),
        );
        let part_configs = &self.rules[&id];
        let mut body_param_defs = String::new();
        let mut body_encode_part = String::new();
        let mut substr_id = 0;
        for part in part_configs.iter() {
            if !part.is_public {
                continue;
            }
            let substr_id_str = substr_id.to_string();
            match part
                .solidity
                .ok_or(anyhow!("The public part must have a Solidity type."))?
            {
                SoldityType::String => {
                    body_param_defs += Self::STRING_PARAM_DEFS
                        .replace("<%ID%>", &substr_id_str)
                        .as_str();
                    let mut new_encode_part =
                        Self::STRING_ENCODE_PART.replace("<%ID%>", substr_id_str.as_str());
                    new_encode_part =
                        new_encode_part.replace("<%ID+1%>", (substr_id + 1).to_string().as_str());
                    body_encode_part += new_encode_part.as_str();
                }
                SoldityType::Uint => {
                    body_param_defs += Self::UINT_PARAM_DEFS
                        .replace("<%ID%>", &substr_id_str)
                        .as_str();
                    let mut new_encode_part =
                        Self::UINT_ENCODE_PART.replace("<%ID%>", substr_id_str.as_str());
                    new_encode_part =
                        new_encode_part.replace("<%ID+1%>", (substr_id + 1).to_string().as_str());
                    body_encode_part += new_encode_part.as_str();
                }
                SoldityType::Decimal => {
                    body_param_defs += Self::DECIMAL_PARAM_DEFS
                        .replace("<%ID%>", &substr_id_str)
                        .as_str();
                    let mut new_encode_part =
                        Self::DECIMAL_ENCODE_PART.replace("<%ID%>", substr_id_str.as_str());
                    new_encode_part =
                        new_encode_part.replace("<%ID+1%>", (substr_id + 1).to_string().as_str());
                    body_encode_part += new_encode_part.as_str();
                }
            }
            substr_id += 1;
        }
        template = template.replace("<%BODY_PARAM_DEFS%>", &body_param_defs);
        template = template.replace("<%BODY_ENCODE_PART%>", &body_encode_part);
        Ok(template)
    }

    fn gen_manipulator_for_one_rule(&self, id: usize) -> Result<String> {
        let mut template: String = include_str!("Manipulator.sol.template").to_string();
        template = template.replace("<%RULE_INDEX%>", format!("Rule{}", id.to_string()).as_str());
        Ok(template)
    }

    fn gen_deploy_script_for_one_rule(&self, id: usize) -> Result<String> {
        let mut template: String = include_str!("DeployManipulator.s.sol.template").to_string();
        template = template.replace("<%RULE_INDEX%>", format!("{}", id.to_string()).as_str());
        Ok(template)
    }

    pub fn replace_verifier_names(&self, solidity_project_path: &PathBuf, id: usize) -> Result<()> {
        let src_path = solidity_project_path.join("src");
        let verifier_path = src_path
            .join(format!("rule{}", id.to_string()))
            .join("Verifier.sol");
        let mut verifier_code = fs::read_to_string(&verifier_path)?;
        verifier_code = verifier_code.replace("Verifier", &format!("Rule{}Verifier", id));
        let mut verifier_file = File::create(&verifier_path)?;
        write!(verifier_file, "{}", verifier_code)?;
        verifier_file.flush()?;
        // for id in self.rules.keys() {
        //     let verifier_path = src_path
        //         .join(format!("rule{}", id.to_string()))
        //         .join("Verifier.sol");
        //     let mut verifier_code = fs::read_to_string(&verifier_path)?;
        //     verifier_code = verifier_code.replace("Verifier", &format!("Rule{}Verifier", id));
        //     let mut verifier_file = File::create(&verifier_path)?;
        //     write!(verifier_file, "{}", verifier_code)?;
        //     verifier_file.flush()?;
        // }
        Ok(())
    }

    pub fn copy_abi_files(
        &self,
        solidity_project_path: &PathBuf,
        relayer_project_path: &PathBuf,
    ) -> Result<()> {
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
}
