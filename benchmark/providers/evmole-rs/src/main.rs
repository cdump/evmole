use std::collections::HashMap;
use std::io::Write;
use std::{env, fs, process};

use hex::FromHex;

#[derive(serde::Deserialize)]
struct Input {
    code: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: ./main MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]");
        process::exit(1);
    }
    let mode = &args[1];
    let indir = &args[2];
    let outfile = &args[3];

    let selectors: HashMap<String, Vec<String>> = if mode == "arguments" {
        let file_content = fs::read_to_string(&args[4])?;
        serde_json::from_str(&file_content)?
    } else {
        HashMap::new()
    };

    let mut ret_selectors: HashMap<String, Vec<String>> = HashMap::new();
    let mut ret_arguments: HashMap<String, HashMap<String, String>> = HashMap::new();

    for entry in fs::read_dir(indir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();
        let hex_code: String = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            v.code
        };
        let code = hex::decode(hex_code.strip_prefix("0x").unwrap())?;

        // println!("processing {}", fname);

        if mode == "arguments" {
            let r: HashMap<String, String> = selectors[&fname]
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

    let mut file = fs::File::create(outfile)?;
    if mode == "arguments" {
        let _ = serde_json::to_writer(&mut file, &ret_arguments);
    } else {
        let _ = serde_json::to_writer(&mut file, &ret_selectors);
    }
    file.flush()?;

    Ok(())
}
