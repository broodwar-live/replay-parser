//! Build order similarity comparison.
//!
//! Provides distance metrics for comparing build orders across replays,
//! enabling "find similar builds" and build order database search.
//!
//! The primary metric is a weighted edit distance on the action sequence,
//! normalized to [0.0, 1.0] where 0.0 = identical and 1.0 = completely different.

use crate::analysis::{BuildAction, BuildOrderEntry};

/// A simplified build order representation for comparison.
///
/// Strips timing and focuses on the sequence of actions for a single player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSequence {
    /// Ordered list of action type IDs.
    actions: Vec<ActionKey>,
}

/// A compact key representing a build action for comparison purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ActionKey {
    Build(u16),
    Train(u16),
    UnitMorph(u16),
    BuildingMorph(u16),
    Research(u8),
    Upgrade(u8),
    TrainFighter,
}

impl From<&BuildAction> for ActionKey {
    fn from(action: &BuildAction) -> Self {
        match action {
            BuildAction::Build(id) => ActionKey::Build(*id),
            BuildAction::Train(id) => ActionKey::Train(*id),
            BuildAction::UnitMorph(id) => ActionKey::UnitMorph(*id),
            BuildAction::BuildingMorph(id) => ActionKey::BuildingMorph(*id),
            BuildAction::Research(id) => ActionKey::Research(*id),
            BuildAction::Upgrade(id) => ActionKey::Upgrade(*id),
            BuildAction::TrainFighter => ActionKey::TrainFighter,
        }
    }
}

impl BuildSequence {
    /// Extract a build sequence for a specific player from build order entries.
    pub fn from_build_order(entries: &[BuildOrderEntry], player_id: u8) -> Self {
        let actions = entries
            .iter()
            .filter(|e| e.player_id == player_id)
            .map(|e| ActionKey::from(&e.action))
            .collect();
        Self { actions }
    }

    /// Extract a build sequence limited to the first N actions (opening).
    ///
    /// Useful for comparing openings regardless of mid/late game divergence.
    pub fn from_build_order_opening(
        entries: &[BuildOrderEntry],
        player_id: u8,
        max_actions: usize,
    ) -> Self {
        let actions = entries
            .iter()
            .filter(|e| e.player_id == player_id)
            .take(max_actions)
            .map(|e| ActionKey::from(&e.action))
            .collect();
        Self { actions }
    }

    /// Number of actions in the sequence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether the sequence is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Compute the similarity between two build sequences.
///
/// Returns a value in [0.0, 1.0] where:
/// - 1.0 = identical sequences
/// - 0.0 = completely different (or one is empty)
///
/// Uses normalized edit distance (Levenshtein) on the action sequences.
pub fn similarity(a: &BuildSequence, b: &BuildSequence) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let dist = edit_distance(&a.actions, &b.actions);
    let max_len = a.len().max(b.len());
    1.0 - (dist as f64 / max_len as f64)
}

/// Compute the longest common subsequence ratio between two build sequences.
///
/// Returns a value in [0.0, 1.0] where:
/// - 1.0 = one sequence is a subsequence of the other
/// - 0.0 = no common actions in order
///
/// LCS is more forgiving than edit distance — it ignores inserted actions
/// and focuses on shared structure.
pub fn lcs_similarity(a: &BuildSequence, b: &BuildSequence) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let lcs_len = lcs_length(&a.actions, &b.actions);
    let max_len = a.len().max(b.len());
    lcs_len as f64 / max_len as f64
}

/// Result of comparing two build orders.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimilarityResult {
    /// Edit-distance-based similarity [0.0, 1.0].
    pub edit_similarity: f64,
    /// LCS-based similarity [0.0, 1.0].
    pub lcs_similarity: f64,
    /// Number of actions in sequence A.
    pub len_a: usize,
    /// Number of actions in sequence B.
    pub len_b: usize,
}

/// Compare two build orders and return detailed similarity metrics.
pub fn compare(a: &BuildSequence, b: &BuildSequence) -> SimilarityResult {
    SimilarityResult {
        edit_similarity: similarity(a, b),
        lcs_similarity: lcs_similarity(a, b),
        len_a: a.len(),
        len_b: b.len(),
    }
}

/// Rank a list of candidate build sequences by similarity to a query.
///
/// Returns indices into `candidates` sorted by descending similarity.
pub fn rank_by_similarity(
    query: &BuildSequence,
    candidates: &[BuildSequence],
) -> Vec<(usize, f64)> {
    let mut scores: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (i, similarity(query, c)))
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

// ---------------------------------------------------------------------------
// Edit distance (Levenshtein)
// ---------------------------------------------------------------------------

