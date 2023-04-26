use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use relayer::RegexType as SoldityType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPartConfig {
    is_public: bool,
    regex_def: String,
    solidity: Option<SoldityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryConfig {
    pub max_header_size: usize,
    pub max_body_size: usize,
    pub rules: HashMap<usize, Vec<BodyPartConfig>>,
}

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

    pub fn output_solidity_codes(&self, solidity_project_path: &str) -> Result<()> {
        let src_path = Path::new(solidity_project_path).join("src");
        for id in self.rules.keys() {
            let dir_path = src_path.join(format!("rule{}", id.to_string()));
            if dir_path.is_dir() {
                fs::remove_dir_all(&dir_path)?;
            }
            fs::create_dir(&dir_path)?;
            let verifier = self.output_verifier_for_one_rule(*id)?;
            let mut verifier_file = File::create(dir_path.join("VerifierWrapper.sol"))?;
            write!(verifier_file, "{}", verifier)?;
            verifier_file.flush()?;
            let manipulator = self.output_manipulator_for_one_rule(*id)?;
            let mut manipulator_file = File::create(dir_path.join("Manipulator.sol"))?;
            write!(manipulator_file, "{}", manipulator)?;
            manipulator_file.flush()?;
        }
        Ok(())
    }

    fn output_verifier_for_one_rule(&self, id: usize) -> Result<String> {
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

    fn output_manipulator_for_one_rule(&self, id: usize) -> Result<String> {
        let mut template: String = include_str!("Manipulator.sol.template").to_string();
        template = template.replace("<%RULE_INDEX%>", format!("Rule{}", id.to_string()).as_str());
        Ok(template)
    }
}
