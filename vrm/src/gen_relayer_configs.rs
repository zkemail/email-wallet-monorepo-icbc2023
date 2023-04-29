use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use crate::entry_config::{BodyPartConfig, EntryConfig};
use anyhow::Result;
use halo2_zk_email::DefaultEmailVerifyConfigParams;
use relayer::{ManipulationDef, ManipulationDefsJson};
use std::fs::File;
use std::io::Write;

impl EntryConfig {
    pub fn gen_config_files(&self, relayer_project_path: &PathBuf, id: usize) -> Result<()> {
        let manipulation_defs_path = relayer_project_path
            .join("configs")
            .join("manipulation_defs.json");
        let mut manipulation_defs_json = if manipulation_defs_path.is_file() {
            serde_json::from_reader(File::open(&manipulation_defs_path)?)?
        } else {
            let manipulation_defs_map = HashMap::<usize, ManipulationDef>::new();
            ManipulationDefsJson {
                rules: manipulation_defs_map,
            }
        };
        let def = self.gen_config_files_for_one_rule(id, relayer_project_path)?;
        manipulation_defs_json.rules.insert(id, def);

        // let mut manipulation_defs_map = HashMap::<usize, ManipulationDef>::new();
        // for id in self.rules.keys() {
        //     let def = self.gen_config_files_for_one_rule(*id, relayer_project_path)?;
        //     manipulation_defs_map.insert(*id, def);
        // }
        // let manipulation_defs_json = ManipulationDefsJson {
        //     rules: manipulation_defs_map,
        // };
        write!(
            File::create(&manipulation_defs_path)?,
            "{}",
            serde_json::to_string_pretty::<ManipulationDefsJson>(&manipulation_defs_json)?
        )?;
        Ok(())
    }

    fn gen_config_files_for_one_rule(
        &self,
        id: usize,
        relayer_project_path: &PathBuf,
    ) -> Result<ManipulationDef> {
        let config_path = relayer_project_path.join("configs");
        let mut app_params: DefaultEmailVerifyConfigParams =
            serde_json::from_str(include_str!("app_default.config"))?;
        app_params.header_max_byte_size = self.max_header_size;
        app_params.body_max_byte_size = self.max_body_size;
        let pwd = env::current_dir().unwrap();
        app_params.bodyhash_regex_filepath = pwd
            .join(&app_params.bodyhash_regex_filepath)
            .to_str()
            .unwrap()
            .to_string();
        app_params.bodyhash_substr_filepath = pwd
            .join(&app_params.bodyhash_substr_filepath)
            .to_str()
            .unwrap()
            .to_string();
        for path in app_params.header_regex_filepathes.iter_mut() {
            *path = pwd.join(&path).to_str().unwrap().to_string();
        }
        for path in app_params.header_substr_filepathes.iter_mut() {
            *path = pwd.join(&path).to_str().unwrap().to_string();
        }
        let regex_path = config_path.join(&format!("regex_body_id{}.txt", id));
        let mut num_public_part = 0;
        let mut solidity_types = vec![];
        for (idx, part_config) in self.rules[&id].iter().enumerate() {
            if !part_config.is_public {
                continue;
            }
            app_params
                .body_regex_filepathes
                .push(regex_path.to_str().unwrap().to_string());
            let substr_path =
                config_path.join(&format!("substr_body_id{}_{}.txt", id, num_public_part));
            app_params
                .body_substr_filepathes
                .push(substr_path.to_str().unwrap().to_string());
            let mut substr_defs_vec = vec![];
            let mut substr_def_str = String::new();
            if idx != 0 {
                substr_def_str += &self.rules[&id][idx - 1].regex_def;
            }
            substr_def_str += &part_config.regex_def;
            if idx != self.rules[&id].len() - 1 {
                substr_def_str += &self.rules[&id][idx + 1].regex_def;
            }
            substr_defs_vec.push(substr_def_str);
            substr_defs_vec.push(part_config.regex_def.clone());
            app_params.body_substr_regexes.push(substr_defs_vec);
            num_public_part += 1;
            solidity_types.push(part_config.solidity.unwrap());
        }
        let new_config_str = serde_json::to_string_pretty(&app_params)?;
        let app_config_path = config_path.join(&format!("app_circuit_id{}.config", id));
        write!(File::create(app_config_path.clone())?, "{}", new_config_str)?;

        let def = ManipulationDef {
            app_config_path: app_config_path.to_str().unwrap().to_string(),
            agg_config_path: config_path
                .join("agg_circuit.config")
                .to_str()
                .unwrap()
                .to_string(),
            app_pk_path: config_path
                .join(&format!("app_{}.pk", id))
                .to_str()
                .unwrap()
                .to_string(),
            agg_pk_path: config_path
                .join(&format!("agg_{}.pk", id))
                .to_str()
                .unwrap()
                .to_string(),
            max_header_size: app_params.header_max_byte_size,
            max_body_size: app_params.body_max_byte_size,
            types: solidity_types,
        };
        Ok(def)
    }
}
