import type { Action, DecisionFeedback, Position, TrainingSpot } from "./engine";
import charts from "./data/preflop_charts.json";

type PolicyFrequencies = Record<Action, number>;

interface OpenChartEntry {
  position: string;
  max_stack_bb?: number;
  tokens: string[];
}

interface VersusChartEntry {
  hero_position: string;
  villain_position: string;
  max_stack_bb?: number;
  tokens: string[];
}

interface ChartBook {
  opens: OpenChartEntry[];
  cold_calls: VersusChartEntry[];
  continue_vs_3bet: VersusChartEntry[];
  three_bets: VersusChartEntry[];
  continue_vs_4bet: VersusChartEntry[];
}

const CHARTS = charts as ChartBook;

type ComboShape = "Pair" | "Suited" | "Offsuit";

interface HandClass {
  high: string;
  low: string;
  shape: ComboShape;
}

interface WeightedHandClass {
  handClass: HandClass;
  weight: number;
}

const RANK_ORDER = ["A", "K", "Q", "J", "T", "9", "8", "7", "6", "5", "4", "3", "2"];

function positionLabel(position: Position): string {
  switch (position) {
    case "Utg":
      return "UTG";
    case "Hj":
      return "HJ";
    case "Co":
      return "CO";
    case "Btn":
      return "BTN";
    case "Sb":
      return "SB";
    case "Bb":
      return "BB";
  }
}

function rankValue(rank: string): number {
  const index = RANK_ORDER.indexOf(rank);
  return 14 - index;
}

function rankFromValue(value: number): string {
  return RANK_ORDER[14 - value];
}

function parseWeightedToken(token: string): [string, number] {
  const [body, weight] = token.split("@");
  return [body, weight ? Number(weight) || 1 : 1];
}

function parseExactHandClass(token: string): HandClass {
  if (token.length === 2) {
    return {
      high: token[0],
      low: token[1],
      shape: "Pair",
    };
  }
  let high = token[0];
  let low = token[1];
  if (rankValue(low) > rankValue(high)) {
    const temp = high;
    high = low;
    low = temp;
  }
  const shape = token[2] === "s" ? "Suited" : "Offsuit";
  return { high, low, shape };
}

function parseDashToken(token: string, weight: number): WeightedHandClass[] {
  const [start, end] = token.split("-");
  const first = parseExactHandClass(start);
  const last = parseExactHandClass(end);
  if (first.shape === "Pair" && last.shape === "Pair") {
    const result: WeightedHandClass[] = [];
    for (let value = rankValue(first.high); value >= rankValue(last.high); value -= 1) {
      const rank = rankFromValue(value);
      result.push({
        handClass: { high: rank, low: rank, shape: "Pair" },
        weight,
      });
    }
    return result;
  }
  if (first.high === last.high && first.shape === last.shape) {
    const highBound = Math.max(rankValue(first.low), rankValue(last.low));
    const lowBound = Math.min(rankValue(first.low), rankValue(last.low));
    const result: WeightedHandClass[] = [];
    for (let value = highBound; value >= lowBound; value -= 1) {
      result.push({
        handClass: {
          high: first.high,
          low: rankFromValue(value),
          shape: first.shape,
        },
        weight,
      });
    }
    return result;
  }
  return [];
}

function parsePlusToken(token: string, weight: number): WeightedHandClass[] {
  const base = parseExactHandClass(token.slice(0, -1));
  const result: WeightedHandClass[] = [];
  if (base.shape === "Pair") {
    for (let value = rankValue(base.high); value <= 14; value += 1) {
      const rank = rankFromValue(value);
      result.push({
        handClass: { high: rank, low: rank, shape: "Pair" },
        weight,
      });
    }
    return result.reverse();
  }
  for (let value = rankValue(base.low); value < rankValue(base.high); value += 1) {
    result.push({
      handClass: {
        high: base.high,
        low: rankFromValue(value),
        shape: base.shape,
      },
      weight,
    });
  }
  return result.reverse();
}

function parseRange(tokens: string[]): WeightedHandClass[] {
  const classes: WeightedHandClass[] = [];
  for (const token of tokens) {
    const [body, weight] = parseWeightedToken(token);
    if (body.includes("-")) {
      classes.push(...parseDashToken(body, weight));
    } else if (body.endsWith("+")) {
      classes.push(...parsePlusToken(body, weight));
    } else {
      classes.push({ handClass: parseExactHandClass(body), weight });
    }
  }
  return classes;
}

