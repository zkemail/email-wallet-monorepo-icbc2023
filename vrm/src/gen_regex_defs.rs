use crate::entry_config::*;
use crate::js_caller::*;
use anyhow::anyhow;
use anyhow::Result;
use fancy_regex::Regex;
use itertools::Itertools;
// use daggy::Dag;
// use daggy::NodeIndex;
// use daggy::Walker;
use graph_cycles::Cycles;
use petgraph::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::format;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

impl EntryConfig {
    // pub const CACTH_ALL_REGEX:&'static str = "(0|1|2|3|4|5|6|7|8|9|a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z|A|B|C|D|E|F|G|H|I|J|K|L|M|N|O|P|Q|R|S|T|U|V|W|X|Y|Z|!|\"|#|\\$|%|&|'|\\(|\\)|\\*|\\+|,|-|\\.|\\/|:|;|<|=|>|\\?|@|\\[|\\\\|\\]|\\^|_|`|{|\\||}|~| |\t|\n|\r|\x0b|\x0c)";
    pub fn gen_regex_files(&self, relayer_project_path: &str) -> Result<()> {
        let config_path = Path::new(relayer_project_path).join("configs");
        for id in self.rules.keys() {
            let regex_file_path = config_path.join(format!("regex_body_id{}.txt", id.to_string()));
            let id_to_substr_path = |substr_id: usize| {
                config_path.join(format!(
                    "substr_body_id{}_{}",
                    id.to_string(),
                    substr_id.to_string()
                ))
            };
            self.gen_regex_files_for_one_rule(*id, &regex_file_path, id_to_substr_path)?;
        }
        Ok(())
    }

