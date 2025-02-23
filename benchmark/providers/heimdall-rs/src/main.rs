use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

use clap::{Parser, ValueEnum};

use heimdall_core::heimdall_decompiler::DecompilerArgsBuilder;

#[derive(Debug, serde::Deserialize)]
struct Input {
    code: String,
}

#[derive(ValueEnum, Clone, PartialEq)]
enum Mode {
    Selectors,
    Arguments,
    Mutability,
    Flow,
}

#[derive(Parser)]
struct Args {
    mode: Mode,

    input_dir: String,

    output_file: String,

    selectors_file: Option<String>,
}

async fn measure_time<T, F>(f: F) -> (T, u64)
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f.await;
    let duration_us = start.elapsed().as_micros() as u64;
    (result, duration_us)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();

    type Meta = u64; // duration in us

    let selectors: HashMap<String, (Meta, Vec<String>)> = match cfg.mode {
        Mode::Selectors | Mode::Flow => HashMap::new(),
        Mode::Arguments | Mode::Mutability => {
            let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
            serde_json::from_str(&file_content)?
        }
    };

    let mut ret_selectors: HashMap<String, (Meta, Vec<String>)> = HashMap::new();
    let mut ret_other: HashMap<String, (Meta, HashMap<String, String>)> = HashMap::new();
    let mut ret_flow: HashMap<String, (Meta, BTreeSet<(usize, usize)>)> = HashMap::new();

    for entry in fs::read_dir(cfg.input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();
        let hex_code: String = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            v.code
        };
        match cfg.mode {
            Mode::Selectors => {
                let dargs = DecompilerArgsBuilder::new()
                    .target(hex_code)
                    .skip_resolving(true)
                    .build()?;

                let (result, duration_us) = measure_time(heimdall_core::heimdall_decompiler::decompile(dargs)).await;

                let r = match result {
                    Err(e) => {
                        println!("got error for {}: {}", fname, e);
                        vec![]
                    }
                    Ok(v) => v
                        .abi
                        .functions
                        .keys()
                        .map(|x| x.strip_prefix("Unresolved_").unwrap().to_string())
                        .collect(),
                };
                ret_selectors.insert(fname, (duration_us, r));
            }
            Mode::Arguments => {
                let dargs = DecompilerArgsBuilder::new()
                    .target(hex_code)
                    .skip_resolving(true)
                    .build()?;

                let (result, duration_us) = measure_time(heimdall_core::heimdall_decompiler::decompile(dargs)).await;

                let r = match result {
                    Err(e) => {
                        println!("got error for {}: {}", fname, e);
                        selectors[&fname]
                            .1
                            .iter()
                            .map(|s| (s.to_string(), "not_found".to_string()))
                            .collect()
                    }
                    Ok(v) => {
                        let args: HashMap<String, String> = v
                            .abi
                            .functions
                            .iter()
                            .map(|(name, v)| {
                                let selector =
                                    name.strip_prefix("Unresolved_").unwrap().to_string();
                                let arguments: Vec<_> =
                                    v[0].inputs.iter().map(|v| v.ty.to_string()).collect();
                                (selector, arguments.join(","))
                            })
                            .collect();

                        selectors[&fname]
                            .1
                            .iter()
                            .map(|s| {
                                (
                                    s.to_string(),
                                    match args.get(s) {
                                        Some(v) => v.to_string(),
                                        None => "not_found".to_string(),
                                    },
                                )
                            })
                            .collect()
                    }
                };
                ret_other.insert(fname, (duration_us, r));
            }
            Mode::Mutability => {
                let dargs = DecompilerArgsBuilder::new()
                    .target(hex_code)
                    .skip_resolving(true)
                    .build()?;

                let (result, duration_us) = measure_time(heimdall_core::heimdall_decompiler::decompile(dargs)).await;

                let r = match result {
                    Err(e) => {
                        println!("got error for {}: {}", fname, e);
                        selectors[&fname]
                            .1
                            .iter()
                            .map(|s| (s.to_string(), "not_found".to_string()))
                            .collect()
                    }
                    Ok(v) => {
                        let args: HashMap<String, String> = v
                            .abi
                            .functions
                            .iter()
                            .map(|(name, v)| {
                                let selector =
                                    name.strip_prefix("Unresolved_").unwrap().to_string();
                                let mutability = v[0].state_mutability.as_json_str().to_string();
                                (selector, mutability)
                            })
                            .collect();

                        selectors[&fname]
                            .1
                            .iter()
                            .map(|s| {
                                (
                                    s.to_string(),
                                    match args.get(s) {
                                        Some(v) => v.to_string(),
                                        None => "not_found".to_string(),
                                    },
                                )
                            })
                            .collect()
                    }
                };
                ret_other.insert(fname, (duration_us, r));
            }
            Mode::Flow => {
                let cfg_args = heimdall_core::heimdall_cfg::CfgArgsBuilder::new()
                    .target(hex_code)
                    .build()?;

                let (result, duration_us) = measure_time(heimdall_core::heimdall_cfg::cfg(cfg_args)).await;
                let cfg = result?;

                let mut jump_dest_mapping: HashMap<usize, usize> = HashMap::new();
                let mut control_flow: BTreeSet<(usize, usize)> = BTreeSet::new();

                // Helper function to parse hex addresses
                fn parse_hex_address(hex_str: &str) -> usize {
                    let s = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                    usize::from_str_radix(s, 16).unwrap()
                }

                // Split blocks by JUMPDEST
                for node in cfg.graph.raw_nodes() {
                    let instructions: Vec<_> = node.weight
                        .lines()
                        .filter_map(|line| {
                            let (pc_hex, op) = line.trim_end().split_once(" ")?;
                            let pc = parse_hex_address(pc_hex);
                            Some((pc, op))
                        })
                        .collect();

                    let mut current_pc = instructions[0].0;
                    for (pc, op) in instructions.iter().skip(1) {
                        if *op == "JUMPDEST" {
                            jump_dest_mapping.insert(instructions[0].0, *pc);
                            control_flow.insert((current_pc, *pc));
                            current_pc = *pc;
                        }
                    }
                }

                // Process edges
                control_flow.extend(cfg.graph.raw_edges().iter().map(|edge| {
                    let source = cfg.graph.node_weight(edge.source())
                        .and_then(|s| s.split_once(" "))
                        .map(|(hex, _)| parse_hex_address(hex))
                        .unwrap();

                    let target = cfg.graph.node_weight(edge.target())
                        .and_then(|s| s.split_once(" "))
                        .map(|(hex, _)| parse_hex_address(hex))
                        .unwrap();

                    let from = jump_dest_mapping.get(&source).copied().unwrap_or(source);
                    (from, target)
                }));

                ret_flow.insert(fname, (duration_us, control_flow));
            }
        }
    }

    let file = fs::File::create(cfg.output_file)?;
    let mut bw = BufWriter::new(file);
    if cfg.mode == Mode::Selectors {
        let _ = serde_json::to_writer(&mut bw, &ret_selectors);
    } else if cfg.mode == Mode::Flow {
        let _ = serde_json::to_writer(&mut bw, &ret_flow);
    } else {
        let _ = serde_json::to_writer(&mut bw, &ret_other);
    }
    bw.flush()?;

    Ok(())
}
