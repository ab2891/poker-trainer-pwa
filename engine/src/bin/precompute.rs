//! Precompute equity table + solver strategies for all position matchups.
//!
//! Run: cargo run --bin precompute --release
//! Outputs: engine/data/solved_strategies.json
//!
//! This data is embedded in the WASM binary via include_str!() so the
//! app starts instantly with no runtime computation.

use poker_trainer_engine::equity::{EquityTable, HandClass, NUM_CLASSES};
use poker_trainer_engine::solver::{self, SolverResult};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Pos {
    name: &'static str,
    order: u8,
}

const POSITIONS: [Pos; 6] = [
    Pos { name: "Utg", order: 2 },
    Pos { name: "Hj", order: 3 },
    Pos { name: "Co", order: 4 },
    Pos { name: "Btn", order: 5 },
    Pos { name: "Sb", order: 0 },
    Pos { name: "Bb", order: 1 },
];

fn main() {
    eprintln!("Computing equity table (500 samples per pair)...");
    let table = EquityTable::compute(500);
    eprintln!("Equity table done.");

    let stack_depths: &[f32] = &[20.0, 40.0, 100.0];
    let mut all_results = Vec::new();

    for &stack_bb in stack_depths {
        let stack_bucket = if stack_bb <= 30.0 { 0u8 } else if stack_bb <= 60.0 { 1 } else { 2 };

        for opener in &POSITIONS {
            for responder in &POSITIONS {
                if opener.name == responder.name {
                    continue;
                }
                if opener.name == "Bb" {
                    continue;
                }

                let opener_ip = opener.order > responder.order;
                eprintln!(
                    "Solving {}-vs-{} at {}bb (ip={})...",
                    opener.name, responder.name, stack_bb, opener_ip
                );
                let result = solver::solve_matchup(&table, stack_bb, opener_ip, 2000);

                let mut hands = serde_json::Map::new();
                for class_idx in 0..NUM_CLASSES {
                    let hc = HandClass::from_index(class_idx);
                    let label = hc.label();

                    let open = result.opener_open_strategy(class_idx);
                    let vs_open = result.responder_vs_open(class_idx);
                    let vs_3bet = result.opener_vs_3bet(class_idx);
                    let vs_4bet = result.responder_vs_4bet(class_idx);
                    let vs_5bet = result.opener_vs_5bet(class_idx);

                    hands.insert(
                        label,
                        json!({
                            "open": round_arr(&open),
                            "vs_open": round_arr3(&vs_open),
                            "vs_3bet": round_arr3(&vs_3bet),
                            "vs_4bet": round_arr3(&vs_4bet),
                            "vs_5bet": round_arr(&vs_5bet),
                        }),
                    );
                }

                all_results.push(json!({
                    "opener": opener.name,
                    "responder": responder.name,
                    "stack_bb": stack_bb,
                    "stack_bucket": stack_bucket,
                    "opener_ip": opener_ip,
                    "hands": hands,
                }));
            }
        }
    }

    let output = json!({ "matchups": all_results });
    let path = "data/solved_strategies.json";
    std::fs::write(path, serde_json::to_string(&output).unwrap()).unwrap();
    eprintln!("Wrote {path} ({} matchups)", all_results.len());
}

fn round_arr(a: &[f32; 2]) -> [f32; 2] {
    [round3(a[0]), round3(a[1])]
}

fn round_arr3(a: &[f32; 3]) -> [f32; 3] {
    [round3(a[0]), round3(a[1]), round3(a[2])]
}

fn round3(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}
