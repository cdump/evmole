use std::collections::HashMap;
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
}

#[derive(Parser)]
struct Args {
    mode: Mode,

    input_dir: String,

    output_file: String,

    selectors_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();

    type Meta = u64; // duration in ms

    let selectors: HashMap<String, (Meta, Vec<String>)> = match cfg.mode {
        Mode::Selectors => HashMap::new(),
        Mode::Arguments | Mode::Mutability => {
            let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
            serde_json::from_str(&file_content)?
        }
    };

    let mut ret_selectors: HashMap<String, (Meta, Vec<String>)> = HashMap::new();
    let mut ret_other: HashMap<String, (Meta, HashMap<String, String>)> = HashMap::new();

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
                let now = Instant::now();
                let result = heimdall_core::heimdall_decompiler::decompile(dargs).await;
                let duration_ms = now.elapsed().as_millis() as u64;
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
                ret_selectors.insert(fname, (duration_ms, r));
            }
            Mode::Arguments => {
                let dargs = DecompilerArgsBuilder::new()
                    .target(hex_code)
                    .skip_resolving(true)
                    .build()?;
                let now = Instant::now();
                let result = heimdall_core::heimdall_decompiler::decompile(dargs).await;
                let duration_ms = now.elapsed().as_millis() as u64;
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
                ret_other.insert(fname, (duration_ms, r));
            }
            Mode::Mutability => {
                let dargs = DecompilerArgsBuilder::new()
                    .target(hex_code)
                    .skip_resolving(true)
                    .build()?;
                let now = Instant::now();
                let result = heimdall_core::heimdall_decompiler::decompile(dargs).await;
                let duration_ms = now.elapsed().as_millis() as u64;
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
                ret_other.insert(fname, (duration_ms, r));
            }
        }
    }

    let file = fs::File::create(cfg.output_file)?;
    let mut bw = BufWriter::new(file);
    if cfg.mode == Mode::Selectors {
        let _ = serde_json::to_writer(&mut bw, &ret_selectors);
    } else {
        let _ = serde_json::to_writer(&mut bw, &ret_other);
    }
    bw.flush()?;

    Ok(())
}