function normalizedHandClass(spot: TrainingSpot): HandClass {
  const first = spot.hole_cards.first.rank[0] === "T" ? "T" : spot.hole_cards.first.rank[0];
  const second = spot.hole_cards.second.rank[0] === "T" ? "T" : spot.hole_cards.second.rank[0];
  let high = first;
  let low = second;
  if (rankValue(low) > rankValue(high)) {
    const temp = high;
    high = low;
    low = temp;
  }
  const pair = high === low;
  const suited = spot.hole_cards.first.suit === spot.hole_cards.second.suit;
  return {
    high,
    low,
    shape: pair ? "Pair" : suited ? "Suited" : "Offsuit",
  };
}

function handWeightFromTokens(spot: TrainingSpot, tokens: string[]): number {
  const target = normalizedHandClass(spot);
  const found = parseRange(tokens).find((entry) => {
    return (
      entry.handClass.high === target.high
      && entry.handClass.low === target.low
      && entry.handClass.shape === target.shape
    );
  });
  return found?.weight ?? 0;
}

function selectOpenEntry(position: Position, stackBb: number): OpenChartEntry | null {
  const candidates = CHARTS.opens
    .filter((entry) => entry.position === positionLabel(position))
    .filter((entry) => entry.max_stack_bb === undefined || stackBb <= entry.max_stack_bb)
    .sort((a, b) => (a.max_stack_bb ?? Number.POSITIVE_INFINITY) - (b.max_stack_bb ?? Number.POSITIVE_INFINITY));
  return candidates[0] ?? null;
}

function selectVersusEntry(
  entries: VersusChartEntry[],
  heroPosition: Position,
  villainPosition: Position,
  stackBb: number,
): VersusChartEntry | null {
  const candidates = entries
    .filter((entry) => entry.hero_position === positionLabel(heroPosition) && entry.villain_position === positionLabel(villainPosition))
    .filter((entry) => entry.max_stack_bb === undefined || stackBb <= entry.max_stack_bb)
    .sort((a, b) => (a.max_stack_bb ?? Number.POSITIVE_INFINITY) - (b.max_stack_bb ?? Number.POSITIVE_INFINITY));
  return candidates[0] ?? null;
}

function clampAndNormalize(freq: PolicyFrequencies): PolicyFrequencies {
  const raise = Math.max(0, freq.Raise);
  const call = Math.max(0, freq.Call);
  const fold = Math.max(0, freq.Fold);
  const total = raise + call + fold;
  if (total <= Number.EPSILON) {
    return { Raise: 0, Call: 0, Fold: 1 };
  }
  return {
    Raise: raise / total,
    Call: call / total,
    Fold: fold / total,
  };
}

