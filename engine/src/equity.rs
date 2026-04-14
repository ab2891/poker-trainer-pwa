//! Preflop equity computation.
//!
//! Builds a 169×169 table of average equity between canonical hand classes
//! (AA, AKs, AKo, ..., 32o). Each entry is the average all-in equity of
//! class A vs class B across all valid combo pairings (card-removal applied).
//!
//! The table is computed once via Monte Carlo and cached for the solver.

use crate::model::{Card, HoleCards, Rank, Suit};
use rand::seq::SliceRandom;
use rand::thread_rng;

pub const NUM_CLASSES: usize = 169;

/// Playability bonus added to the per-player realization factor in the CFR solver.
///
/// Raw preflop equity + flat IP/OOP realization underestimates hands that flop well
/// (suited connectors, small pairs, suited wheels) and overestimates hands that flop
/// poorly (offsuit junk with gaps). This function captures the structural playability
/// of each hand class as a delta in [-0.10, +0.12] to add to realization.
///
/// Returns a multiplier adjustment: +0.10 means "realize 10% more of your equity
/// share than a flat-playability hand of the same raw equity." Tuned empirically
/// against published GTO charts (Jonathan Little, GTO Wizard free charts).
pub fn playability_bonus(class_idx: usize) -> f32 {
    let hc = HandClass::from_index(class_idx);

    if hc.high == hc.low {
        // Pair: small pairs set-mine profitably (huge implied odds)
        if hc.high <= 4 {
            return 0.10; // 22, 33, 44, 55, 66
        }
        if hc.high <= 7 {
            return 0.05; // 77, 88, 99
        }
        return 0.02; // TT+, medium implied odds
    }

    let gap = hc.high - hc.low;
    let is_broadway = hc.high >= 9; // T or better

    if hc.suited {
        // Suited hands — flush draws, straight draws, often combo draws
        if gap == 1 {
            return 0.12; // Suited connectors: 98s, 87s, 76s, 65s, 54s
        }
        if gap == 2 {
            return 0.08; // Suited one-gappers: 97s, 86s, 75s, 64s
        }
        if gap == 3 {
            return 0.05; // Suited two-gappers: 96s, 85s
        }
        if hc.high == 12 && hc.low <= 4 {
            return 0.09; // Suited wheel aces: A5s-A2s (flush + straight + nut blocker)
        }
        if hc.high == 12 {
            return 0.05; // Suited aces generally
        }
        if is_broadway && hc.low >= 8 {
            return 0.06; // Suited broadway: KQs, KJs, QJs, JTs, T9s
        }
        return 0.03; // Generic suited
    }

    // Offsuit — no flush potential, worse straight potential
    if is_broadway && hc.low >= 9 {
        return 0.0; // AKo, KQo, QJo etc. — fine as-is
    }
    if is_broadway && gap >= 2 && hc.low < 8 {
        return -0.04; // Weak offsuit broadway with gap: KTo, K9o, QTo
    }
    if hc.high == 12 && hc.low < 9 {
        return -0.05; // Weak offsuit aces: A9o, A8o, A7o (dominated kicker)
    }
    if gap >= 4 {
        return -0.10; // Offsuit junk with big gap: Q5o, J6o, K4o, T4o
    }
    -0.03
}

