use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::{env, fs};

#[derive(Debug, serde::Deserialize)]
struct Input {
    code: String,
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: ./main MODE INPUT_DIR OUTPUT_FILE");
        std::process::exit(1);
    }
    let mode = &args[1];
    let indir = &args[2];
    let outfile = &args[3];

    if mode != "selectors" {
        eprintln!("Only 'selectors' mode supported");
        std::process::exit(1);
    }

    let mut ret: HashMap<String, Vec<String>> = HashMap::new();
    for entry in fs::read_dir(indir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();

        let code: Vec<u8> = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            let x = v.code.strip_prefix("0x").unwrap();
            hex::decode(x).unwrap()
        };

        let string_selectors: Vec<_> = evm_hound::selectors_from_bytecode(&code)
            .into_iter()
            .map(hex::encode)
            .collect();

        ret.insert(fname, string_selectors);
    }

    let file = fs::File::create(outfile)?;
    let mut bw = BufWriter::new(file);
    let _ = serde_json::to_writer(&mut bw, &ret);
    bw.flush()?;

    Ok(())
}
