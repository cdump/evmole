use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};

use clap::{Parser, ValueEnum};
use hex::FromHex;

#[derive(serde::Deserialize)]
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

    #[arg(long)]
    filter_filename: Option<String>,

    #[arg(long)]
    filter_selector: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();

    let selectors: HashMap<String, Vec<String>> = match cfg.mode {
        Mode::Selectors => HashMap::new(),
        _ => {
            let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
            serde_json::from_str(&file_content)?
        }
    };

    let only_selector = if let Some(s) = cfg.filter_selector {
        vec![s.strip_prefix("0x").unwrap_or(&s).to_string()]
    } else {
        vec![]
    };

    let mut ret_selectors: HashMap<String, Vec<String>> = HashMap::new();
    let mut ret_other: HashMap<String, HashMap<String, String>> = HashMap::new();

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
            hex::decode(v.code.strip_prefix("0x").expect("0x prefix expected"))?
        };

        // println!("processing {}", fname);

        match cfg.mode {
            Mode::Selectors => {
                let r = evmole::function_selectors(&code, 0);
                ret_selectors.insert(fname, r.iter().map(hex::encode).collect());
            }
            Mode::Arguments | Mode::Mutability => {
                let fsel = if !only_selector.is_empty() {
                    &only_selector
                } else {
                    &selectors[&fname]
                };

                ret_other.insert(
                    fname,
                    fsel.iter()
                        .map(|s| {
                            let selector = <[u8; 4]>::from_hex(s).unwrap();
                            (
                                s.to_string(),
                                match cfg.mode {
                                    Mode::Arguments => {
                                        evmole::function_arguments(&code, &selector, 0)
                                    }
                                    Mode::Mutability => {
                                        evmole::function_state_mutability(&code, &selector, 0)
                                            .as_json_str()
                                            .to_string()
                                    }
                                    _ => panic!("impossible mode"),
                                },
                            )
                        })
                        .collect(),
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
