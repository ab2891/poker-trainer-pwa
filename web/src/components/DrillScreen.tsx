/**
 * DrillScreen.tsx — PLACEHOLDER end-to-end drill flow.
 *
 * This file exists to prove the engine wiring works. Every visual decision in
 * here is deliberately bland — no opinions about layout, spacing, motion,
 * typography, color of cards, etc. REPLACE THIS COMPONENT.
 *
 * What it demonstrates (so you know how to wire your own version):
 *
 *   1. On mount, calls `generateSpot()` with a default config to get a drill.
 *   2. Renders the hole cards, position context, and the prompt.
 *   3. Three action buttons (Raise / Call / Fold).
 *   4. On click, calls `evaluateAction()` with the spot and the chosen action.
 *   5. Renders the feedback (correct/incorrect, EV comparison, explanation).
 *   6. "Next hand" button calls `generateSpot()` again.
 *
 * The two state machines you actually care about:
 *
 *   - `spot`: TrainingSpot | null         (the current drill)
 *   - `feedback`: DecisionFeedback | null (null until they answer)
 *
 * Everything else is presentation. Throw it out and rebuild.
 */

import { useEffect, useState } from "react";
import {
  type Action,
  type DecisionFeedback,
  type TrainingConfig,
  type TrainingSpot,
  cardLabel,
  defaultConfig,
  evaluateAction,
  generateSpot,
  positionLabel,
  suitIsRed,
} from "../engine";

