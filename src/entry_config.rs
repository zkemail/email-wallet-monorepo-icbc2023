use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use halo2_zk_email::vrm::DecomposedRegexConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryConfig {
    pub max_header_size: usize,
    pub rules: HashMap<usize, DecomposedRegexConfig>,
}
