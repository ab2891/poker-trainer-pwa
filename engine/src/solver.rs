//! Preflop CFR solver.
//!
//! Solves pairwise (opener vs responder) preflop games using external-sampling
//! counterfactual regret minimization. Produces Nash-equilibrium action
//! frequencies for every canonical hand class at every decision point.
//!
//! Game tree (per matchup, fixed sizing):
//!   Opener: fold | raise(2.5bb)
//!     → Responder: fold | call | 3bet(~8bb)
//!       → Opener: fold | call | 4bet(~18bb)
//!         → Responder: fold | call | 5bet-allin
//!           → Opener: fold | call(allin)
//!
//! Terminal nodes resolve via preflop all-in equity from the equity table.

use crate::equity::{playability_bonus, EquityTable, HandClass, NUM_CLASSES};
use rand::Rng;
use std::collections::HashMap;

const OPEN_SIZE: f32 = 2.5;
const BLIND_TOTAL: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Player {
    Opener,
    Responder,
}

impl Player {
    fn opponent(self) -> Self {
        match self {
            Player::Opener => Player::Responder,
            Player::Responder => Player::Opener,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    OpenerOpen,
    ResponderFacing,
    OpenerFacing3Bet,
    ResponderFacing4Bet,
    OpenerFacing5Bet,
}

impl NodeKind {
    fn actor(self) -> Player {
        match self {
            NodeKind::OpenerOpen => Player::Opener,
            NodeKind::ResponderFacing => Player::Responder,
            NodeKind::OpenerFacing3Bet => Player::Opener,
            NodeKind::ResponderFacing4Bet => Player::Responder,
            NodeKind::OpenerFacing5Bet => Player::Opener,
        }
    }

    fn num_actions(self) -> usize {
        match self {
            NodeKind::OpenerOpen => 2,
            NodeKind::OpenerFacing5Bet => 2,
            _ => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct InfoSetKey {
    node: NodeKind,
    hand_class: u8,
}

struct RegretEntry {
    regret_sum: Vec<f32>,
    strategy_sum: Vec<f32>,
}

impl RegretEntry {
    fn new(n_actions: usize) -> Self {
        RegretEntry {
            regret_sum: vec![0.0; n_actions],
            strategy_sum: vec![0.0; n_actions],
        }
    }

    fn current_strategy(&self) -> Vec<f32> {
        let n = self.regret_sum.len();
        let mut strategy = vec![0.0f32; n];
        let mut positive_sum = 0.0f32;
        for &r in &self.regret_sum {
            positive_sum += r.max(0.0);
        }
        if positive_sum > 0.0 {
            for i in 0..n {
                strategy[i] = self.regret_sum[i].max(0.0) / positive_sum;
            }
        } else {
            let uniform = 1.0 / n as f32;
            for s in &mut strategy {
                *s = uniform;
            }
        }
        strategy
    }

    fn average_strategy(&self) -> Vec<f32> {
        let total: f32 = self.strategy_sum.iter().sum();
        if total > 0.0 {
            self.strategy_sum.iter().map(|&s| s / total).collect()
        } else {
            let n = self.strategy_sum.len();
            vec![1.0 / n as f32; n]
        }
    }
}

struct GameConfig {
    stack_bb: f32,
    open_size: f32,
    three_bet_size: f32,
    four_bet_size: f32,
    rake_pct: f32,
    opener_ip: bool,
}

impl GameConfig {
    fn new(stack_bb: f32, opener_ip: bool) -> Self {
        let three_bet_size = if opener_ip { OPEN_SIZE * 3.0 } else { OPEN_SIZE * 3.5 };
        let four_bet_size = three_bet_size * 2.3;
        GameConfig {
            stack_bb,
            open_size: OPEN_SIZE,
            three_bet_size,
            four_bet_size,
            rake_pct: 0.0,
            opener_ip,
        }
    }
}

#[derive(Clone)]
pub struct SolverResult {
    pub strategies: HashMap<(NodeKind, u8), Vec<f32>>,
}

impl SolverResult {
    pub fn opener_open_strategy(&self, hand_class: usize) -> [f32; 2] {
        let key = (NodeKind::OpenerOpen, hand_class as u8);
        let s = self.strategies.get(&key).cloned().unwrap_or_else(|| vec![0.5, 0.5]);
        [s[0], s[1]]
    }

    pub fn responder_vs_open(&self, hand_class: usize) -> [f32; 3] {
        let key = (NodeKind::ResponderFacing, hand_class as u8);
        let s = self.strategies.get(&key).cloned().unwrap_or_else(|| vec![1.0, 0.0, 0.0]);
        [s[0], s[1], s[2]]
    }

    pub fn opener_vs_3bet(&self, hand_class: usize) -> [f32; 3] {
        let key = (NodeKind::OpenerFacing3Bet, hand_class as u8);
        let s = self.strategies.get(&key).cloned().unwrap_or_else(|| vec![1.0, 0.0, 0.0]);
        [s[0], s[1], s[2]]
    }

    pub fn responder_vs_4bet(&self, hand_class: usize) -> [f32; 3] {
        let key = (NodeKind::ResponderFacing4Bet, hand_class as u8);
        let s = self.strategies.get(&key).cloned().unwrap_or_else(|| vec![1.0, 0.0, 0.0]);
        [s[0], s[1], s[2]]
    }

    pub fn opener_vs_5bet(&self, hand_class: usize) -> [f32; 2] {
        let key = (NodeKind::OpenerFacing5Bet, hand_class as u8);
        let s = self.strategies.get(&key).cloned().unwrap_or_else(|| vec![0.5, 0.5]);
        [s[0], s[1]]
    }
}

pub fn solve_matchup(
    equity_table: &EquityTable,
    stack_bb: f32,
    opener_ip: bool,
    iterations: usize,
) -> SolverResult {
    let config = GameConfig::new(stack_bb, opener_ip);
    let mut entries: HashMap<InfoSetKey, RegretEntry> = HashMap::new();
    let mut rng = rand::thread_rng();

    // Chance-sampling CFR: for each iteration, we fix hero's class and enumerate
    // over all villain classes. This gives dense regret updates — every info set
    // for every hand gets ~NUM_CLASSES visits per iteration, not ~1.
    for _iter in 0..iterations {
        for traverser in [Player::Opener, Player::Responder] {
            let hero_class = rng.gen_range(0..NUM_CLASSES) as u8;
            for villain_class in 0..NUM_CLASSES {
                if hero_class as usize == villain_class {
                    continue;
                }

                // Initial state: opener has not yet committed chips; responder is the BB
                // and has posted 1bb already. The pot carries only the responder's blind
                // initially (SB dead money is omitted for simplicity in this heads-up tree).
                cfr_traverse(
                    &config,
                    equity_table,
                    &mut entries,
                    NodeKind::OpenerOpen,
                    hero_class,
                    villain_class as u8,
                    traverser,
                    0.0, // opener_invested — BTN/UTG/etc have not yet committed
                    1.0, // responder_invested — BB has posted 1bb
                    1.0, // pot — only the BB's blind is live money at this point
                    0.0,
                    &mut rng,
                );
            }
        }
    }

    let mut strategies = HashMap::new();
    for (key, entry) in &entries {
        strategies.insert((key.node, key.hand_class), entry.average_strategy());
    }

    SolverResult { strategies }
}

#[allow(clippy::too_many_arguments)]
fn cfr_traverse<R: Rng>(
    config: &GameConfig,
    equity_table: &EquityTable,
    entries: &mut HashMap<InfoSetKey, RegretEntry>,
    node: NodeKind,
    opener_class: u8,
    responder_class: u8,
    traverser: Player,
    opener_invested: f32,
    responder_invested: f32,
    pot: f32,
    _depth: f32,
    rng: &mut R,
) -> f32 {
    let actor = node.actor();
    let n_actions = node.num_actions();
    let hero_class = match actor {
        Player::Opener => opener_class,
        Player::Responder => responder_class,
    };

    let key = InfoSetKey {
        node,
        hand_class: hero_class,
    };

    let entry = entries
        .entry(key)
        .or_insert_with(|| RegretEntry::new(n_actions));
    let strategy = entry.current_strategy();

    if actor != traverser {
        let action = sample_action(&strategy, rng);
        let (next_node, new_opener_inv, new_responder_inv, new_pot, terminal_value) =
            apply_action(config, equity_table, node, action, opener_class, responder_class,
                         opener_invested, responder_invested, pot);

        if let Some((op, rp)) = terminal_value {
            let val = match traverser {
                Player::Opener => op,
                Player::Responder => rp,
            };
            entry.strategy_sum.iter_mut().zip(&strategy).for_each(|(s, &p)| *s += p);
            return val;
        }

        entry.strategy_sum.iter_mut().zip(&strategy).for_each(|(s, &p)| *s += p);
        cfr_traverse(
            config, equity_table, entries, next_node.unwrap(),
            opener_class, responder_class, traverser,
            new_opener_inv, new_responder_inv, new_pot, _depth + 1.0, rng,
        )
    } else {
        let mut action_values = vec![0.0f32; n_actions];
        let mut node_value = 0.0f32;

        for action in 0..n_actions {
            let (next_node, new_opener_inv, new_responder_inv, new_pot, terminal_value) =
                apply_action(config, equity_table, node, action, opener_class, responder_class,
                             opener_invested, responder_invested, pot);

            if let Some((op, rp)) = terminal_value {
                action_values[action] = match traverser {
                    Player::Opener => op,
                    Player::Responder => rp,
                };
            } else {
                action_values[action] = cfr_traverse(
                    config, equity_table, entries, next_node.unwrap(),
                    opener_class, responder_class, traverser,
                    new_opener_inv, new_responder_inv, new_pot, _depth + 1.0, rng,
                );
            }

            node_value += strategy[action] * action_values[action];
        }

        let entry = entries.get_mut(&key).unwrap();
        for action in 0..n_actions {
            entry.regret_sum[action] += action_values[action] - node_value;
        }
        entry.strategy_sum.iter_mut().zip(&strategy).for_each(|(s, &p)| *s += p);

        node_value
    }
}

// Payoff convention: returns (opener_payoff, responder_payoff) for terminal nodes.
// NON-zero-sum — each player's realized equity is reduced by their own realization
// factor, and the "missing" equity doesn't transfer to the other player (it represents
// rake/variance/skill loss). This is critical: with zero-sum + IP bonus, BB was
// getting an artificial subsidy for calling and over-defending.
//
// Realization factors (100bb cash):
//   IP player (opener_ip=true for opener, or responder when opener OOP): 0.95
//   OOP player: 0.82
// These are tuned to produce reasonable GTO-like equilibria, not derived from first
// principles. Could be better modeled with postflop tree solving but that's out of scope.
#[allow(clippy::type_complexity)]
fn apply_action(
    config: &GameConfig,
    equity_table: &EquityTable,
    node: NodeKind,
    action: usize,
    opener_class: u8,
    responder_class: u8,
    opener_invested: f32,
    responder_invested: f32,
    pot: f32,
) -> (Option<NodeKind>, f32, f32, f32, Option<(f32, f32)>) {
    // Realization factors for the "call" / "showdown" terminals where postflop plays.
    // Base factors capture IP vs OOP advantage (0.95 IP, 0.82 OOP at 100bb).
    // Playability bonuses adjust per hand — suited connectors realize more, offsuit
    // junk realizes less — based on the hand's structural features.
    let (opener_base, responder_base) = if config.opener_ip {
        (0.95f32, 0.82f32)
    } else {
        (0.82f32, 0.95f32)
    };
    let opener_real = (opener_base + playability_bonus(opener_class as usize)).clamp(0.65, 1.10);
    let responder_real =
        (responder_base + playability_bonus(responder_class as usize)).clamp(0.65, 1.10);
    match node {
        NodeKind::OpenerOpen => match action {
            0 => {
                // Opener folds before opening. Opener commits nothing, loses 0.
                // Responder keeps their blind (net 0, since the SB/dead money isn't
                // modeled here — responder's investment refunds itself).
                (None, opener_invested, responder_invested, pot, Some((0.0, 0.0)))
            }
            _ => {
                let new_opener_inv = config.open_size;
                let new_pot = new_opener_inv + responder_invested;
                (Some(NodeKind::ResponderFacing), new_opener_inv, responder_invested, new_pot, None)
            }
        },
        NodeKind::ResponderFacing => match action {
            0 => {
                // Responder folds to the open. Opener wins responder_invested (the blind).
                // Responder loses responder_invested.
                (None, opener_invested, responder_invested, pot, Some((responder_invested, -responder_invested)))
            }
            1 => {
                // Responder calls. Both players reach showdown with postflop realization.
                let new_resp_inv = config.open_size;
                let total_pot = opener_invested + new_resp_inv;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let op = eq * opener_real * total_pot - opener_invested;
                let rp = (1.0 - eq) * responder_real * total_pot - new_resp_inv;
                (None, opener_invested, new_resp_inv, total_pot, Some((op, rp)))
            }
            _ => {
                let new_resp_inv = config.three_bet_size;
                let new_pot = opener_invested + new_resp_inv;
                (Some(NodeKind::OpenerFacing3Bet), opener_invested, new_resp_inv, new_pot, None)
            }
        },
        NodeKind::OpenerFacing3Bet => match action {
            0 => {
                // Opener folds to 3-bet. Opener loses their open amount.
                // Responder wins opener's open contribution.
                (None, opener_invested, responder_invested, pot, Some((-opener_invested, opener_invested)))
            }
            1 => {
                // Opener calls 3-bet. Both to showdown with postflop realization.
                let new_opener_inv = config.three_bet_size;
                let total_pot = new_opener_inv + responder_invested;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let op = eq * opener_real * total_pot - new_opener_inv;
                let rp = (1.0 - eq) * responder_real * total_pot - responder_invested;
                (None, new_opener_inv, responder_invested, total_pot, Some((op, rp)))
            }
            _ => {
                let new_opener_inv = config.four_bet_size;
                let new_pot = new_opener_inv + responder_invested;
                (Some(NodeKind::ResponderFacing4Bet), new_opener_inv, responder_invested, new_pot, None)
            }
        },
        NodeKind::ResponderFacing4Bet => match action {
            0 => {
                // Responder folds to 4-bet. Loses their 3-bet investment.
                (None, opener_invested, responder_invested, pot, Some((responder_invested, -responder_invested)))
            }
            1 => {
                let new_resp_inv = config.four_bet_size;
                let total_pot = opener_invested + new_resp_inv;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let op = eq * opener_real * total_pot - opener_invested;
                let rp = (1.0 - eq) * responder_real * total_pot - new_resp_inv;
                (None, opener_invested, new_resp_inv, total_pot, Some((op, rp)))
            }
            _ => {
                let new_resp_inv = config.stack_bb;
                let new_pot = opener_invested + new_resp_inv;
                (Some(NodeKind::OpenerFacing5Bet), opener_invested, new_resp_inv, new_pot, None)
            }
        },
        NodeKind::OpenerFacing5Bet => match action {
            0 => {
                // Opener folds to 5-bet. Loses their 4-bet investment.
                (None, opener_invested, responder_invested, pot, Some((-opener_invested, opener_invested)))
            }
            _ => {
                // All-in showdown. Raw equity — no postflop realization since there's
                // no postflop decisions after a preflop all-in.
                let new_opener_inv = config.stack_bb;
                let total_pot = new_opener_inv + responder_invested;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let op = eq * total_pot - new_opener_inv;
                let rp = (1.0 - eq) * total_pot - responder_invested;
                (None, new_opener_inv, responder_invested, total_pot, Some((op, rp)))
            }
        },
    }
}

fn sample_action<R: Rng>(strategy: &[f32], rng: &mut R) -> usize {
    let r: f32 = rng.gen();
    let mut cum = 0.0;
    for (i, &p) in strategy.iter().enumerate() {
        cum += p;
        if r < cum {
            return i;
        }
    }
    strategy.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_produces_strategies() {
        let table = EquityTable::compute(5);
        let result = solve_matchup(&table, 100.0, false, 200);

        let aa = HandClass { high: 12, low: 12, suited: false }.index();
        let open = result.opener_open_strategy(aa);
        assert!(open[1] > 0.5, "AA should open >50% of the time, got {:.1}%", open[1] * 100.0);
    }

    #[test]
    fn test_trash_folds() {
        let table = EquityTable::compute(5);
        let result = solve_matchup(&table, 100.0, false, 200);

        let seven_two_o = HandClass { high: 5, low: 0, suited: false }.index();
        let open = result.opener_open_strategy(seven_two_o);
        assert!(open[0] > 0.5, "72o should fold >50% as opener, got {:.1}%", open[0] * 100.0);
    }
}