fn edit_distance(a: &[ActionKey], b: &[ActionKey]) -> usize {
    let m = a.len();
    let n = b.len();

    // Use two rows instead of full matrix for memory efficiency.
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

// ---------------------------------------------------------------------------
// Longest common subsequence
// ---------------------------------------------------------------------------

fn lcs_length(a: &[ActionKey], b: &[ActionKey]) -> usize {
    let m = a.len();
    let n = b.len();

    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                curr[j] = prev[j - 1] + 1;
            } else {
                curr[j] = prev[j].max(curr[j - 1]);
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }

    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::BuildAction;

    fn entry(frame: u32, player: u8, action: BuildAction) -> BuildOrderEntry {
        BuildOrderEntry {
            frame,
            real_seconds: frame as f64 / 23.81,
            player_id: player,
            action,
        }
    }

    fn seq(actions: &[ActionKey]) -> BuildSequence {
        BuildSequence {
            actions: actions.to_vec(),
        }
    }

    #[test]
    fn test_identical_sequences() {
        let a = seq(&[
            ActionKey::Train(7),
            ActionKey::Train(7),
            ActionKey::Build(111),
        ]);
        let b = seq(&[
            ActionKey::Train(7),
            ActionKey::Train(7),
            ActionKey::Build(111),
        ]);
        assert!((similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_completely_different() {
        let a = seq(&[
            ActionKey::Train(0),
            ActionKey::Train(0),
            ActionKey::Train(0),
        ]);
        let b = seq(&[
            ActionKey::Build(111),
            ActionKey::Build(113),
            ActionKey::Build(114),
        ]);
        assert!((similarity(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_sequences() {
        let empty = seq(&[]);
        let non_empty = seq(&[ActionKey::Train(0)]);

        assert!((similarity(&empty, &empty) - 1.0).abs() < 0.001);
        assert!((similarity(&empty, &non_empty) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_one_difference() {
        // 3 actions, 1 different → edit distance 1, similarity 2/3 ≈ 0.667
        let a = seq(&[
            ActionKey::Train(7),
            ActionKey::Train(7),
            ActionKey::Build(111),
        ]);
        let b = seq(&[
            ActionKey::Train(7),
            ActionKey::Train(7),
            ActionKey::Build(113),
        ]);
        let s = similarity(&a, &b);
        assert!((s - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_different_lengths() {
        let a = seq(&[ActionKey::Train(7), ActionKey::Build(111)]);
        let b = seq(&[
            ActionKey::Train(7),
            ActionKey::Build(111),
            ActionKey::Train(0),
        ]);
        // Edit distance 1, max_len 3 → similarity 2/3
        let s = similarity(&a, &b);
        assert!((s - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_lcs_similarity_subsequence() {
        // a is a subsequence of b → LCS = len(a) = 2, max_len = 3
        let a = seq(&[ActionKey::Train(7), ActionKey::Build(111)]);
        let b = seq(&[
            ActionKey::Train(7),
            ActionKey::Train(0),
            ActionKey::Build(111),
        ]);
        let s = lcs_similarity(&a, &b);
        assert!((s - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_lcs_no_common() {
        let a = seq(&[ActionKey::Train(0)]);
        let b = seq(&[ActionKey::Build(111)]);
        assert!((lcs_similarity(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_from_build_order() {
        let bo = vec![
            entry(100, 0, BuildAction::Train(7)),
            entry(200, 1, BuildAction::Train(64)),
            entry(300, 0, BuildAction::Build(111)),
            entry(400, 0, BuildAction::Train(0)),
        ];

        let p0 = BuildSequence::from_build_order(&bo, 0);
        assert_eq!(p0.len(), 3); // Train(7), Build(111), Train(0)

        let p1 = BuildSequence::from_build_order(&bo, 1);
        assert_eq!(p1.len(), 1); // Train(64)
    }

    #[test]
    fn test_from_build_order_opening() {
        let bo = vec![
            entry(100, 0, BuildAction::Train(7)),
            entry(200, 0, BuildAction::Train(7)),
            entry(300, 0, BuildAction::Build(111)),
            entry(400, 0, BuildAction::Train(0)),
            entry(500, 0, BuildAction::Train(0)),
        ];

        let opening = BuildSequence::from_build_order_opening(&bo, 0, 3);
        assert_eq!(opening.len(), 3);
    }

    #[test]
    fn test_compare() {
        let a = seq(&[ActionKey::Train(7), ActionKey::Build(111)]);
        let b = seq(&[ActionKey::Train(7), ActionKey::Build(113)]);
        let result = compare(&a, &b);
        assert_eq!(result.len_a, 2);
        assert_eq!(result.len_b, 2);
        assert!((result.edit_similarity - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rank_by_similarity() {
        let query = seq(&[
            ActionKey::Train(7),
            ActionKey::Build(111),
            ActionKey::Train(0),
        ]);
        let candidates = vec![
            seq(&[ActionKey::Build(160), ActionKey::Train(65)]), // very different
            seq(&[
                ActionKey::Train(7),
                ActionKey::Build(111),
                ActionKey::Train(0),
            ]), // identical
            seq(&[
                ActionKey::Train(7),
                ActionKey::Build(111),
                ActionKey::Train(32),
            ]), // close
        ];

        let ranked = rank_by_similarity(&query, &candidates);
        assert_eq!(ranked[0].0, 1); // identical first
        assert!((ranked[0].1 - 1.0).abs() < 0.001);
        assert_eq!(ranked[1].0, 2); // close second
        assert_eq!(ranked[2].0, 0); // different last
    }
}