    fn gen_regex_files_for_one_rule<F: Fn(usize) -> PathBuf>(
        &self,
        id: usize,
        regex_file_path: &PathBuf,
        id_to_substr_path: F,
    ) -> Result<()> {
        let catch_all: String = cacth_all_regex_str()?;
        let text_context_prefix = text_context_prefix_regex_str()?;
        let first_part = BodyPartConfig {
            is_public: false,
            regex_def: "(".to_string() + catch_all.as_str() + "+)?" + &text_context_prefix,
            max_size: self.max_body_size,
            solidity: None,
        };
        let last_part = BodyPartConfig {
            is_public: false,
            regex_def: "\r\n(".to_string() + catch_all.as_str() + "+)?",
            max_size: self.max_body_size,
            solidity: None,
        };
        let part_configs = vec![&[first_part][..], &self.rules[&id][..], &[last_part][..]].concat();
        let mut all_regex = String::new();
        let mut public_config_indexes = vec![];
        let mut part_regexes = vec![];
        for (idx, config) in part_configs.iter().enumerate() {
            all_regex += &config.regex_def;
            if config.is_public {
                public_config_indexes.push(idx);
            }
            part_regexes.push(Regex::new(&format_regex_str(&config.regex_def)?)?);
        }
        // println!("all_regex {}", all_regex);
        let dfa_val = get_dfa_json_value(&all_regex)?;
        let regex_text = dfa_to_regex_def_text(&dfa_val)?;
        let mut regex_file = File::create(regex_file_path)?;
        write!(regex_file, "{}", regex_text)?;
        regex_file.flush()?;

        let mut graph = Graph::<bool, char, Directed, usize>::with_capacity(0, 0);
        let max_state = get_max_state(&dfa_val)?;
        add_graph_nodes(&dfa_val, &mut graph, None, max_state)?;
        let mut writers = public_config_indexes
            .iter()
            .enumerate()
            .map(|(substr_id, index)| {
                let mut writer = BufWriter::new(File::create(id_to_substr_path(substr_id))?);
                writer.write_fmt(format_args!("{}\n", &part_configs[*index].max_size))?;
                writer.write_fmt(format_args!("0\n{}\n", self.max_body_size - 1))?;
                Ok(writer)
            })
            .collect::<Result<Vec<BufWriter<File>>>>()?;
        let accepted_state = get_accepted_state(&dfa_val).unwrap();

        let mut remove_edges = HashSet::new();
        graph.visit_all_cycles(|g, cycle_nodes| {
            let e: EdgeIndex<usize> = g
                .find_edge(cycle_nodes[cycle_nodes.len() - 1], cycle_nodes[0])
                .unwrap();
            if !remove_edges.contains(&e) {
                remove_edges.insert(e);
            }
        });
        for e in remove_edges.iter() {
            graph.remove_edge(*e).unwrap();
        }
        // let cycles = graph.cycles();
        // println!("cycles len {}", cycles.len());
        // println!("cycles {:?}", cycles);

        // let write_substr_patterns = || {
        //     let path_chars = rev_chars.iter().rev().map(|c| *c).collect::<Vec<char>>();
        //     let path_states = nodes
        //         .iter()
        //         .rev()
        //         .map(|n| n.index())
        //         .collect::<Vec<usize>>();
        //     let mut cur_regex_id = 0;
        //     let mut last_match_start_index = None;
        //     let mut last_match_end_index = 0;
        //     let mut concat_str = String::new();
        //     for (idx, (state, char)) in path_states
        //         .into_iter()
        //         .zip(path_chars.into_iter())
        //         .enumerate()
        //     {
        //         concat_str += &format_regex_str(&char.to_string())?;
        //         if part_regexes[cur_regex_id].find(&concat_str).is_ok() {
        //             if last_match_start_index.is_none() {
        //                 last_match_start_index = Some(idx);
        //             } else {

        //             }
        //         }
        //     }
        //     Ok(())
        // };

        // let accepted_state_index = NodeIndex::from(accepted_state);
        // let mut pathes = Vec::<Vec<NodeIndex<usize>>>::new();
        // let mut stack = Vec::<(NodeIndex<usize>, Vec<NodeIndex<usize>>)>::new();
        // stack.push((accepted_state_index, vec![accepted_state_index]));
        // let mut visited = HashSet::new();

        // while stack.len() != 0 {
        //     println!("stack size {} visited size {}", stack.len(), visited.len());
        //     let (node, path) = stack.pop().unwrap();
        //     println!("node {:?}", node);
        //     println!("is visited {}", visited.contains(&node));
        //     if node.index() == 0 {
        //         println!("found!!");
        //         println!("path {:?}", path);
        //         // pathes.push(path);

        //         continue;
        //     } else {
        //         if visited.contains(&node) {
        //             visited.remove(&node);
        //         } else {
        //             visited.insert(node);
        //             let mut parents = graph.neighbors(node).detach();
        //             let mut parents_set = HashSet::new();
        //             while let Some((_, parent)) = parents.next(&graph) {
        //                 if parent.index() == node.index() {
        //                     continue;
        //                 }
        //                 if !parents_set.contains(&parent) {
        //                     println!("parent  {:?}", parent);
        //                     parents_set.insert(parent);
        //                     if parent.index() == 0 {
        //                         stack.push((node, path.clone()));
        //                     }
        //                     stack.push((parent, vec![path.clone(), vec![parent]].concat()));
        //                 }
        //             }
        //         }
        //     }
        // }
        // println!("pathes {:?}", pathes);
        Ok(())

        // let mut parents = graph.neighbors(accepted_state_index).detach();
        // while let Some((edge_index, parent)) = parents.next(&graph) {
        //     if parent.index() == accepted_state {
        //         continue;
        //     }

        //     stack.push(())
        // }
        // stack.push((accepted_state_index,))

        // let mut nodes: Vec<NodeIndex<usize>> = Vec::new();
        // nodes.push(accepted_state_index);
        // let mut rev_chars: Vec<char> = Vec::new();
        // let mut str_graph =
        //     Graph::<(String, Vec<usize>), (), Directed, usize>::with_capacity(max_state + 1, 0);
        // for idx in 0..=max_state {
        //     str_graph.add_node((String::new(), vec![idx]));
        // }
        // let mut is_checked_edge = HashMap::<String, bool>::new();
        // for i in 0..=max_state {
        //     is_found.insert(NodeIndex::from(i), false);
        // }
        // is_found.insert(accepted_state_index, true);
        // let mut is_visited = HashMap::<NodeIndex<usize>, bool>::new();
        // for i in 0..=max_state {
        //     is_visited.insert(NodeIndex::from(i), false);
        // }
        // is_visited.insert(accepted_state_index, true);

        // let mut parents = graph.neighbors(accepted_state_index).detach();
        // while let Some((edge_index, parent)) = parents.next(&graph) {
        //     if parent.index() == accepted_state {
        //         continue;
        //     }
        //     let edge_str = graph.edge_weight(edge_index).unwrap().to_string();
        //     if is_checked_edge.get(&edge_str).is_none() {
        //         is_checked_edge.insert(edge_str, true);
        //         nodes.push(parent);
        //     }
        // }

        // while nodes.len() != 0 {
        //     // println!("nodes {:?}", nodes);
        //     println!("cur node len {}", nodes.len());
        //     let child: NodeIndex<usize> = nodes.pop().unwrap();
        //     // println!("child {}", child.index());
        //     if child.index() == 0 {
        //         let (str, states) = str_graph.node_weight(child).unwrap();
        //         println!("concat str {}\n", str);
        //         // println!("states {:?}\n", states);
        //         // println!("nodes {:?}", nodes);
        //         continue;
        //     }
        //     let mut parents = graph.neighbors(child).detach();
        //     let mut num_parent: i32 = 0;
        //     while let Some((edge_index, parent)) = parents.next(&graph) {
        //         num_parent += 1;
        //         if child.index() == parent.index() {
        //             // println!("same {}", child.index());
        //             continue;
        //         }
        //         let (child_string, child_nodes) = str_graph.node_weight(child).unwrap();
        //         let new_char = graph.edge_weight(edge_index).unwrap();
        //         let new_str = new_char.to_string() + child_string;
        //         let concat_nodes = vec![vec![parent.index()], child_nodes.clone()].concat();
        //         if is_checked_edge.get(&new_str).is_none() || !is_checked_edge[&new_str] {
        //             println!(
        //                 "num_parent {} child index {} parent index {} new str {}",
        //                 num_parent,
        //                 child.index(),
        //                 parent.index(),
        //                 new_str.as_str()
        //             );
        //             nodes.push(parent);
        //             is_checked_edge.insert(new_str, true);
        //             let new_char = graph.edge_weight(edge_index).unwrap();
        //             let new_str = new_char.to_string() + child_string;
        //             let (parent_str, parent_nodes) = str_graph.node_weight_mut(parent).unwrap();
        //             *parent_str = new_str;
        //             *parent_nodes = concat_nodes;
        //         }
        //     }
        // if !is_visited[&child] {
        //     is_visited.insert(child, true);
        //     let mut parents = graph.neighbors(child).detach();
        //     let mut num_parent: i32 = 0;
        //     while let Some((edge_index, parent)) = parents.next(&graph) {
        //         num_parent += 1;
        //         if child.index() == parent.index() {
        //             // println!("same {}", child.index());
        //             continue;
        //         }
        //         if !is_found[&parent] {
        //             // println!(
        //             //     "num_parent {} child index {} parent index {}",
        //             //     num_parent,
        //             //     child.index(),
        //             //     parent.index()
        //             // );
        //             nodes.push(parent);
        //             is_found.insert(parent, true);
        //             let (child_string, child_nodes) = str_graph.node_weight(child).unwrap();
        //             let mut child_nodes = child_nodes.clone();
        //             let new_char = graph.edge_weight(edge_index).unwrap();
        //             let new_str = new_char.to_string() + child_string;
        //             let (parent_str, parent_nodes) = str_graph.node_weight_mut(parent).unwrap();
        //             *parent_str = new_str;
        //             parent_nodes.append(&mut child_nodes);
        //         }
        //     }
        // }
        // }

        // let mut dfs = Dfs::new(&graph, accepted_state_index.clone());
        // while let Some(parent) = dfs.next(&graph) {
        //     if parent.index() == 0 {
        //         nodes.pop();
        //         rev_chars.pop();
        //         println!("nodes {:?}\n, rev_chars {:?}\n", nodes, rev_chars);
        //     } else {
        //         if nodes.len() == 0 {
        //             nodes.push(accepted_state_index);
        //         }
        //         let child = nodes.last().unwrap();
        //         if child.index() == parent.index() {
        //             println!("same {:?}", child);
        //             continue;
        //         }
        //         println!("child {}, parent {}", child.index(), parent.index());
        //         let child_to_parent = graph.find_edge(*child, parent);
        //         match child_to_parent {
        //             Some(edge_index) => {
        //                 let edge_char = graph
        //                     .edge_weight(edge_index)
        //                     .ok_or(anyhow!("Edge char is not found"))?;
        //                 nodes.push(parent);
        //                 rev_chars.push(*edge_char);
        //             }
        //             None => {
        //                 nodes.pop();
        //                 rev_chars.pop();
        //             }
        //         }
        //     }
        // }

        // while nodes.len() != 0 {
        //     let child = nodes.pop().unwrap();
        //     let mut parents = graph.neighbors(child).detach();
        //     while let Some((edge_index, parent)) = parents.next(&graph) {
        //         let edge_char = graph
        //             .edge_weight(edge_index)
        //             .ok_or(anyhow!("Edge char is not found"))?;
        //     }
        // }

        // let mut concat_regex = String::new();
        // let mut last_dfa_val = vec![];
        // let mut last_max_state = None;
        // let mut last_max_state_only_public = None;
        // let mut num_public = 0;
        // // let mut public_indexes = vec![];
        // // let mut all_substr_states = vec![];

        // for (idx, config) in part_configs.iter().enumerate() {
        //     concat_regex += &config.regex_def; //format!(r#"{}{}"#, concat_regex, config.regex_def);
        //     last_dfa_val = get_dfa_json_value(&concat_regex)?;
        //     let next_max_state = get_max_state(&last_dfa_val)?;
        //     let accepted_state = get_accepted_state(&last_dfa_val).unwrap();
        //     println!(
        //         "next_max_state {} accepted_state {} num_public {}",
        //         next_max_state, accepted_state, num_public
        //     );
        //     if config.is_public {
        //         let mut writer = BufWriter::new(File::create(id_to_substr_path(num_public))?);
        //         writer.write_fmt(format_args!("{}\n", &config.max_size))?;
        //         writer.write_fmt(format_args!("0\n{}\n", self.max_body_size - 1))?;
        //         // public_indexes.push(idx);
        //         add_graph_nodes(
        //             &last_dfa_val,
        //             &mut graph,
        //             last_max_state_only_public,
        //             next_max_state,
        //         )?;
        //         let mut substr_states = Vec::<(usize, usize)>::new();
        //         let susbtr_start = match last_max_state {
        //             Some(v) => NodeIndex::from(v),
        //             None => NodeIndex::from(0),
        //         };
        //         let mut nodes: VecDeque<NodeIndex<usize>> = VecDeque::new();
        //         nodes.push_back(NodeIndex::from(accepted_state));
        //         while nodes.len() != 0 {
        //             let child = nodes.pop_front().unwrap();
        //             // let weight = graph.node_weight(NodeIndex::from(child)).unwrap();
        //             // if *weight {
        //             //     substr_states.push((child.index(), child.index()));
        //             // }
        //             let mut parents = graph.neighbors(child).detach();
        //             while let Some((_, parent)) = parents.next(&graph) {
        //                 substr_states.push((parent.index(), child.index()));
        //                 if susbtr_start != parent {
        //                     writer.write_fmt(format_args!(
        //                         "{} {}\n",
        //                         parent.index(),
        //                         child.index()
        //                     ))?;
        //                 }
        //             }
        //             // for (_, parent) in graph.neighbors(child).enumerate() {
        //             //     substr_states.push((parent.index(), child.index()));
        //             //     if susbtr_start != parent {
        //             //         nodes.push_back(parent);
        //             //     }
        //             // }
        //         }
        //         // all_substr_states.push(substr_states);
        //         last_max_state_only_public = Some(next_max_state);
        //         num_public += 1;
        //         writer.flush()?;
        //     }
        //     last_max_state = Some(next_max_state);
        // }

        // let num_rules = all_substr_states.len();
        // for id in 0..num_rules {
        //     let mut text = String::new();
        //     let index = public_indexes[id];
        //     let config = &part_configs[index];
        //     text += &format!("{}\n", config.max_size);
        //     text += &format!("0\n{}\n", self.max_body_size - 1);
        //     for (parent, child) in all_substr_states[id].iter() {
        //         text += &format!("{} {}\n", parent, child);
        //     }
        //     let mut substr_file = File::create(id_to_substr_path(id))?;
        //     write!(substr_file, "{}", text)?;
        //     substr_file.flush()?;
        // }
    }
}