static ALL_SUITS: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
static ALL_RANKS: [Rank; 13] = [
    Rank::Two,
    Rank::Three,
    Rank::Four,
    Rank::Five,
    Rank::Six,
    Rank::Seven,
    Rank::Eight,
    Rank::Nine,
    Rank::Ten,
    Rank::Jack,
    Rank::Queen,
    Rank::King,
    Rank::Ace,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandClass {
    pub high: u8,
    pub low: u8,
    pub suited: bool,
}

impl HandClass {
    pub fn index(&self) -> usize {
        if self.high == self.low {
            (12 - self.high as usize)
        } else if self.suited {
            13 + pair_index(self.high, self.low)
        } else {
            13 + 78 + pair_index(self.high, self.low)
        }
    }

    pub fn from_index(idx: usize) -> Self {
        if idx < 13 {
            let r = (12 - idx) as u8;
            HandClass { high: r, low: r, suited: false }
        } else if idx < 13 + 78 {
            let (h, l) = pair_from_index(idx - 13);
            HandClass { high: h, low: l, suited: true }
        } else {
            let (h, l) = pair_from_index(idx - 13 - 78);
            HandClass { high: h, low: l, suited: false }
        }
    }

    pub fn from_hole_cards(cards: HoleCards) -> Self {
        let r1 = rank_value(cards.first.rank);
        let r2 = rank_value(cards.second.rank);
        let suited = cards.first.suit == cards.second.suit;
        let (high, low) = if r1 >= r2 { (r1, r2) } else { (r2, r1) };
        HandClass { high, low, suited: if high == low { false } else { suited } }
    }

    pub fn label(&self) -> String {
        let h = rank_char(self.high);
        let l = rank_char(self.low);
        if self.high == self.low {
            format!("{h}{l}")
        } else if self.suited {
            format!("{h}{l}s")
        } else {
            format!("{h}{l}o")
        }
    }

    pub fn combos(&self) -> Vec<HoleCards> {
        let mut out = Vec::new();
        if self.high == self.low {
            let rank = index_to_rank(self.high);
            for i in 0..4 {
                for j in (i + 1)..4 {
                    out.push(HoleCards {
                        first: Card { rank, suit: ALL_SUITS[i] },
                        second: Card { rank, suit: ALL_SUITS[j] },
                    });
                }
            }
        } else {
            let rh = index_to_rank(self.high);
            let rl = index_to_rank(self.low);
            if self.suited {
                for &s in &ALL_SUITS {
                    out.push(HoleCards {
                        first: Card { rank: rh, suit: s },
                        second: Card { rank: rl, suit: s },
                    });
                }
            } else {
                for &s1 in &ALL_SUITS {
                    for &s2 in &ALL_SUITS {
                        if s1 != s2 {
                            out.push(HoleCards {
                                first: Card { rank: rh, suit: s1 },
                                second: Card { rank: rl, suit: s2 },
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

fn pair_index(high: u8, low: u8) -> usize {
    let h = high as usize;
    let l = low as usize;
    let offset = 78 - h * (h + 1) / 2;
    offset + (h - 1 - l)
}

fn pair_from_index(idx: usize) -> (u8, u8) {
    let mut high = 12usize;
    let mut offset = 0usize;
    loop {
        let count = high;
        if idx < offset + count {
            let within = idx - offset;
            let low = high - 1 - within;
            return (high as u8, low as u8);
        }
        offset += count;
        if high == 0 {
            break;
        }
        high -= 1;
    }
    (1, 0)
}

pub fn rank_value(r: Rank) -> u8 {
    match r {
        Rank::Two => 0,
        Rank::Three => 1,
        Rank::Four => 2,
        Rank::Five => 3,
        Rank::Six => 4,
        Rank::Seven => 5,
        Rank::Eight => 6,
        Rank::Nine => 7,
        Rank::Ten => 8,
        Rank::Jack => 9,
        Rank::Queen => 10,
        Rank::King => 11,
        Rank::Ace => 12,
    }
}

fn rank_char(v: u8) -> char {
    match v {
        0 => '2', 1 => '3', 2 => '4', 3 => '5', 4 => '6', 5 => '7',
        6 => '8', 7 => '9', 8 => 'T', 9 => 'J', 10 => 'Q', 11 => 'K',
        12 => 'A', _ => '?',
    }
}

fn index_to_rank(v: u8) -> Rank {
    ALL_RANKS[v as usize]
}

fn cards_overlap(a: &HoleCards, b: &HoleCards) -> bool {
    let same = |c1: &Card, c2: &Card| c1.rank == c2.rank && c1.suit == c2.suit;
    same(&a.first, &b.first)
        || same(&a.first, &b.second)
        || same(&a.second, &b.first)
        || same(&a.second, &b.second)
}

pub fn evaluate_showdown(hero: HoleCards, villain: HoleCards, board: &[Card]) -> f32 {
    let hero_strength = best_hand_rank(hero, board);
    let villain_strength = best_hand_rank(villain, board);
    if hero_strength > villain_strength {
        1.0
    } else if hero_strength == villain_strength {
        0.5
    } else {
        0.0
    }
}

fn best_hand_rank(hole: HoleCards, board: &[Card]) -> u32 {
    let mut all = Vec::with_capacity(7);
    all.push(hole.first);
    all.push(hole.second);
    all.extend_from_slice(board);

    let mut best = 0u32;
    let n = all.len();
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                for l in (k + 1)..n {
                    for m in (l + 1)..n {
                        let hand = [all[i], all[j], all[k], all[l], all[m]];
                        let r = rank_5card(&hand);
                        if r > best {
                            best = r;
                        }
                    }
                }
            }
        }
    }
    best
}

fn rank_5card(cards: &[Card; 5]) -> u32 {
    let mut ranks: Vec<u8> = cards.iter().map(|c| rank_value(c.rank)).collect();
    ranks.sort_unstable();
    ranks.reverse();

    let flush = cards.iter().all(|c| c.suit == cards[0].suit);
    let straight = is_straight(&ranks);
    let wheel = ranks == [12, 3, 2, 1, 0];
    let is_str = straight || wheel;

    let mut counts: [u8; 13] = [0; 13];
    for &r in &ranks {
        counts[r as usize] += 1;
    }

    let mut groups: Vec<(u8, u8)> = counts
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(r, &c)| (c, r as u8))
        .collect();
    groups.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    let category = match (flush, is_str, groups[0].0) {
        (true, true, _) => 8,
        (_, _, 4) => 7,
        (_, _, 3) if groups.len() == 2 => 6,
        (true, false, _) => 5,
        (false, true, _) | (false, _, _) if is_str && !flush => 4,
        (_, _, 3) => 3,
        (_, _, 2) if groups.len() == 3 => 2,
        (_, _, 2) => 1,
        _ => 0,
    };

    let kickers: u32 = if wheel && is_str {
        3 << 12 | 2 << 8 | 1 << 4 | 0
    } else {
        groups
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &(_, r))| acc | ((r as u32) << (16 - i * 4)))
    };

    (category << 20) | kickers
}

fn is_straight(sorted_desc: &[u8]) -> bool {
    if sorted_desc.len() < 5 {
        return false;
    }
    for i in 0..4 {
        if sorted_desc[i] != sorted_desc[i + 1] + 1 {
            return false;
        }
    }
    true
}

pub struct EquityTable {
    data: Vec<f32>,
}

impl EquityTable {
    pub fn compute(samples_per_matchup: usize) -> Self {
        let mut data = vec![0.5f32; NUM_CLASSES * NUM_CLASSES];
        let mut rng = thread_rng();

        for i in 0..NUM_CLASSES {
            for j in i..NUM_CLASSES {
                let class_a = HandClass::from_index(i);
                let class_b = HandClass::from_index(j);
                let eq = compute_class_equity(&class_a, &class_b, samples_per_matchup, &mut rng);
                data[i * NUM_CLASSES + j] = eq;
                data[j * NUM_CLASSES + i] = 1.0 - eq;
            }
            data[i * NUM_CLASSES + i] = 0.5;
        }

        EquityTable { data }
    }

    pub fn get(&self, class_a: usize, class_b: usize) -> f32 {
        self.data[class_a * NUM_CLASSES + class_b]
    }
}

fn compute_class_equity<R: rand::Rng>(
    a: &HandClass,
    b: &HandClass,
    samples: usize,
    rng: &mut R,
) -> f32 {
    let combos_a = a.combos();
    let combos_b = b.combos();

    let mut eq_sum = 0.0f64;
    let mut count = 0u32;

    let mut deck: Vec<Card> = Vec::with_capacity(48);

    for _ in 0..samples {
        let ca = combos_a[rng.gen_range(0..combos_a.len())];
        let cb = combos_b[rng.gen_range(0..combos_b.len())];
        if cards_overlap(&ca, &cb) {
            continue;
        }

        deck.clear();
        for &r in &ALL_RANKS {
            for &s in &ALL_SUITS {
                let c = Card { rank: r, suit: s };
                let same = |x: &Card| x.rank == c.rank && x.suit == c.suit;
                if !same(&ca.first)
                    && !same(&ca.second)
                    && !same(&cb.first)
                    && !same(&cb.second)
                {
                    deck.push(c);
                }
            }
        }

        deck.shuffle(rng);
        eq_sum += evaluate_showdown(ca, cb, &deck[..5]) as f64;
        count += 1;
    }

    if count == 0 {
        0.5
    } else {
        (eq_sum / count as f64) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hand_class_roundtrip() {
        for i in 0..NUM_CLASSES {
            let hc = HandClass::from_index(i);
            assert_eq!(hc.index(), i, "roundtrip failed for index {i}: {}", hc.label());
        }
    }

    #[test]
    fn test_aa_vs_kk_equity() {
        let aa = HandClass { high: 12, low: 12, suited: false };
        let kk = HandClass { high: 11, low: 11, suited: false };
        let mut rng = rand::thread_rng();
        let eq = compute_class_equity(&aa, &kk, 200, &mut rng);
        assert!(eq > 0.75 && eq < 0.90, "AA vs KK equity should be ~82%, got {eq}");
    }

    #[test]
    fn test_combo_counts() {
        let aa = HandClass { high: 12, low: 12, suited: false };
        assert_eq!(aa.combos().len(), 6);
        let aks = HandClass { high: 12, low: 11, suited: true };
        assert_eq!(aks.combos().len(), 4);
        let ako = HandClass { high: 12, low: 11, suited: false };
        assert_eq!(ako.combos().len(), 12);
    }
}
