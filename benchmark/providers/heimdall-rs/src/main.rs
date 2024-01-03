use std::collections::HashMap;
use std::io::Write;
use std::{env, fs};

use heimdall_core::decompile::out::abi::ABIStructure;
use heimdall_core::decompile::DecompilerArgsBuilder;

#[derive(Debug, serde::Deserialize)]
struct Input {
    code: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: ./main MODE INPUT_DIR OUTPUT_FILE [SELECTORS_FILE]");
        std::process::exit(1);
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

        let dargs = DecompilerArgsBuilder::new()
            .target(hex_code)
            .skip_resolving(true)
            .build()?;

        // println!("processing {}", fname);

        //if fname == "0x940259178FbF021e625510919BC2FF0B944E5613.json" {
        //    if mode == "arguments" {
        //        let r: HashMap<String, String> = selectors[&fname].iter().map(|s| (s.to_string(), "not_found".to_string())).collect();
        //        ret_arguments.insert(fname, r);
        //    } else {
        //        ret_selectors.insert(fname, Vec::new());
        //    }
        //    continue
        //}

        let result = heimdall_core::decompile::decompile(dargs).await?;
        let abi = result.abi.unwrap();

        if mode == "arguments" {
            let args: HashMap<String, String> = abi
                .iter()
                .filter_map(|e| match e {
                    ABIStructure::Function(v) => {
                        let selector = v.name.strip_prefix("Unresolved_").unwrap().to_string();
                        let a: Vec<_> = v.inputs.iter().map(|v| v.type_.to_string()).collect();
                        let args = a.join(",");
                        Some((selector, args))
                    }
                    _ => None,
                })
                .collect();

            let r: HashMap<String, String> = selectors[&fname]
                .iter()
                .map(|s| match args.get(s) {
                    Some(v) => (s.to_string(), v.to_string()),
                    None => (s.to_string(), "not_found".to_string()),
                })
                .collect();
            ret_arguments.insert(fname, r);
        } else {
            let r: Vec<String> = abi
                .iter()
                .filter_map(|e| match e {
                    ABIStructure::Function(v) => {
                        Some(v.name.strip_prefix("Unresolved_").unwrap().to_string())
                    }
                    _ => None,
                })
                .collect();
            ret_selectors.insert(fname, r);
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
