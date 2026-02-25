use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::{env, fs};

use evmole::control_flow_graph::{Block, BlockType};
use evmole::{ContractInfoArgs, contract_info};

fn block_for_pc(blocks: &BTreeMap<usize, Block>, pc: usize) -> Option<usize> {
    let (start, block) = blocks.range(..=pc).next_back()?;
    if pc <= block.end { Some(*start) } else { None }
}

fn btype_name(b: &BlockType) -> &'static str {
    match b {
        BlockType::Terminate { .. } => "Terminate",
        BlockType::Jump { .. } => "Jump",
        BlockType::Jumpi { .. } => "Jumpi",
        BlockType::DynamicJump { .. } => "DynamicJump",
        BlockType::DynamicJumpi { .. } => "DynamicJumpi",
    }
}

fn main() {
    let mut code_file: Option<String> = None;
    let mut targets: Vec<usize> = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--code-file" => code_file = args.next(),
            "--target-pc" => {
                if let Some(v) = args.next() {
                    let h = v.trim_start_matches("0x");
                    targets.push(usize::from_str_radix(h, 16).unwrap());
                }
            }
            _ => {}
        }
    }

    let path = code_file.expect("--code-file required");
    let text = fs::read_to_string(path).expect("read code file");
    let hex = text.trim().trim_start_matches("0x");
    let code = alloy_primitives::hex::decode(hex).expect("decode hex");

    let info = contract_info(
        ContractInfoArgs::new(&code)
            .with_selectors()
            .with_control_flow_graph(),
    );
    let functions = info.functions.unwrap_or_default();
    let cfg = info.control_flow_graph.expect("cfg");

    println!("functions={}", functions.len());
    println!("blocks={}", cfg.blocks.len());

    let mut succ: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut pred: HashMap<usize, Vec<usize>> = HashMap::new();
    for (start, block) in &cfg.blocks {
        let mut add = |to: usize| {
            if cfg.blocks.contains_key(&to) {
                succ.entry(*start).or_default().push(to);
                pred.entry(to).or_default().push(*start);
            }
        };
        match &block.btype {
            BlockType::Terminate { .. } => {}
            BlockType::Jump { to } => add(*to),
            BlockType::Jumpi { true_to, false_to } => {
                add(*true_to);
                add(*false_to);
            }
            BlockType::DynamicJump { to } => {
                for dj in to {
                    if let Some(dst) = dj.to {
                        add(dst)
                    }
                }
            }
            BlockType::DynamicJumpi { true_to, false_to } => {
                add(*false_to);
                for dj in true_to {
                    if let Some(dst) = dj.to {
                        add(dst)
                    }
                }
            }
        }
    }

    let context_blocks: Vec<(String, usize, usize)> = functions
        .iter()
        .filter_map(|f| {
            block_for_pc(&cfg.blocks, f.bytecode_offset).map(|b| {
                (
                    alloy_primitives::hex::encode(f.selector),
                    f.bytecode_offset,
                    b,
                )
            })
        })
        .collect();

    println!("context_blocks={}", context_blocks.len());

    for tpc in targets {
        let tblock = block_for_pc(&cfg.blocks, tpc);
        println!("\nTARGET pc=0x{tpc:x} block={:?}", tblock);
        let Some(tb) = tblock else {
            continue;
        };

        // reachable contexts
        let mut reached_by: Vec<(String, usize, usize)> = Vec::new();
        for (sel, off, cblock) in &context_blocks {
            let mut q = VecDeque::new();
            let mut vis = HashSet::new();
            q.push_back(*cblock);
            vis.insert(*cblock);
            let mut ok = false;
            while let Some(n) = q.pop_front() {
                if n == tb {
                    ok = true;
                    break;
                }
                if let Some(nexts) = succ.get(&n) {
                    for &nx in nexts {
                        if vis.insert(nx) {
                            q.push_back(nx);
                        }
                    }
                }
            }
            if ok {
                reached_by.push((sel.clone(), *off, *cblock));
            }
        }
        println!("reachable_contexts={}", reached_by.len());

        // predecessor chain (depth 2)
        let mut lvl1 = pred.get(&tb).cloned().unwrap_or_default();
        lvl1.sort_unstable();
        lvl1.dedup();
        println!("pred_l1_count={}", lvl1.len());
        for p1 in lvl1.iter().take(12) {
            let b1 = cfg.blocks.get(p1).unwrap();
            println!(
                "  p1=0x{p1:x} type={} end=0x{:x}",
                btype_name(&b1.btype),
                b1.end
            );
            let mut lvl2 = pred.get(p1).cloned().unwrap_or_default();
            lvl2.sort_unstable();
            lvl2.dedup();
            for p2 in lvl2.iter().take(5) {
                let b2 = cfg.blocks.get(p2).unwrap();
                println!(
                    "    p2=0x{p2:x} type={} end=0x{:x}",
                    btype_name(&b2.btype),
                    b2.end
                );
            }
            if lvl2.len() > 5 {
                println!("    ... {} more p2", lvl2.len() - 5);
            }
        }
        if lvl1.len() > 12 {
            println!("  ... {} more p1", lvl1.len() - 12);
        }

        // shortest path from first context
        if let Some((sel, off, src)) = reached_by.first() {
            let mut q = VecDeque::new();
            let mut vis = HashSet::new();
            let mut prev: HashMap<usize, usize> = HashMap::new();
            q.push_back(*src);
            vis.insert(*src);
            while let Some(n) = q.pop_front() {
                if n == tb {
                    break;
                }
                if let Some(nexts) = succ.get(&n) {
                    for &nx in nexts {
                        if vis.insert(nx) {
                            prev.insert(nx, n);
                            q.push_back(nx);
                        }
                    }
                }
            }
            if vis.contains(&tb) {
                let mut path = vec![tb];
                let mut cur = tb;
                while let Some(p) = prev.get(&cur) {
                    path.push(*p);
                    cur = *p;
                }
                path.reverse();
                println!(
                    "shortest_path_from selector=0x{sel} off=0x{off:x} len={}",
                    path.len()
                );
                for b in path.iter().take(30) {
                    let bb = cfg.blocks.get(b).unwrap();
                    println!(
                        "  block=0x{b:x} type={} end=0x{:x}",
                        btype_name(&bb.btype),
                        bb.end
                    );
                }
                if path.len() > 30 {
                    println!("  ... {} more blocks", path.len() - 30);
                }
            }
        }
    }
}
