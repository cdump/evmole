use std::{collections::BTreeMap, fs, path::PathBuf, time::Instant};

use evmole::{
    EventExecutionProfile, EventSelector, contract_events_with_profile, contract_events_with_stats,
};

#[derive(Debug, Default)]
struct Args {
    code_hex: Option<String>,
    code_file: Option<PathBuf>,
    raw_file: Option<PathBuf>,
    iters: usize,
    warmup: usize,
    profile: bool,
    top_pc: usize,
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
  --profile             Enable detailed execution profile logs
  --top-pc <N>          Top N PCs to print in profile mode (default: 12)
  --show-events         Print extracted event selectors
  -h, --help            Show this help

Exactly one of --code-hex / --code-file / --raw-file must be provided."
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        iters: 1,
        warmup: 0,
        profile: false,
        top_pc: 12,
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
            "--profile" => args.profile = true,
            "--top-pc" => {
                let v = it.next().ok_or("--top-pc requires a value")?;
                args.top_pc = v.parse().map_err(|_| format!("invalid --top-pc: {v}"))?;
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
    if args.top_pc == 0 {
        return Err("--top-pc must be >= 1".to_string());
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

fn print_top_pc(title: &str, map: &BTreeMap<usize, u64>, top_n: usize) {
    if map.is_empty() {
        println!("{title}: (empty)");
        return;
    }
    let mut pairs: Vec<(usize, u64)> = map.iter().map(|(k, v)| (*k, *v)).collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    println!("{title}:");
    for (pc, cnt) in pairs.into_iter().take(top_n) {
        println!("  pc=0x{pc:x} ({pc}) count={cnt}");
    }
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
        if args.profile {
            let _ = contract_events_with_profile(&code);
        } else {
            let _ = contract_events_with_stats(&code);
        }
    }

    let mut last_events = Vec::new();
    let mut last_stats = evmole::EventExtractionStats::default();
    let mut last_profile = EventExecutionProfile::default();
    let mut elapsed_ms = Vec::with_capacity(args.iters);

    for _ in 0..args.iters {
        let t0 = Instant::now();
        let (events, stats, profile) = if args.profile {
            contract_events_with_profile(&code)
        } else {
            let (events, stats) = contract_events_with_stats(&code);
            (events, stats, EventExecutionProfile::default())
        };
        let dt = t0.elapsed().as_secs_f64() * 1000.0;
        elapsed_ms.push(dt);
        last_events = events;
        last_stats = stats;
        last_profile = profile;
    }

    let total_ms: f64 = elapsed_ms.iter().sum();
    let avg_ms = total_ms / elapsed_ms.len() as f64;
    let min_ms = elapsed_ms.iter().copied().fold(f64::INFINITY, f64::min);
    let max_ms = elapsed_ms.iter().copied().fold(0.0, f64::max);

    let jump_total = last_stats
        .jump_classify_cache_hits
        .saturating_add(last_stats.jump_classify_cache_misses);
    let jump_hit_rate = if jump_total == 0 {
        0.0
    } else {
        last_stats.jump_classify_cache_hits as f64 / jump_total as f64
    };

    let entry_total = last_stats
        .entry_state_cache_hits
        .saturating_add(last_stats.entry_state_cache_misses);
    let entry_hit_rate = if entry_total == 0 {
        0.0
    } else {
        last_stats.entry_state_cache_hits as f64 / entry_total as f64
    };

    let probe_total = last_stats
        .probe_cache_hits
        .saturating_add(last_stats.probe_cache_misses);
    let probe_hit_rate = if probe_total == 0 {
        0.0
    } else {
        last_stats.probe_cache_hits as f64 / probe_total as f64
    };

    let can_fork_total = last_stats
        .jump_classify_can_fork_true
        .saturating_add(last_stats.jump_classify_can_fork_false);
    let can_fork_true_rate = if can_fork_total == 0 {
        0.0
    } else {
        last_stats.jump_classify_can_fork_true as f64 / can_fork_total as f64
    };

    println!("code_len: {}", code.len());
    println!(
        "time_ms: avg={avg_ms:.3} min={min_ms:.3} max={max_ms:.3} (iters={})",
        args.iters
    );
    println!("events: {}", last_events.len());
    println!(
        "selectors: total={} after_mutability_prune={} pruned_view_or_pure={}",
        last_stats.selectors_total,
        last_stats.selectors_after_mutability_prune,
        last_stats.selectors_pruned_view_or_pure
    );
    println!(
        "jump_cache: hit={} miss={} rate={:.2}%",
        last_stats.jump_classify_cache_hits,
        last_stats.jump_classify_cache_misses,
        jump_hit_rate * 100.0
    );
    println!(
        "entry_cache: hit={} miss={} rate={:.2}%",
        last_stats.entry_state_cache_hits,
        last_stats.entry_state_cache_misses,
        entry_hit_rate * 100.0
    );
    println!(
        "probe_cache: hit={} miss={} rate={:.2}%",
        last_stats.probe_cache_hits,
        last_stats.probe_cache_misses,
        probe_hit_rate * 100.0
    );
    println!(
        "jump_can_fork_true: {}/{} ({:.2}%)",
        last_stats.jump_classify_can_fork_true,
        can_fork_total,
        can_fork_true_rate * 100.0
    );
    let static_dead_total = last_stats
        .static_dead_other_prunes
        .saturating_add(last_stats.static_dead_current_prunes);
    println!(
        "static_dead_prunes: other={} current={} total={}",
        last_stats.static_dead_other_prunes,
        last_stats.static_dead_current_prunes,
        static_dead_total
    );

    if args.profile {
        println!(
            "states: pushed={} popped={} queue_peak={} state_limit_breaks={}",
            last_profile.states_pushed,
            last_profile.states_popped,
            last_profile.queue_peak,
            last_profile.state_limit_breaks
        );
        println!(
            "jump ops: jump_total={} jump_visited_breaks={} jumpi_total={} jumpi_visited_breaks={} visited_cap_hits={}",
            last_profile.jump_total,
            last_profile.jump_visited_breaks,
            last_profile.jumpi_total,
            last_profile.jumpi_visited_breaks,
            last_profile.visited_cap_hits
        );
        println!(
            "jumpi outcomes: keep={} switch={} fork={} throttled={} deduped={} invalid_other_pc={} unreachable_both={} unreachable_current={} unreachable_other={}",
            last_profile.jumpi_decision_keep,
            last_profile.jumpi_decision_switch,
            last_profile.jumpi_decision_fork,
            last_profile.jumpi_fork_throttled,
            last_profile.jumpi_fork_deduped,
            last_profile.jumpi_invalid_other_pc,
            last_profile.jumpi_unreachable_both,
            last_profile.jumpi_unreachable_current,
            last_profile.jumpi_unreachable_other
        );
        print_top_pc(
            "top context starts",
            &last_profile.context_start_by_pc,
            args.top_pc,
        );
        print_top_pc("top jumpi", &last_profile.jumpi_by_pc, args.top_pc);
        print_top_pc(
            "top jumpi can_fork=true",
            &last_profile.jumpi_can_fork_true_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi can_fork=false",
            &last_profile.jumpi_can_fork_false_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi cache miss",
            &last_profile.jumpi_cache_miss_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi decision fork",
            &last_profile.jumpi_decision_fork_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi decision switch",
            &last_profile.jumpi_decision_switch_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi fork throttled",
            &last_profile.jumpi_fork_throttled_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi fork deduped",
            &last_profile.jumpi_fork_deduped_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi unreachable_both",
            &last_profile.jumpi_unreachable_both_by_pc,
            args.top_pc,
        );
        print_top_pc(
            "top jumpi invalid_other_pc",
            &last_profile.jumpi_invalid_other_pc_by_pc,
            args.top_pc,
        );
    }

    if args.show_events {
        last_events.sort_unstable();
        for evt in &last_events {
            println!("event: {}", fmt_selector(evt));
        }
    }
}
