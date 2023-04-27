use std::{collections::HashMap, fs::File, path::Path};

use anyhow::{anyhow, Result};
use fancy_regex::Regex;
use relayer::RegexType as SoldityType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPartConfig {
    pub is_public: bool,
    pub regex_def: String,
    pub max_size: usize,
    pub solidity: Option<SoldityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryConfig {
    pub max_header_size: usize,
    pub max_body_size: usize,
    pub rules: HashMap<usize, Vec<BodyPartConfig>>,
}
