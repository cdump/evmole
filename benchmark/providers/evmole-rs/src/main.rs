use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::fs;

use clap::Parser;

use hex::FromHex;

#[derive(serde::Deserialize)]
struct Input {
    code: String,
}

#[derive(Parser, Debug)]
struct Args {
    mode: String,

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

    let selectors: HashMap<String, Vec<String>> = if cfg.mode == "arguments" {
        let file_content = fs::read_to_string(cfg.selectors_file.unwrap())?;
        serde_json::from_str(&file_content)?
    } else {
        HashMap::new()
    };

    let only_selector = if let Some(s) = cfg.filter_selector {
        vec![s.strip_prefix("0x").unwrap_or(&s).to_string()]
    } else {
        vec![]
    };

    let mut ret_selectors: HashMap<String, Vec<String>> = HashMap::new();
    let mut ret_arguments: HashMap<String, HashMap<String, String>> = HashMap::new();

    for entry in fs::read_dir(cfg.input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();

        if let Some(ref v) = cfg.filter_filename {
            if !fname.contains(v) {
                continue
            }
        }

        let hex_code: String = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            v.code
        };
        let code = hex::decode(hex_code.strip_prefix("0x").unwrap())?;

        // println!("processing {}", fname);

        if cfg.mode == "arguments" {
            let fsel = if !only_selector.is_empty() {
                &only_selector
            } else {
                &selectors[&fname]
            };

            let r: HashMap<String, String> = fsel
                .iter()
                .map(|s| {
                    let selector = <[u8; 4]>::from_hex(s).unwrap();
                    (
                        s.to_string(),
                        evmole::function_arguments(&code, &selector, 0),
                    )
                })
                .collect();

            ret_arguments.insert(fname, r);
        } else {
            let r = evmole::function_selectors(&code, 0);
            ret_selectors.insert(fname, r.iter().map(hex::encode).collect());
        }
    }

    let file = fs::File::create(cfg.output_file)?;
    let mut bw = BufWriter::new(file);
    if cfg.mode == "arguments" {
        let _ = serde_json::to_writer(&mut bw, &ret_arguments);
    } else {
        let _ = serde_json::to_writer(&mut bw, &ret_selectors);
    }
    bw.flush()?;

    Ok(())
}
