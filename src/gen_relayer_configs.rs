use crate::entry_config::EntryConfig;
use anyhow::Result;
use halo2_zk_email::DefaultEmailVerifyConfigParams;
use relayer::config::{ManipulationDef, ManipulationDefsJson};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

impl EntryConfig {
    pub fn gen_config_files(&self, relayer_project_path: &PathBuf, id: usize) -> Result<()> {
        self.gen_regex_files_for_one_rule(id, relayer_project_path)?;
        let manipulation_defs_path = relayer_project_path
            .join("configs")
            .join("manipulation_defs.json");
        let mut manipulation_defs_json = serde_json::from_reader::<File, ManipulationDefsJson>(
            File::open(&manipulation_defs_path)?,
        )?;
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

    fn gen_regex_files_for_one_rule(
        &self,
        id: usize,
        relayer_project_path: &PathBuf,
    ) -> Result<()> {
        let config_path = relayer_project_path.join("configs");
        let allstr_file_path = config_path.join(&format!("body_id{}_allstr.txt", id));
        let mut substr_file_pathes = vec![];
        let mut num_public_part = 0;
        for part in self.rules[&id].parts.iter() {
            if part.is_public {
                substr_file_pathes.push(
                    PathBuf::new().join(&format!("body_id{}_substr_{}.txt", id, num_public_part)),
                );
                num_public_part += 1;
            }
        }
        self.rules[&id].gen_regex_files(&allstr_file_path, &substr_file_pathes)?;
        Ok(())
    }

    fn gen_config_files_for_one_rule(
        &self,
        id: usize,
        relayer_project_path: &PathBuf,
    ) -> Result<ManipulationDef> {
        let config_path = relayer_project_path.join("configs");
        let mut app_params: DefaultEmailVerifyConfigParams =
            serde_json::from_str(include_str!("templates/app_default.config"))?;
        app_params.header_max_byte_size = self.max_header_size;
        app_params.body_max_byte_size = self.rules[&id].max_byte_size;
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
        app_params.body_regex_filepath = config_path
            .join(&format!("body_id{}_allstr.txt", id))
            .to_str()
            .unwrap()
            .to_string();
        let mut num_public_part = 0;
        let mut solidity_types = vec![];
        for (idx, part_config) in self.rules[&id].parts.iter().enumerate() {
            if !part_config.is_public {
                continue;
            }
            let substr_path =
                config_path.join(&format!("body_id{}_substr_{}.txt", id, num_public_part));
            app_params
                .body_substr_filepathes
                .push(substr_path.to_str().unwrap().to_string());
            let mut substr_defs_vec = vec![];
            let mut substr_def_str = String::new();
            if idx != 0 {
                substr_def_str += &self.rules[&id].parts[idx - 1].regex_def;
            }
            substr_def_str += &part_config.regex_def;
            if idx != self.rules[&id].parts.len() - 1 {
                substr_def_str += &self.rules[&id].parts[idx + 1].regex_def;
            }
            substr_defs_vec.push(substr_def_str);
            substr_defs_vec.push(part_config.regex_def.clone());
            app_params.body_substr_regexes.push(substr_defs_vec);
            solidity_types.push(part_config.solidity.unwrap());
            num_public_part += 1;
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
