/**
 * engine.ts — typed wrapper around the WASM engine.
 *
 * The wasm-pack output exposes plain functions that take/return JS values.
 * This file is the only place that imports from `./wasm/`, and it normalizes
 * the API surface into something React components can call without thinking
 * about the WASM boundary.
 *
 * The actual game logic lives in Rust at `../engine/src/`. To rebuild after
 * editing the Rust source: `npm run engine:build` from the `web/` directory.
 */

import init, {
  default_config as wasmDefaultConfig,
  generate_spot as wasmGenerateSpot,
  evaluate_action as wasmEvaluateAction,
  training_modes as wasmTrainingModes,
} from "./wasm/poker_trainer_engine";

// ----------------------------------------------------------------------------
// Types — these mirror the public Rust types in engine/src/model.rs.
// They are kept loose on purpose: the wire format from serde-wasm-bindgen
// uses the Rust enum/struct names verbatim, so adding new variants in Rust
// doesn't break the TS side.
// ----------------------------------------------------------------------------

export type Position = "Utg" | "Hj" | "Co" | "Btn" | "Sb" | "Bb";
export type Action = "Raise" | "Call" | "Fold";
export type Suit = "Hearts" | "Diamonds" | "Clubs" | "Spades";
export type Rank =
  | "Ace" | "King" | "Queen" | "Jack" | "Ten"
  | "Nine" | "Eight" | "Seven" | "Six"
  | "Five" | "Four" | "Three" | "Two";

export interface Card {
  rank: Rank;
  suit: Suit;
}

export interface HoleCards {
  first: Card;
  second: Card;
}

export type ScenarioKind =
  | "OpenRaiseFirstIn"
  | "FacingOpen"
  | "FacingThreeBet"
  | "FacingSqueeze";

export type TrainingMode =
  | "Mixed"
  | "RaiseFirstIn"
  | "OpenDefense"
  | "ThreeBetDefense"
  | "SqueezeDefense";

export interface TrainingConfig {
  stack_depth_bb: number;
  rake_pct: number;
  training_mode: TrainingMode;
}

export interface ActionEvaluation {
  action: Action;
  ev_bb: number;
  equity_pct: number;
  fold_equity_pct: number;
  explanation: string;
}

export interface PreflopAction {
  actor: Position;
  kind:
    | "FoldedToHero"
    | "OpenRaise"
    | "FlatCall"
    | "ThreeBet"
    | "Squeeze";
  size_bb: number | null;
}

export interface TrainingSpot {
  title: string;
  street: "Preflop" | "Flop" | "Turn" | "River";
  scenario_kind: ScenarioKind;
  hero_position: Position;
  villain_position: Position;
  opener_position: Position | null;
  hole_cards: HoleCards;
  board: Card[];
  pot_bb: number;
  stack_bb: number;
  rake_pct: number;
  hero_invested_bb: number;
  call_cost_bb: number;
  raise_to_bb: number;
  pot_odds_pct: number;
  villain_range_pct: number;
  prompt: string;
  facing: { size_bb: number };
  action_history: PreflopAction[];
  evaluations: ActionEvaluation[];
}

export interface DecisionFeedback {
  selected_action: Action;
  selected_ev_bb: number;
  selected_equity_pct: number;
  selected_fold_equity_pct: number;
  correct_action: Action;
  correct_ev_bb: number;
  correct_equity_pct: number;
  correct_fold_equity_pct: number;
  pot_odds_pct: number;
  is_correct: boolean;
  explanation: string;
}

export interface TrainingModeOption {
  value: TrainingMode;
  label: string;
}

// ----------------------------------------------------------------------------
// Init — call once before any other engine function.
// ----------------------------------------------------------------------------

let initialized = false;
let initPromise: Promise<void> | null = null;

export async function ensureEngineReady(): Promise<void> {
  if (initialized) return;
  if (!initPromise) {
    initPromise = init().then(() => {
      initialized = true;
    });
  }
  return initPromise;
}

// ----------------------------------------------------------------------------
// Public API — small, typed, hides the WASM boundary entirely.
// ----------------------------------------------------------------------------

export async function defaultConfig(): Promise<TrainingConfig> {
  await ensureEngineReady();
  return wasmDefaultConfig() as TrainingConfig;
}

export async function generateSpot(config: TrainingConfig): Promise<TrainingSpot> {
  await ensureEngineReady();
  return wasmGenerateSpot(config) as TrainingSpot;
}

export async function evaluateAction(
  spot: TrainingSpot,
  action: Action,
): Promise<DecisionFeedback> {
  await ensureEngineReady();
  return wasmEvaluateAction(spot, action.toLowerCase()) as DecisionFeedback;
}

export async function trainingModes(): Promise<TrainingModeOption[]> {
  await ensureEngineReady();
  return wasmTrainingModes() as TrainingModeOption[];
}

// ----------------------------------------------------------------------------
// Convenience — display helpers used by the placeholder UI.
// You'll probably rewrite or replace these as you build the real components.
// ----------------------------------------------------------------------------

export function rankShort(rank: Rank): string {
  switch (rank) {
    case "Ace": return "A";
    case "King": return "K";
    case "Queen": return "Q";
    case "Jack": return "J";
    case "Ten": return "T";
    case "Nine": return "9";
    case "Eight": return "8";
    case "Seven": return "7";
    case "Six": return "6";
    case "Five": return "5";
    case "Four": return "4";
    case "Three": return "3";
    case "Two": return "2";
  }
}

export function suitGlyph(suit: Suit): string {
  switch (suit) {
    case "Hearts": return "♥";
    case "Diamonds": return "♦";
    case "Clubs": return "♣";
    case "Spades": return "♠";
  }
}

export function suitIsRed(suit: Suit): boolean {
  return suit === "Hearts" || suit === "Diamonds";
}

export function cardLabel(card: Card): string {
  return `${rankShort(card.rank)}${suitGlyph(card.suit)}`;
}

export function positionLabel(p: Position): string {
  switch (p) {
    case "Utg": return "UTG";
    case "Hj": return "HJ";
    case "Co": return "CO";
    case "Btn": return "BTN";
    case "Sb": return "SB";
    case "Bb": return "BB";
  }
}