export function DrillScreen() {
  const [config, setConfig] = useState<TrainingConfig | null>(null);
  const [spot, setSpot] = useState<TrainingSpot | null>(null);
  const [feedback, setFeedback] = useState<DecisionFeedback | null>(null);
  const [stats, setStats] = useState({ answered: 0, correct: 0 });
  const [error, setError] = useState<string | null>(null);

  // On first render: load default config + first spot.
  useEffect(() => {
    (async () => {
      try {
        const cfg = await defaultConfig();
        setConfig(cfg);
        const first = await generateSpot(cfg);
        setSpot(first);
      } catch (e) {
        setError((e as Error).message ?? String(e));
      }
    })();
  }, []);

  async function handleAction(action: Action) {
    if (!spot || feedback) return;
    try {
      const result = await evaluateAction(spot, action);
      setFeedback(result);
      setStats((s) => ({
        answered: s.answered + 1,
        correct: s.correct + (result.is_correct ? 1 : 0),
      }));
    } catch (e) {
      setError((e as Error).message ?? String(e));
    }
  }

  async function handleNext() {
    if (!config) return;
    try {
      const next = await generateSpot(config);
      setSpot(next);
      setFeedback(null);
    } catch (e) {
      setError((e as Error).message ?? String(e));
    }
  }

  if (error) {
    return (
      <div className="glass glass-strong p-6 max-w-md text-center">
        <p className="text-fg-muted text-sm font-mono">engine error</p>
        <p className="text-fg mt-2">{error}</p>
      </div>
    );
  }

  if (!spot) {
    return (
      <div className="text-fg-muted text-sm font-serif italic">
        loading engine…
      </div>
    );
  }

  const accuracy =
    stats.answered === 0
      ? 0
      : Math.round((stats.correct / stats.answered) * 100);

  return (
    <div className="w-full max-w-2xl flex flex-col items-center gap-8">
      {/*
        TODO: replace this whole block with your real hero typography. The
        Fluxly screenshot has a serif-italic accent on a key word — you might
        want to apply that to the position name or the prompt verb.
      */}
      <header className="text-center">
        <p className="text-fg-subtle font-mono text-xs uppercase tracking-widest mb-2">
          {spot.scenario_kind} · {spot.stack_bb} BB deep
        </p>
        <h1 className="font-serif text-4xl sm:text-5xl text-fg leading-tight">
          You are <span className="italic">{positionLabel(spot.hero_position)}</span>
        </h1>
        <p className="text-fg-muted mt-3 max-w-md mx-auto text-sm">
          {spot.prompt}
        </p>
      </header>

      {/* Hole cards. TODO: design real card faces. These are deliberately ugly. */}
      <div className="flex gap-4 my-2">
        <CardFace card={spot.hole_cards.first} />
        <CardFace card={spot.hole_cards.second} />
      </div>

      {/* Pot / stack / action context */}
      <div className="glass px-6 py-4 flex gap-6 text-center">
        <Stat label="pot" value={`${spot.pot_bb.toFixed(1)} bb`} />
        <Stat label="to call" value={`${spot.call_cost_bb.toFixed(1)} bb`} />
        {spot.raise_to_bb > 0 && (
          <Stat label="raise to" value={`${spot.raise_to_bb.toFixed(1)} bb`} />
        )}
        {spot.pot_odds_pct > 0 && (
          <Stat label="pot odds" value={`${spot.pot_odds_pct.toFixed(0)}%`} />
        )}
      </div>

      {/* Action history */}
      {spot.action_history.length > 0 && (
        <p className="text-fg-subtle text-xs font-mono text-center max-w-md">
          {spot.action_history
            .map((a) => {
              const actor = positionLabel(a.actor);
              switch (a.kind) {
                case "FoldedToHero":
                  return "folds to you";
                case "OpenRaise":
                  return `${actor} opens to ${a.size_bb?.toFixed(1)}`;
                case "FlatCall":
                  return `${actor} calls ${a.size_bb?.toFixed(1)}`;
                case "ThreeBet":
                  return `${actor} 3-bets to ${a.size_bb?.toFixed(1)}`;
                case "Squeeze":
                  return `${actor} squeezes to ${a.size_bb?.toFixed(1)}`;
              }
            })
            .join("  →  ")}
        </p>
      )}

      {/* Action buttons. TODO: redesign — these are bland on purpose. */}
      {!feedback && (
        <div className="flex gap-3">
          <ActionButton label="Fold" onClick={() => handleAction("Fold")} />
          <ActionButton label="Call" onClick={() => handleAction("Call")} />
          <ActionButton label="Raise" onClick={() => handleAction("Raise")} />
        </div>
      )}

      {/* Feedback. TODO: redesign — show EV comparison nicely. */}
      {feedback && (
        <div className="glass glass-strong p-6 max-w-lg w-full flex flex-col gap-3">
          <div className="flex items-baseline justify-between">
            <h2
              className={
                "font-serif text-2xl " +
                (feedback.is_correct ? "text-fg" : "text-fg-muted italic")
              }
            >
              {feedback.is_correct ? "Correct" : "GTO says " + feedback.correct_action}
            </h2>
            <p className="text-fg-subtle font-mono text-xs">
              EV {feedback.selected_ev_bb.toFixed(2)} → {feedback.correct_ev_bb.toFixed(2)} bb
            </p>
          </div>
          <p className="text-fg-muted text-sm leading-relaxed">
            {feedback.explanation}
          </p>
          <button
            onClick={handleNext}
            className="glass mt-2 self-end px-5 py-2 text-sm text-fg hover:text-accent transition-colors"
          >
            next hand
          </button>
        </div>
      )}

      {/* Session stats. TODO: probably move this to a corner widget. */}
      <p className="text-fg-faint font-mono text-xs mt-4">
        {stats.correct} / {stats.answered} · {accuracy}%
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sub-components — placeholder, replace freely
// ---------------------------------------------------------------------------

function CardFace({ card }: { card: { rank: string; suit: string } }) {
  // TODO: replace with a real card design (SVG faces, gradients, motion, etc).
  // This is the bare minimum to prove the engine is sending valid card data.
  return (
    <div className="glass glass-strong w-20 h-28 sm:w-24 sm:h-32 flex flex-col items-center justify-center text-3xl sm:text-4xl font-serif">
      <span className="text-fg leading-none">{cardLabel(card as never)}</span>
      <span
        className={
          "text-2xl sm:text-3xl mt-1 " +
          (suitIsRed(card.suit as never) ? "text-fg" : "text-fg-muted")
        }
        aria-hidden
      >
        {/* The cardLabel above already includes the suit glyph; this is decoration */}
      </span>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col items-center">
      <span className="text-fg-subtle font-mono text-[10px] uppercase tracking-widest">
        {label}
      </span>
      <span className="text-fg font-serif text-lg leading-tight mt-0.5">
        {value}
      </span>
    </div>
  );
}

function ActionButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="glass glass-strong px-8 py-3 text-fg font-serif text-lg tracking-tight hover:text-accent hover:border-glass-border-strong transition-colors duration-200 min-w-[110px]"
    >
      {label}
    </button>
  );
}
