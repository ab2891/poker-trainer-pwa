//! WASM boundary. Exposes 4 functions to JS:
//!
//! - `generate_spot(config)` → `TrainingSpot` JSON
//! - `evaluate_action(spot, action)` → `DecisionFeedback` JSON
//! - `default_config()` → `TrainingConfig` JSON
//! - `training_modes()` → list of available training modes
//!
//! The wire format is plain serde-wasm-bindgen, which produces idiomatic JS
//! objects on the React side (no JSON.parse needed).

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::model::{
    self, generate_training_spot, Action, DecisionFeedback, TrainingConfig, TrainingMode,
    TrainingSpot,
};

/// Set up a panic hook so Rust panics include a stack-style message in the
/// browser console (as a thrown JsError) instead of just `RuntimeError`.
#[wasm_bindgen(start)]
pub fn init() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("[engine panic] {info}");
        wasm_bindgen::throw_str(&msg);
    }));
}

/// Build a fresh default config the JS side can mutate before passing back.
#[wasm_bindgen]
pub fn default_config() -> Result<JsValue, JsValue> {
    let config = TrainingConfig::default();
    serde_wasm_bindgen::to_value(&config).map_err(into_js_err)
}

/// Generate a new training spot for the given config.
#[wasm_bindgen]
pub fn generate_spot(config: JsValue) -> Result<JsValue, JsValue> {
    let config: TrainingConfig = serde_wasm_bindgen::from_value(config).map_err(into_js_err)?;
    let spot = generate_training_spot(config);
    serde_wasm_bindgen::to_value(&spot).map_err(into_js_err)
}

/// Evaluate the user's chosen action against the given spot. Returns
/// a `DecisionFeedback` describing whether they were correct, the EVs of
/// the choice they made and the GTO-best alternative, and an explanation.
#[wasm_bindgen]
pub fn evaluate_action(spot: JsValue, action: &str) -> Result<JsValue, JsValue> {
    let spot: TrainingSpot = serde_wasm_bindgen::from_value(spot).map_err(into_js_err)?;
    let action = parse_action(action)?;
    let selected = spot.evaluation_for(action).clone();
    let best = spot.best_action().clone();
    let mixed = spot.mixed_strategy();
    let is_mixed = mixed.is_some();
    let mixed_actions = mixed.clone().unwrap_or_default();
    let is_correct = if is_mixed {
        mixed_actions.iter().any(|m| m.action == selected.action)
    } else {
        selected.action == best.action
    };

    let feedback = DecisionFeedback {
        selected_action: selected.action,
        selected_ev_bb: selected.ev_bb,
        selected_equity_pct: selected.equity_pct,
        selected_fold_equity_pct: selected.fold_equity_pct,
        correct_action: best.action,
        correct_ev_bb: best.ev_bb,
        correct_equity_pct: best.equity_pct,
        correct_fold_equity_pct: best.fold_equity_pct,
        pot_odds_pct: spot.pot_odds_pct,
        is_correct,
        is_mixed,
        mixed_actions,
        explanation: model::build_feedback_explanation(&spot, &selected, &best),
    };

    serde_wasm_bindgen::to_value(&feedback).map_err(into_js_err)
}

/// List the training modes available, with display labels for the UI.
#[wasm_bindgen]
pub fn training_modes() -> Result<JsValue, JsValue> {
    #[derive(Serialize, Deserialize)]
    struct ModeOption {
        value: TrainingMode,
        label: &'static str,
    }
    let modes = [
        TrainingMode::Mixed,
        TrainingMode::RaiseFirstIn,
        TrainingMode::OpenDefense,
        TrainingMode::ThreeBetDefense,
        TrainingMode::SqueezeDefense,
    ]
    .map(|m| ModeOption {
        value: m,
        label: m.label(),
    });
    serde_wasm_bindgen::to_value(&modes).map_err(into_js_err)
}

fn parse_action(s: &str) -> Result<Action, JsValue> {
    match s.to_ascii_lowercase().as_str() {
        "raise" => Ok(Action::Raise),
        "call" => Ok(Action::Call),
        "fold" => Ok(Action::Fold),
        other => Err(JsValue::from_str(&format!("unknown action: {other}"))),
    }
}

fn into_js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}
