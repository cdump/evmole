use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};
use clap::{Parser, ValueEnum};

use storage_layout_extractor::{
    self,
    watchdog::Watchdog,
    extractor::{
        chain::{
            version::{ChainVersion, EthereumVersion},
            Chain,
        },
        contract::Contract,
    },
    tc,
    vm,
};

#[derive(serde::Deserialize)]
struct Input {
    #[serde(rename = "runtimeBytecode")]
    code: String,
}

#[derive(ValueEnum, Clone, PartialEq)]
enum Mode {
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

// TODO: improve this, validate with github upstream (new issue)
fn type_str(tp: &tc::abi::AbiType) -> String {
    use tc::abi::AbiType::*;
    // eprintln!("{:?}", tp);
    match tp {
        Any => "uint256".to_string(),
        Number { size } | UInt { size } => format!("uint{}", size.unwrap_or(256)),
        Int { size } => format!("int{}", size.unwrap_or(256)),
        Address => "address".to_string(),
        Selector => "err_selector".to_string(),
        Function => "err_function".to_string(),
        Bool => "bool".to_string(),
        Array { size, tp } => format!("{}[{}]", type_str(tp), u16::try_from(size.0).unwrap_or(1)),
        Bytes { length } => format!("bytes{}", length.unwrap_or(32)),
        Bits { length } => "errbits".to_string(),
        DynArray { tp } => format!("{}[]", type_str(tp)),
        DynBytes => "bytes".to_string(),
        Mapping { key_type, value_type } => format!("mapping({} => {})", type_str(key_type), type_str(value_type)),
        Struct { elements } => {
            "struct".to_string()
        },
        InfiniteType => "uint256".to_string(),
        ConflictedType { conflicts, reasons } => "err_conflict".to_string(),
    }
}

#[derive(Debug)]
struct MyWatchDog {
    pub end: Instant,
}

impl MyWatchDog {
    fn new(time_limit: Duration) -> Self {
        MyWatchDog {
            end: Instant::now() +  time_limit,
        }
    }
}

impl Watchdog for MyWatchDog {
    fn should_stop(&self) -> bool {
        let now = Instant::now();
        now >= self.end
    }

    fn poll_every(&self) -> usize {
        storage_layout_extractor::constant::DEFAULT_WATCHDOG_POLL_LOOP_ITERATIONS
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::parse();
    type Meta = u64; // duration in us
    let mut ret_other: HashMap<String, (Meta, HashMap<String, String>)> = HashMap::new();
    for entry in fs::read_dir(cfg.input_dir)? {
        let entry = entry?;
        let path = entry.path();
        let fname = entry.file_name().to_str().unwrap().to_string();
        let code = {
            let file_content = fs::read_to_string(path)?;
            let v: Input = serde_json::from_str(&file_content)?;
            hex::decode(v.code.strip_prefix("0x").expect("0x prefix expected"))?
        };
        eprintln!("processing {}", fname);

        let contract = Contract::new(
            code,
            Chain::Ethereum {
                version: EthereumVersion::latest(),
            },
        );

        let vm_config = vm::Config::default();
        let unifier_config = tc::Config::default();
        // let watchdog = LazyWatchdog.in_rc();
        let watchdog = std::rc::Rc::new(MyWatchDog::new(Duration::from_secs(3)));
        let extractor = storage_layout_extractor::new(contract, vm_config, unifier_config, watchdog);
        let now = Instant::now();
        let r = extractor.analyze();
        let dur = now.elapsed().as_micros() as u64;

        ret_other.insert(
            fname,
            (
                dur,
                match r {
                    Ok(layout) => {
                        layout.slots().iter().map(|s|
                            (
                                format!(
                                    "{}_{}",
                                    hex::encode(s.index.0.to_be_bytes()),
                                    s.offset / 8,
                                ),
                                type_str(&s.typ),
                            )).collect()
                    },
                    Err(_err) => {
                        HashMap::new()
                        // "err".to_string()
                    },
                }
            )
        );
    }

    let file = fs::File::create(cfg.output_file)?;
    let mut bw = BufWriter::new(file);
    let _ = serde_json::to_writer(&mut bw, &ret_other);
    bw.flush()?;

    Ok(())
}
