//! Preflop GTO trainer engine.
//!
//! This crate is a WASM-friendly fork of the engine half of `pokerTrainer` (the
//! original Rust desktop app at `~/projects/pokerTrainer`). The `model` and
//! `charts` modules are copied verbatim from there with two changes: serde
//! derives added to the public types, and `getrandom`'s `js` feature enabled
//! so `rand::thread_rng()` works in browser WASM.
//!
//! The `wasm` module exposes a small JS boundary; everything else is the
//! standard Rust API.

pub mod charts;
pub mod equity;
pub mod model;
pub mod solver;
pub mod wasm;

pub use model::{
    generate_training_spot, Action, ActionEvaluation, Card, DecisionFeedback, FacingAction,
    HoleCards, MixedAction, Position, PreflopAction, PreflopActionKind, Rank, ScenarioKind, Street,
    Suit, TrainingConfig, TrainingMode, TrainingSpot,
};
