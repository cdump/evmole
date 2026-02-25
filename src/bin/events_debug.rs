use std::{fs, path::PathBuf, time::Instant};

use evmole::{EventSelector, contract_events};

#[derive(Debug, Default)]
struct Args {
    code_hex: Option<String>,
    code_file: Option<PathBuf>,
    raw_file: Option<PathBuf>,
    iters: usize,
    warmup: usize,
    show_events: bool,
}

fn usage() -> &'static str {
    "Usage:
  cargo run --release --bin events_debug -- [OPTIONS]

Options:
  --code-hex <HEX>      Bytecode hex string (with or without 0x prefix)
  --code-file <PATH>    Text file containing bytecode hex
  --raw-file <PATH>     Raw bytecode file
  --iters <N>           Timed iterations (default: 1)
  --warmup <N>          Warmup iterations (default: 0)
  --show-events         Print extracted event selectors
  -h, --help            Show this help

Exactly one of --code-hex / --code-file / --raw-file must be provided."
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        iters: 1,
        warmup: 0,
        show_events: false,
        ..Default::default()
    };

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            "--code-hex" => {
                args.code_hex = Some(it.next().ok_or("--code-hex requires a value")?);
            }
            "--code-file" => {
                args.code_file = Some(PathBuf::from(
                    it.next().ok_or("--code-file requires a value")?,
                ));
            }
            "--raw-file" => {
                args.raw_file = Some(PathBuf::from(
                    it.next().ok_or("--raw-file requires a value")?,
                ));
            }
            "--iters" => {
                let v = it.next().ok_or("--iters requires a value")?;
                args.iters = v.parse().map_err(|_| format!("invalid --iters: {v}"))?;
            }
            "--warmup" => {
                let v = it.next().ok_or("--warmup requires a value")?;
                args.warmup = v.parse().map_err(|_| format!("invalid --warmup: {v}"))?;
            }
            "--show-events" => args.show_events = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let inputs = [
        args.code_hex.is_some(),
        args.code_file.is_some(),
        args.raw_file.is_some(),
    ]
    .into_iter()
    .filter(|v| *v)
    .count();
    if inputs != 1 {
        return Err("provide exactly one of --code-hex / --code-file / --raw-file".to_string());
    }
    if args.iters == 0 {
        return Err("--iters must be >= 1".to_string());
    }
    Ok(args)
}

fn decode_hex(input: &str) -> Result<Vec<u8>, String> {
    let s = input.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    alloy_primitives::hex::decode(s).map_err(|e| format!("hex decode failed: {e}"))
}

fn load_code(args: &Args) -> Result<Vec<u8>, String> {
    if let Some(hex) = &args.code_hex {
        return decode_hex(hex);
    }
    if let Some(path) = &args.code_file {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("failed to read code file '{}': {e}", path.display()))?;
        return decode_hex(&text);
    }
    if let Some(path) = &args.raw_file {
        return fs::read(path)
            .map_err(|e| format!("failed to read raw file '{}': {e}", path.display()));
    }
    Err("no input provided".to_string())
}

fn fmt_selector(s: &EventSelector) -> String {
    alloy_primitives::hex::encode(s)
}

fn main() {
    let args = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };

    let code = match load_code(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    if code.is_empty() {
        eprintln!("error: empty bytecode");
        std::process::exit(2);
    }

    for _ in 0..args.warmup {
        let _ = contract_events(&code);
    }

    let mut last_events = Vec::new();
    let mut elapsed_ms = Vec::with_capacity(args.iters);

    for _ in 0..args.iters {
        let t0 = Instant::now();
        let events = contract_events(&code);
        let dt = t0.elapsed().as_secs_f64() * 1000.0;
        elapsed_ms.push(dt);
        last_events = events;
    }

    let total_ms: f64 = elapsed_ms.iter().sum();
    let avg_ms = total_ms / elapsed_ms.len() as f64;
    let min_ms = elapsed_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let max_ms = elapsed_ms.iter().copied().fold(0.0, f64::max);

    println!("code_len: {}", code.len());
    println!(
        "time_ms: avg={avg_ms:.3} min={min_ms:.3} max={max_ms:.3} (iters={})",
        args.iters
    );
    println!("events: {}", last_events.len());

    if args.show_events {
        last_events.sort_unstable();
        for evt in &last_events {
            println!("event: {}", fmt_selector(evt));
        }
    }
}
