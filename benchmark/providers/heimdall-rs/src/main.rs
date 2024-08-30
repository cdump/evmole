use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::{env, fs};

use heimdall_core::heimdall_decompiler::DecompilerArgsBuilder;

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

    let selectors: HashMap<String, Vec<String>> = if mode == "selectors" {
        HashMap::new()
    } else {
        let file_content = fs::read_to_string(&args[4])?;
        serde_json::from_str(&file_content)?
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

        let result = heimdall_core::heimdall_decompiler::decompile(dargs).await;
        if let Err(e) = result {
            println!("got error for {}: {}", fname, e);
            if mode == "selectors" {
                ret_selectors.insert(fname, Vec::new());
            } else {
                let r: HashMap<String, String> = selectors[&fname].iter().map(|s| (s.to_string(), "not_found".to_string())).collect();
                ret_arguments.insert(fname, r);
            }
            continue
        }
        let abi = result?.abi.functions;

        if mode == "selectors" {
            let r: Vec<String> = abi
                .keys()
                .map(|x| x.strip_prefix("Unresolved_").unwrap().to_string())
                .collect();
            ret_selectors.insert(fname, r);
        } else {
            let args: HashMap<String, String> = abi
                .iter()
                .map(|(name, v)| {
                    let selector = name.strip_prefix("Unresolved_").unwrap().to_string();
                    let info = if mode == "arguments" {
                        let a: Vec<_> = v[0].inputs.iter().map(|v| v.ty.to_string()).collect();
                        a.join(",")
                    } else {
                        v[0].state_mutability.as_json_str().to_string()
                    };
                    (selector, info)
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
        }
    }

    let file = fs::File::create(outfile)?;
    let mut bw = BufWriter::new(file);
    if mode == "selectors" {
        let _ = serde_json::to_writer(&mut bw, &ret_selectors);
    } else {
        let _ = serde_json::to_writer(&mut bw, &ret_arguments);
    }
    bw.flush()?;

    Ok(())
}