export function policyFrequenciesForSpot(spot: TrainingSpot): PolicyFrequencies | null {
  if (spot.street !== "Preflop") return null;

  if (spot.scenario_kind === "OpenRaiseFirstIn") {
    const open = selectOpenEntry(spot.hero_position, spot.stack_bb);
    if (!open) return null;
    const raiseWeight = handWeightFromTokens(spot, open.tokens);
    return clampAndNormalize({
      Raise: raiseWeight,
      Call: 0,
      Fold: 1 - raiseWeight,
    });
  }

  if (spot.scenario_kind === "FacingOpen") {
    const callEntry = selectVersusEntry(
      CHARTS.cold_calls,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    const raiseEntry = selectVersusEntry(
      CHARTS.three_bets,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    if (!callEntry && !raiseEntry) return null;
    const callWeight = callEntry ? handWeightFromTokens(spot, callEntry.tokens) : 0;
    const raiseWeight = raiseEntry ? handWeightFromTokens(spot, raiseEntry.tokens) : 0;
    return clampAndNormalize({
      Raise: raiseWeight,
      Call: callWeight,
      Fold: 1 - raiseWeight - callWeight,
    });
  }

  if (spot.scenario_kind === "FacingThreeBet") {
    const continueEntry = selectVersusEntry(
      CHARTS.continue_vs_3bet,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    if (!continueEntry) return null;
    const continueWeight = handWeightFromTokens(spot, continueEntry.tokens);
    const raiseSeedEntry = selectVersusEntry(
      CHARTS.continue_vs_4bet,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    const raiseSeedWeight = raiseSeedEntry ? handWeightFromTokens(spot, raiseSeedEntry.tokens) : 0;
    const raiseWeight = continueWeight > 0
      ? Math.min(continueWeight, raiseSeedWeight > 0 ? Math.max(raiseSeedWeight, continueWeight * 0.35) : 0)
      : 0;
    const callWeight = Math.max(0, continueWeight - raiseWeight);
    return clampAndNormalize({
      Raise: raiseWeight,
      Call: callWeight,
      Fold: 1 - continueWeight,
    });
  }

  if (spot.scenario_kind === "FacingSqueeze") {
    const continueEntry = selectVersusEntry(
      CHARTS.continue_vs_3bet,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    if (!continueEntry) return null;
    const continueWeight = handWeightFromTokens(spot, continueEntry.tokens);
    const raiseSeedEntry = selectVersusEntry(
      CHARTS.continue_vs_4bet,
      spot.hero_position,
      spot.villain_position,
      spot.stack_bb,
    );
    const raiseSeedWeight = raiseSeedEntry ? handWeightFromTokens(spot, raiseSeedEntry.tokens) : 0;
    const raiseWeight = continueWeight > 0
      ? Math.min(continueWeight, raiseSeedWeight > 0 ? Math.max(raiseSeedWeight, continueWeight * 0.35) : 0)
      : 0;
    const callWeight = Math.max(0, continueWeight - raiseWeight);
    return clampAndNormalize({
      Raise: raiseWeight,
      Call: callWeight,
      Fold: 1 - continueWeight,
    });
  }

  return null;
}

function freqPct(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function actionContext(spot: TrainingSpot): string {
  const history = spot.action_history
    .map((action) => {
      const actor = positionLabel(action.actor);
      switch (action.kind) {
        case "FoldedToHero":
          return "action folded to hero";
        case "OpenRaise":
          return `${actor} opened to ${action.size_bb?.toFixed(1)}bb`;
        case "FlatCall":
          return `${actor} called ${action.size_bb?.toFixed(1)}bb`;
        case "ThreeBet":
          return `${actor} 3-bet to ${action.size_bb?.toFixed(1)}bb`;
        case "Squeeze":
          return `${actor} squeezed to ${action.size_bb?.toFixed(1)}bb`;
      }
    })
    .join(" -> ");
  return `${scenarioLabel(spot.scenario_kind)} context (${history})`;
}

function scenarioLabel(kind: TrainingSpot["scenario_kind"]): string {
  switch (kind) {
    case "OpenRaiseFirstIn":
      return "RFI";
    case "FacingOpen":
      return "Facing Open";
    case "FacingThreeBet":
      return "Facing 3-Bet";
    case "FacingSqueeze":
      return "Facing Squeeze";
  }
}

function explainPolicyDecision(
  spot: TrainingSpot,
  selectedAction: Action,
  bestAction: Action,
  frequencies: PolicyFrequencies,
): string {
  const context = actionContext(spot);
  const dist = `Policy mix: Raise ${freqPct(frequencies.Raise)}, Call ${freqPct(frequencies.Call)}, Fold ${freqPct(frequencies.Fold)}.`;
  const chosen = `${selectedAction} frequency: ${freqPct(frequencies[selectedAction])}.`;
  const best = `Highest-frequency action: ${bestAction} at ${freqPct(frequencies[bestAction])}.`;

  if (selectedAction === bestAction) {
    return [
      `${context}.`,
      dist,
      `${selectedAction} is correct here because it appears most often in the chart-derived strategy for this exact positional node and stack depth.`,
      chosen,
    ].join(" ");
  }

  return [
    `${context}.`,
    dist,
    `Your choice was ${selectedAction}, but this node leans toward ${bestAction} in the chart-derived strategy.`,
    `${chosen} ${best}`,
    "In practice: follow the highest-frequency action by default, then mix the lower-frequency line only when deliberately randomizing.",
  ].join(" ");
}

export function chartPolicyFeedback(
  spot: TrainingSpot,
  selectedAction: Action,
): DecisionFeedback | null {
  const frequencies = policyFrequenciesForSpot(spot);
  if (!frequencies) return null;

  const allActions: Action[] = ["Raise", "Call", "Fold"];
  const bestAction = allActions.reduce((best, action) =>
    frequencies[action] > frequencies[best] ? action : best, "Fold");

  const selectedEval = spot.evaluations.find((entry) => entry.action === selectedAction);
  const bestEval = spot.evaluations.find((entry) => entry.action === bestAction);
  if (!selectedEval || !bestEval) return null;

  const explanation = explainPolicyDecision(spot, selectedAction, bestAction, frequencies);

  return {
    selected_action: selectedAction,
    selected_ev_bb: selectedEval.ev_bb,
    selected_equity_pct: selectedEval.equity_pct,
    selected_fold_equity_pct: selectedEval.fold_equity_pct,
    correct_action: bestAction,
    correct_ev_bb: bestEval.ev_bb,
    correct_equity_pct: bestEval.equity_pct,
    correct_fold_equity_pct: bestEval.fold_equity_pct,
    pot_odds_pct: spot.pot_odds_pct,
    is_correct: selectedAction === bestAction,
    explanation,
  };
}
