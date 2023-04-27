use std::collections::HashMap;

use anyhow::{anyhow, Result};
use js_sandbox::{AnyError, Script};
use petgraph::prelude::*;
use serde_json::Value;

pub fn cacth_all_regex_str() -> Result<String> {
    let code: &'static str = include_str!("regex.js");
    let mut script = Script::from_string(code)?;
    let result: String = script.call("catchAllRegexStr", ())?;
    Ok(result)
}

pub fn text_context_prefix_regex_str() -> Result<String> {
    let code: &'static str = include_str!("regex.js");
    let mut script = Script::from_string(code)?;
    let result: String = script.call("textContextPrefix", ())?;
    Ok(result)
}

pub fn format_regex_str(regex: &str) -> Result<String> {
    let code: &'static str = include_str!("regex.js");
    let mut script = Script::from_string(code)?;
    let result: String = script.call("formatRegexPrintable", (regex,))?;
    Ok(result)
}

pub fn get_dfa_json_value(regex: &str) -> Result<Vec<Value>> {
    let code: &'static str = include_str!("regex.js");
    let mut script = Script::from_string(code)?;
    let result: String = script.call("regexToDfa", (regex,))?;
    Ok(serde_json::from_str(&result)?)
}

pub fn get_accepted_state(dfa_val: &[Value]) -> Option<usize> {
    for i in 0..dfa_val.len() {
        if dfa_val[i]["type"] == "accept" {
            return Some(i as usize);
        }
    }
    None
}

pub fn get_max_state(dfa_val: &[Value]) -> Result<usize> {
    let mut max_state = 0;
    for (i, val) in dfa_val.iter().enumerate() {
        for (_, next_node_val) in val["edges"]
            .as_object()
            .ok_or(anyhow!("Edges are not object"))?
            .iter()
        {
            let next_node = next_node_val
                .as_u64()
                .ok_or(anyhow!("next node value is not u64"))? as usize;
            if next_node > max_state {
                max_state = next_node;
            }
        }
    }
    Ok(max_state)
}

pub fn add_graph_nodes(
    dfa_val: &[Value],
    graph: &mut Graph<bool, char, Directed, usize>,
    last_max_state: Option<usize>,
    next_max_state: usize,
) -> Result<()> {
    let first_new_state = match last_max_state {
        Some(v) => v + 1,
        None => 0,
    };
    for idx in first_new_state..=next_max_state {
        graph.add_node(idx == next_max_state);
    }

    for (i, val) in dfa_val.iter().enumerate() {
        for (key, next_node_val) in val["edges"]
            .as_object()
            .ok_or(anyhow!("Edges are not object"))?
            .iter()
        {
            let next_node = next_node_val
                .as_u64()
                .ok_or(anyhow!("next node value is not u64"))? as usize;
            if let Some(max) = last_max_state {
                if i <= max && next_node <= max {
                    continue;
                }
            }
            let key_list: Vec<String> = serde_json::from_str(&key)
                .map_err(|e| anyhow!("serde_json from_str error {}", e))?;
            for key_char in key_list.iter() {
                assert!(key_char.len() == 1);
                let key_char: char = key_char.chars().collect::<Vec<char>>()[0];
                // println!("i {}, next {} key {}", i, next_node, key_char);
                graph.add_edge(NodeIndex::from(next_node), NodeIndex::from(i), key_char);
            }
            // println!("key {}, val {}", k, next_node_val);
            // let k_list: Vec<String> =
            //     serde_json::from_str(&k).map_err(|e| anyhow!("serde_json from_str error {}", e))?;
            // println!("k_first {}", k_list[0]);
            // let next_node = next_node_val
            //     .as_u64()
            //     .ok_or(anyhow!("next node value is not u64"))? as usize;
            // if let Some(max) = last_max_state {
            //     if i <= max && next_node <= max {
            //         continue;
            //     }
            // }
            // // println!("i {}, next_node {}", i, next_node);
            // graph.add_edge(NodeIndex::from(next_node), NodeIndex::from(i), ());
        }
    }
    Ok(())
}

pub fn dfa_to_regex_def_text(dfa_val: &[Value]) -> Result<String> {
    let accepted_state = get_accepted_state(dfa_val).ok_or(anyhow!("No accepted state"))?;
    let max_state = get_max_state(dfa_val)?;
    let mut text = "0\n".to_string();
    text += &format!("{}\n", accepted_state.to_string());
    text += &format!("{}\n", max_state.to_string());
    for (i, val) in dfa_val.iter().enumerate() {
        for (key, next_node_val) in val["edges"]
            .as_object()
            .ok_or(anyhow!("Edges are not object"))?
            .iter()
        {
            let key_list: Vec<String> = serde_json::from_str(&key)
                .map_err(|e| anyhow!("serde_json from_str error {}", e))?;
            for key_char in key_list.iter() {
                let key_char: char = key_char.chars().collect::<Vec<char>>()[0];
                let next_node = next_node_val
                    .as_u64()
                    .ok_or(anyhow!("next node value is not u64"))?
                    as usize;
                // println!("i {} next {} char {}", i, next_node, key_char as u8);
                text += &format!(
                    "{} {} {}\n",
                    i.to_string(),
                    next_node.to_string(),
                    (key_char as u8).to_string()
                );
            }
            // println!(
            //     "char len {}",
            //     key_list[0].chars().collect::<Vec<char>>().len()
            // );
            // let key_char: char = key_list[0].chars().collect::<Vec<char>>()[0];
            // let next_node = next_node_val
            //     .as_u64()
            //     .ok_or(anyhow!("next node value is not u64"))? as usize;
            // println!("i {} next {} char {}", i, next_node, key_char as u8);
            // text += &format!(
            //     "{} {} {}\n",
            //     i.to_string(),
            //     next_node.to_string(),
            //     (key_char as u8).to_string()
            // );
        }
    }
    Ok(text)
}

pub fn to_js_regex(regex: &str) -> String {
    let mut replaced = regex.replace(r"\\/", r"/");
    replaced = replaced.replace(r"\\♥", r"\u000b");
    replaced = replaced.replace(r"\\[", r"[");
    replaced = replaced.replace(r"\\]", r"]");
    replaced = replaced.replace(r"\\.", r".");
    replaced = replaced.replace(r"\\$", r"$");
    replaced = replaced.replace(r"\\^", r"^");
    replaced
}
