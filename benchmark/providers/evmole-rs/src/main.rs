use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

use clap::{Parser, ValueEnum};
use hex::FromHex;

#[derive(serde::Deserialize)]
struct Input {
    code: Option<String>,
    runtimeBytecode: Option<String>,
}

#[derive(ValueEnum, Clone, PartialEq)]
enum Mode {
    Selectors,
    Arguments,
    Mutability,
    Storage,
}

#[derive(Parser)]
struct Args {
    mode: Mode,

    input_dir: String,

    output_file: String,

    selectors_file: Option<String>,

    #[arg(long)]
    filter_filename: Option<String>,

    #[arg(long)]
    filter_selector: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();

    type Meta = u64; // duration in ms

    let selectors: HashMap<String, (Meta, Vec<String>)> = match cfg.mode {
        Mode::Selectors | Mode::Storage => HashMap::new(),
        Mode::Arguments | Mode::Mutability => {
            let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
            serde_json::from_str(&file_content)?
        }
    };

    let only_selector = if let Some(s) = cfg.filter_selector {
        vec![s.strip_prefix("0x").unwrap_or(&s).to_string()]
    } else {
        vec![]
    };

    let mut ret_selectors: HashMap<String, (Meta, Vec<String>)> = HashMap::new();
    let mut ret_other: HashMap<String, (Meta, HashMap<String, String>)> = HashMap::new();

    for entry in fs::read_dir(cfg.input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();

        if let Some(ref v) = cfg.filter_filename {
            if !fname.contains(v) {
                continue;
            }
        }

        let code = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            let code = if v.runtimeBytecode.is_some() {
                v.runtimeBytecode.unwrap()
            } else {
                v.code.unwrap()
            };
            hex::decode(code.strip_prefix("0x").expect("0x prefix expected"))?
        };

        // eprintln!("processing {}", fname);

        match cfg.mode {
            Mode::Selectors => {
                let now = Instant::now();
                let info =
                    evmole::contract_info(evmole::ContractInfoArgs::new(&code).with_selectors());
                let dur = now.elapsed().as_millis() as u64;
                ret_selectors.insert(
                    fname,
                    (
                        dur,
                        info.functions
                            .unwrap()
                            .iter()
                            .map(|f| hex::encode(f.selector))
                            .collect(),
                    ),
                );
            }
            Mode::Arguments => {
                let fsel = if !only_selector.is_empty() {
                    &only_selector
                } else {
                    &selectors[&fname].1
                };

                let now = Instant::now();
                let info =
                    evmole::contract_info(evmole::ContractInfoArgs::new(&code).with_arguments());
                let dur = now.elapsed().as_millis() as u64;

                let args: HashMap<String, String> = info
                    .functions
                    .unwrap()
                    .into_iter()
                    .map(|f| {
                        (
                            hex::encode(f.selector),
                            f.arguments
                                .unwrap()
                                .iter()
                                .map(|t| t.sol_type_name().to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                        )
                    })
                    .collect();

                let res = fsel
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
                    .collect();

                ret_other.insert(fname, (dur, res));
            }
            Mode::Mutability => {
                let fsel = if !only_selector.is_empty() {
                    &only_selector
                } else {
                    &selectors[&fname].1
                };

                let now = Instant::now();
                let info = evmole::contract_info(
                    evmole::ContractInfoArgs::new(&code).with_state_mutability(),
                );
                let dur = now.elapsed().as_millis() as u64;

                let smut: HashMap<String, String> = info
                    .functions
                    .unwrap()
                    .into_iter()
                    .map(|f| {
                        (
                            hex::encode(f.selector),
                            f.state_mutability.unwrap().as_json_str().to_string(),
                        )
                    })
                    .collect();

                let res = fsel
                    .iter()
                    .map(|s| {
                        (
                            s.to_string(),
                            match smut.get(s) {
                                Some(v) => v.to_string(),
                                None => "not_found".to_string(),
                            },
                        )
                    })
                    .collect();

                ret_other.insert(fname, (dur, res));
            }

            Mode::Storage => {
                let now = Instant::now();
                let info =
                    evmole::contract_info(evmole::ContractInfoArgs::new(&code).with_storage());
                let dur = now.elapsed().as_millis() as u64;
                ret_other.insert(
                    fname,
                    (
                        dur,
                        info.storage
                            .unwrap()
                            .into_iter()
                            .map(|sr| {
                                (format!("{}_{}", hex::encode(sr.slot), sr.offset), sr.r#type)
                            })
                            .collect(),
                    ),
                );
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
