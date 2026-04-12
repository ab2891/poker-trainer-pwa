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

use crate::equity::{EquityTable, HandClass, NUM_CLASSES};
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
enum NodeKind {
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

    for _iter in 0..iterations {
        for traverser in [Player::Opener, Player::Responder] {
            let hero_class = rng.gen_range(0..NUM_CLASSES) as u8;
            let villain_class = rng.gen_range(0..NUM_CLASSES) as u8;
            if hero_class == villain_class {
                continue;
            }

            cfr_traverse(
                &config,
                equity_table,
                &mut entries,
                NodeKind::OpenerOpen,
                hero_class,
                villain_class,
                traverser,
                1.0,
                1.0,
                0.0,
                0.0,
                &mut rng,
            );
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

        if let Some(tv) = terminal_value {
            let val = match traverser {
                Player::Opener => tv,
                Player::Responder => -tv,
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

            if let Some(tv) = terminal_value {
                action_values[action] = match traverser {
                    Player::Opener => tv,
                    Player::Responder => -tv,
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
) -> (Option<NodeKind>, f32, f32, f32, Option<f32>) {
    match node {
        NodeKind::OpenerOpen => match action {
            0 => {
                let payoff = -opener_invested;
                (None, opener_invested, responder_invested, pot, Some(payoff))
            }
            _ => {
                let new_pot = pot + config.open_size - opener_invested;
                (Some(NodeKind::ResponderFacing), config.open_size, responder_invested, new_pot + responder_invested, None)
            }
        },
        NodeKind::ResponderFacing => match action {
            0 => {
                let payoff = pot + responder_invested;
                (None, opener_invested, responder_invested, pot, Some(payoff - opener_invested))
            }
            1 => {
                let call_amount = config.open_size;
                let total_pot = pot + call_amount;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let realization = if config.opener_ip { 0.90 } else { 0.95 };
                let payoff = eq * realization * total_pot - opener_invested;
                (None, opener_invested, call_amount, total_pot, Some(payoff))
            }
            _ => {
                let new_resp_inv = config.three_bet_size;
                let new_pot = config.open_size + config.three_bet_size + BLIND_TOTAL;
                (Some(NodeKind::OpenerFacing3Bet), opener_invested, new_resp_inv, new_pot, None)
            }
        },
        NodeKind::OpenerFacing3Bet => match action {
            0 => {
                let payoff = -opener_invested;
                (None, opener_invested, responder_invested, pot, Some(payoff))
            }
            1 => {
                let call_amount = config.three_bet_size;
                let total_pot = call_amount + responder_invested + BLIND_TOTAL;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let realization = if config.opener_ip { 0.95 } else { 0.88 };
                let payoff = eq * realization * total_pot - call_amount;
                (None, call_amount, responder_invested, total_pot, Some(payoff))
            }
            _ => {
                let new_opener_inv = config.four_bet_size;
                let new_pot = config.four_bet_size + config.three_bet_size + BLIND_TOTAL;
                (Some(NodeKind::ResponderFacing4Bet), new_opener_inv, responder_invested, new_pot, None)
            }
        },
        NodeKind::ResponderFacing4Bet => match action {
            0 => {
                let payoff = pot - opener_invested + responder_invested;
                (None, opener_invested, responder_invested, pot, Some(payoff - opener_invested))
            }
            1 => {
                let call_amount = config.four_bet_size;
                let total_pot = opener_invested + call_amount + BLIND_TOTAL;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let payoff = eq * total_pot - opener_invested;
                (None, opener_invested, call_amount, total_pot, Some(payoff))
            }
            _ => {
                let allin = config.stack_bb;
                let new_pot = config.four_bet_size + allin + BLIND_TOTAL;
                (Some(NodeKind::OpenerFacing5Bet), opener_invested, allin, new_pot, None)
            }
        },
        NodeKind::OpenerFacing5Bet => match action {
            0 => {
                let payoff = -opener_invested;
                (None, opener_invested, responder_invested, pot, Some(payoff))
            }
            _ => {
                let total_pot = config.stack_bb * 2.0 + BLIND_TOTAL;
                let eq = equity_table.get(opener_class as usize, responder_class as usize);
                let payoff = eq * total_pot - config.stack_bb;
                (None, config.stack_bb, config.stack_bb, total_pot, Some(payoff))
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
