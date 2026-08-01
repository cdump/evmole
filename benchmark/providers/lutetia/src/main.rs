use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

use clap::{Parser, ValueEnum};

#[derive(Debug, serde::Deserialize)]
struct Input {
    code: Option<String>,

    #[serde(rename = "runtimeBytecode")]
    runtime_bytecode: Option<String>,
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
}

use lutetia::contract::Contract;
use lutetia::decompiler::{DecompilerConfig, OutputFormat, decompile_bytecode};
use lutetia::sparser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();

    type Meta = u64; // duration in ms

    let selectors: HashMap<String, (Meta, Vec<String>)> = match cfg.mode {
        Mode::Arguments | Mode::Mutability => {
            let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
            serde_json::from_str(&file_content)?
        }
        _ => HashMap::default(),
    };

    let config = DecompilerConfig {
        timeout_secs: 5,
        format: OutputFormat::Text,
        color: false,
    };

    let mut ret_selectors: HashMap<String, (Meta, Vec<String>)> = HashMap::new();
    let mut ret_other: HashMap<String, (Meta, HashMap<String, String>)> = HashMap::new();

    let mut ok = false;
    for entry in fs::read_dir(cfg.input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();

        let hex_code = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            if v.runtime_bytecode.is_some() {
                v.runtime_bytecode.unwrap()
            } else {
                v.code.unwrap()
            }
        };

        eprintln!("processing {}", fname);
        // if fname == "0x852b9435d1373C7E1d51dd52AeBd2aA54422c30D.json" {
        //     ok = true;
        //     continue;
        // }
        // if !ok {
        //     continue;
        // }

        let cache_file = format!("cache/{}", fname);

        let (duration_us, mut info) = if let Ok(f) = fs::File::open(&cache_file)
            && false
        {
            let rd = std::io::BufReader::new(f);
            let (duration_us, info): (u64, Contract) = serde_json::from_reader(rd).unwrap();
            (duration_us, info)
            // serde_json::re
        } else {
            // continue;

            let now = Instant::now();
            let info = decompile_bytecode(&hex_code, &config).unwrap().contract;
            let duration_us = now.elapsed().as_micros() as u64;
            // let file = fs::File::create(cache_file)?;
            // let mut bw = BufWriter::new(file);
            // serde_json::to_writer(&mut bw, &(duration_us, &info))?;
            // bw.flush()?;
            (duration_us, info)
        };

        match cfg.mode {
            Mode::Selectors => {
                ret_selectors.insert(
                    fname,
                    (
                        duration_us,
                        info.functions.into_iter().map(|f| f.hash).collect(),
                    ),
                );
            }
            Mode::Arguments => {
                let args: HashMap<String, String> = info
                    .functions
                    .into_iter()
                    .map(|f| {
                        (
                            f.hash[2..].to_string(), // strip 0x
                            f.params
                                .into_iter()
                                .map(|t| t.kind)
                                .collect::<Vec<String>>()
                                .join(","),
                        )
                    })
                    .collect();

                let res = selectors[&fname]
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
                    .collect();

                ret_other.insert(fname, (duration_us, res));
            }

            Mode::Mutability => {
                let smut: HashMap<String, String> = info
                    .functions
                    .into_iter()
                    .map(|f| {
                        let val = match (f.payable, f.read_only, f.is_const) {
                            (true, _, _) => "payable",
                            (false, false, false) => "nonpayable",
                            (_, true, false) => "view",
                            (_, _, true) => "pure",
                        }
                        .to_string();
                        (
                            f.hash[2..].to_string(), // strip 0x
                            val,
                        )
                    })
                    .collect();

                let res = selectors[&fname]
                    .1
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

                ret_other.insert(fname, (duration_us, res));
            }

            Mode::Storage => {
                let storage_defs = sparser::rewrite_functions(&mut info.functions);
                for def in storage_defs {
                    //         let type_str = match &def.kind {
                    //             sparser::StorageKind::Simple { size, offset } => {
                    //                 let type_name = match size {
                    //                     160 => "address".to_string(),
                    //                     256 => "uint256".to_string(),
                    //                     8 => "uint8".to_string(),
                    //                     s => format!("uint{s}"),
                    //                 };
                    //                 if *offset > 0 {
                    //                     format!("{type_name} at storage {} offset {}", def.slot, offset / 256)
                    //                 } else {
                    //                     format!("{type_name} at storage {}", def.slot)
                    //                 }
                    //             }
                    //             sparser::StorageKind::Mapping { value_size } => {
                    //                 let val_type = match value_size {
                    //                     160 => "address",
                    //                     256 => "uint256",
                    //                     8 => "uint8",
                    //                     _ => "uint256",
                    //                 };
                    //                 format!("mapping of {val_type} at storage {}", def.slot)
                    //             }
                    //             sparser::StorageKind::Array { element_size } => {
                    //                 let elem_type = match element_size {
                    //                     160 => "address",
                    //                     256 => "uint256",
                    //                     _ => "uint256",
                    //                 };
                    //                 format!("array of {elem_type} at storage {}", def.slot)
                    //             }
                    //             sparser::StorageKind::Struct { field_count } => {
                    //                 format!("struct ({field_count} fields) at storage {}", def.slot)
                    //             }
                }
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
