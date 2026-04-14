//! Build order classification — label openings by name.
//!
//! Matches the first N build order actions against known competitive BW
//! opening patterns per race. Returns the best-matching opening name and
//! confidence score.
//!
//! ## Known openings
//!
//! - **Terran**: Barracks first, 2 Rax, 1-1-1, FD (Factory → Depot), Bio push, Mech
//! - **Zerg**: 4/5/9/12 Pool, 12 Hatch, 3 Hatch before Pool, Lair tech
//! - **Protoss**: 2 Gate, 1 Gate Core, Forge FE, Nexus First, DT rush, Fast Expand

use crate::analysis::{BuildAction, BuildOrderEntry};
use crate::header::Race;

/// Result of classifying a player's opening build order.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpeningClassification {
    /// Human-readable opening name (e.g., "9 Pool", "1 Gate Core").
    pub name: String,
    /// Short tag for database indexing (e.g., "9pool", "1gate_core").
    pub tag: String,
    /// Confidence score (0.0–1.0). Higher = more certain match.
    pub confidence: f64,
    /// Player's race.
    pub race: String,
    /// Number of build actions considered.
    pub actions_analyzed: usize,
}

/// Classify a player's opening from their build order.
///
/// Analyzes the first `max_actions` build actions (default 15) for the
/// given player and matches against known opening patterns.
pub fn classify_opening(
    build_order: &[BuildOrderEntry],
    player_id: u8,
    race: &Race,
) -> OpeningClassification {
    let actions: Vec<&BuildAction> = build_order
        .iter()
        .filter(|e| e.player_id == player_id)
        .take(20)
        .map(|e| &e.action)
        .collect();

    let race_str = match race {
        Race::Terran => "T",
        Race::Protoss => "P",
        Race::Zerg => "Z",
        Race::Unknown(_) => "?",
    };

    let (name, tag, confidence) = match race {
        Race::Terran => classify_terran(&actions),
        Race::Zerg => classify_zerg(&actions, build_order, player_id),
        Race::Protoss => classify_protoss(&actions),
        Race::Unknown(_) => ("Unknown".to_string(), "unknown".to_string(), 0.0),
    };

    OpeningClassification {
        name,
        tag,
        confidence,
        race: race_str.to_string(),
        actions_analyzed: actions.len(),
    }
}

/// Classify all players' openings from a replay's build order.
pub fn classify_all(
    build_order: &[BuildOrderEntry],
    players: &[(u8, Race)],
) -> Vec<OpeningClassification> {
    players
        .iter()
        .map(|(pid, race)| classify_opening(build_order, *pid, race))
        .collect()
}

// ---------------------------------------------------------------------------
// Building ID constants
// ---------------------------------------------------------------------------

const BARRACKS: u16 = 111;
const FACTORY: u16 = 113;
const STARPORT: u16 = 114;
const ACADEMY: u16 = 112;
const COMMAND_CENTER: u16 = 106;
const SUPPLY_DEPOT: u16 = 109;
const REFINERY: u16 = 110;
const BUNKER: u16 = 125;
const MACHINE_SHOP: u16 = 120;

const SPAWNING_POOL: u16 = 142;
const HATCHERY: u16 = 131;
const LAIR: u16 = 132;
const HYDRA_DEN: u16 = 135;
const SPIRE: u16 = 141;

const GATEWAY: u16 = 160;
const FORGE: u16 = 166;
const NEXUS: u16 = 154;
const CYBERNETICS: u16 = 164;
const ASSIMILATOR: u16 = 157;
const PYLON: u16 = 156;
const CITADEL: u16 = 163;
const TEMPLAR_ARCHIVES: u16 = 165;
const STARGATE: u16 = 167;
const ROBOTICS: u16 = 155;
const CANNON: u16 = 162;

// ---------------------------------------------------------------------------
// Terran classification
// ---------------------------------------------------------------------------

fn classify_terran(actions: &[&BuildAction]) -> (String, String, f64) {
    let buildings: Vec<u16> = actions
        .iter()
        .filter_map(|a| match a {
            BuildAction::Build(id) => Some(*id),
            _ => None,
        })
        .collect();

    if buildings.is_empty() {
        return ("Unknown".into(), "unknown".into(), 0.0);
    }

    let rax_count = buildings.iter().filter(|&&b| b == BARRACKS).count();
    let has_factory = buildings.contains(&FACTORY);
    let has_starport = buildings.contains(&STARPORT);
    let has_academy = buildings.contains(&ACADEMY);
    let cc_count = buildings.iter().filter(|&&b| b == COMMAND_CENTER).count();

    // Check order-sensitive patterns.
    let first_production = buildings
        .iter()
        .find(|&&b| b != SUPPLY_DEPOT && b != REFINERY && b != COMMAND_CENTER);

    // 1-1-1: Barracks, Factory, Starport.
    if rax_count >= 1 && has_factory && has_starport && !buildings.contains(&ACADEMY) {
        return ("1-1-1".into(), "111".into(), 0.85);
    }

    // 2 Rax: two barracks before factory.
    if rax_count >= 2 && !has_factory {
        if has_academy {
            return ("2 Rax Academy".into(), "2rax_academy".into(), 0.8);
        }
        return ("2 Rax".into(), "2rax".into(), 0.8);
    }

    // Bio: Barracks + Academy, no factory early.
    if rax_count >= 1 && has_academy && !has_factory {
        return ("Bio".into(), "bio".into(), 0.7);
    }

    // Mech: Factory before second Barracks.
    if has_factory && rax_count == 1 {
        if has_starport {
            return ("Mech".into(), "mech".into(), 0.75);
        }
        let has_machine_shop = buildings.contains(&MACHINE_SHOP);
        if has_machine_shop {
            return ("Factory Expand".into(), "fe_factory".into(), 0.7);
        }
        return ("1 Rax FE".into(), "1rax_fe".into(), 0.6);
    }

    // CC first (FE).
    if cc_count >= 2 && first_production == Some(&COMMAND_CENTER) {
        return ("CC First".into(), "cc_first".into(), 0.75);
    }

    // Bunker rush.
    if buildings.contains(&BUNKER) && buildings.iter().position(|&b| b == BUNKER).unwrap_or(99) < 4
    {
        return ("Bunker Rush".into(), "bunker_rush".into(), 0.7);
    }

    // Default: Barracks first.
    if first_production == Some(&BARRACKS) {
        return ("Barracks First".into(), "rax_first".into(), 0.5);
    }

    ("Unknown Terran".into(), "unknown_t".into(), 0.3)
}

// ---------------------------------------------------------------------------
// Zerg classification
// ---------------------------------------------------------------------------

fn classify_zerg(
    actions: &[&BuildAction],
    build_order: &[BuildOrderEntry],
    player_id: u8,
) -> (String, String, f64) {
    let buildings: Vec<u16> = actions
        .iter()
        .filter_map(|a| match a {
            BuildAction::Build(id) | BuildAction::BuildingMorph(id) => Some(*id),
            _ => None,
        })
        .collect();

    if buildings.is_empty() {
        return ("Unknown".into(), "unknown".into(), 0.0);
    }

    let pool_idx = buildings.iter().position(|&b| b == SPAWNING_POOL);
    let hatch_idx = buildings.iter().position(|&b| b == HATCHERY);
    let has_lair = buildings.contains(&LAIR);
    let has_hydra = buildings.contains(&HYDRA_DEN);
    let has_spire = buildings.contains(&SPIRE);

    // Count workers trained before pool to estimate supply timing.
    let pool_frame = build_order
        .iter()
        .filter(|e| e.player_id == player_id)
        .find(|e| matches!(e.action, BuildAction::Build(id) if id == SPAWNING_POOL))
        .map(|e| e.frame);

    let drones_before_pool = pool_frame
        .map(|pf| {
            build_order
                .iter()
                .filter(|e| {
                    e.player_id == player_id
                        && e.frame < pf
                        && matches!(e.action, BuildAction::Train(41)) // Drone = 41
                })
                .count()
        })
        .unwrap_or(0);

    // X Pool naming: approximate supply count from drone count.
    // Start with 4 drones (+ 1 building overlord at 9), so pool at supply = 4 + drones_before.
    let pool_supply = 4 + drones_before_pool;

    // 3 Hatch before Pool.
    if let (Some(pi), Some(hi)) = (pool_idx, hatch_idx)
        && hi < pi
    {
        let hatch_count = buildings[..pi].iter().filter(|&&b| b == HATCHERY).count();
        if hatch_count >= 2 {
            return ("3 Hatch Before Pool".into(), "3hatch_pool".into(), 0.85);
        }
        return ("12 Hatch".into(), "12hatch".into(), 0.8);
    }

    // Pool timing variants.
    if pool_idx.is_some() {
        // Spire rush.
        if has_spire && !has_hydra {
            return ("Muta Build".into(), "muta".into(), 0.75);
        }
        // Hydra build.
        if has_hydra {
            return ("Hydra Build".into(), "hydra".into(), 0.7);
        }
        // Lair tech.
        if has_lair {
            return ("Lair Tech".into(), "lair_tech".into(), 0.65);
        }

        if pool_supply <= 5 {
            return ("4/5 Pool".into(), "4pool".into(), 0.85);
        }
        if pool_supply <= 9 {
            return ("9 Pool".into(), "9pool".into(), 0.8);
        }
        if pool_supply <= 12 {
            return ("12 Pool".into(), "12pool".into(), 0.75);
        }
        return ("Overpool".into(), "overpool".into(), 0.65);
    }

    ("Unknown Zerg".into(), "unknown_z".into(), 0.3)
}

// ---------------------------------------------------------------------------
// Protoss classification
// ---------------------------------------------------------------------------

fn classify_protoss(actions: &[&BuildAction]) -> (String, String, f64) {
    let buildings: Vec<u16> = actions
        .iter()
        .filter_map(|a| match a {
            BuildAction::Build(id) => Some(*id),
            _ => None,
        })
        .collect();

    if buildings.is_empty() {
        return ("Unknown".into(), "unknown".into(), 0.0);
    }

    let gate_count = buildings.iter().filter(|&&b| b == GATEWAY).count();
    let has_core = buildings.contains(&CYBERNETICS);
    let has_forge = buildings.contains(&FORGE);
    let nexus_count = buildings.iter().filter(|&&b| b == NEXUS).count();
    let has_citadel = buildings.contains(&CITADEL);
    let has_archives = buildings.contains(&TEMPLAR_ARCHIVES);
    let has_stargate = buildings.contains(&STARGATE);
    let has_robo = buildings.contains(&ROBOTICS);
    let has_cannon = buildings.contains(&CANNON);

    let first_non_pylon = buildings
        .iter()
        .find(|&&b| b != PYLON && b != ASSIMILATOR && b != NEXUS);

    // Nexus first.
    if nexus_count >= 2 && first_non_pylon == Some(&NEXUS) {
        return ("Nexus First".into(), "nexus_first".into(), 0.8);
    }

    // Forge FE: Forge before Gateway.
    let forge_idx = buildings.iter().position(|&b| b == FORGE);
    let gate_idx = buildings.iter().position(|&b| b == GATEWAY);
    if let (Some(fi), Some(gi)) = (forge_idx, gate_idx)
        && fi < gi
    {
        if has_cannon {
            return ("Forge FE + Cannon".into(), "forge_fe_cannon".into(), 0.85);
        }
        return ("Forge FE".into(), "forge_fe".into(), 0.85);
    }
    // Forge without gateway yet.
    if has_forge && gate_count == 0 {
        return ("Forge FE".into(), "forge_fe".into(), 0.75);
    }

    // DT rush: Gateway → Core → Citadel → Archives.
    if has_archives && gate_count >= 1 {
        return ("DT Rush".into(), "dt_rush".into(), 0.8);
    }

    // 2 Gate: two gateways before Cybernetics Core.
    if gate_count >= 2 {
        if has_core {
            return ("2 Gate Core".into(), "2gate_core".into(), 0.8);
        }
        return ("2 Gate".into(), "2gate".into(), 0.8);
    }

    // 1 Gate variants.
    if gate_count == 1 && has_core {
        if has_stargate {
            return ("1 Gate Stargate".into(), "1gate_stargate".into(), 0.8);
        }
        if has_robo {
            return ("1 Gate Robo".into(), "1gate_robo".into(), 0.8);
        }
        if has_citadel {
            return ("1 Gate Citadel".into(), "1gate_citadel".into(), 0.75);
        }
        if nexus_count >= 2 {
            return ("1 Gate FE".into(), "1gate_fe".into(), 0.8);
        }
        return ("1 Gate Core".into(), "1gate_core".into(), 0.7);
    }

    if gate_count == 1 {
        return ("Gateway First".into(), "gate_first".into(), 0.5);
    }

    ("Unknown Protoss".into(), "unknown_p".into(), 0.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bo_entry(frame: u32, pid: u8, action: BuildAction) -> BuildOrderEntry {
        BuildOrderEntry {
            frame,
            real_seconds: frame as f64 / 23.81,
            player_id: pid,
            action,
        }
    }

    // -- Terran --

    #[test]
    fn test_classify_111() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(SUPPLY_DEPOT)),
            bo_entry(800, 0, BuildAction::Build(BARRACKS)),
            bo_entry(1200, 0, BuildAction::Build(REFINERY)),
            bo_entry(1800, 0, BuildAction::Build(FACTORY)),
            bo_entry(2500, 0, BuildAction::Build(STARPORT)),
        ];
        let c = classify_opening(&bo, 0, &Race::Terran);
        assert_eq!(c.tag, "111");
        assert!(c.confidence > 0.7);
    }

    #[test]
    fn test_classify_2rax() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(SUPPLY_DEPOT)),
            bo_entry(800, 0, BuildAction::Build(BARRACKS)),
            bo_entry(1200, 0, BuildAction::Build(BARRACKS)),
        ];
        let c = classify_opening(&bo, 0, &Race::Terran);
        assert_eq!(c.tag, "2rax");
    }

    // -- Zerg --

    #[test]
    fn test_classify_9pool() {
        let bo = vec![
            // 5 drones before pool (start with 4 = supply 9)
            bo_entry(100, 0, BuildAction::Train(41)),
            bo_entry(200, 0, BuildAction::Train(41)),
            bo_entry(300, 0, BuildAction::Train(41)),
            bo_entry(400, 0, BuildAction::Train(41)),
            bo_entry(500, 0, BuildAction::Train(41)),
            bo_entry(600, 0, BuildAction::Build(SPAWNING_POOL)),
        ];
        let c = classify_opening(&bo, 0, &Race::Zerg);
        assert_eq!(c.tag, "9pool");
    }

    #[test]
    fn test_classify_4pool() {
        let bo = vec![
            // 0 drones before pool (supply 4)
            bo_entry(100, 0, BuildAction::Build(SPAWNING_POOL)),
        ];
        let c = classify_opening(&bo, 0, &Race::Zerg);
        assert_eq!(c.tag, "4pool");
    }

    #[test]
    fn test_classify_12hatch() {
        let bo = vec![
            bo_entry(100, 0, BuildAction::Train(41)),
            bo_entry(200, 0, BuildAction::Train(41)),
            bo_entry(300, 0, BuildAction::Train(41)),
            bo_entry(400, 0, BuildAction::Build(HATCHERY)),
            bo_entry(800, 0, BuildAction::Build(SPAWNING_POOL)),
        ];
        let c = classify_opening(&bo, 0, &Race::Zerg);
        assert_eq!(c.tag, "12hatch");
    }

    // -- Protoss --

    #[test]
    fn test_classify_1gate_core() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(PYLON)),
            bo_entry(800, 0, BuildAction::Build(GATEWAY)),
            bo_entry(1200, 0, BuildAction::Build(ASSIMILATOR)),
            bo_entry(1600, 0, BuildAction::Build(CYBERNETICS)),
        ];
        let c = classify_opening(&bo, 0, &Race::Protoss);
        assert_eq!(c.tag, "1gate_core");
    }

    #[test]
    fn test_classify_forge_fe() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(PYLON)),
            bo_entry(800, 0, BuildAction::Build(FORGE)),
            bo_entry(1200, 0, BuildAction::Build(CANNON)),
            bo_entry(1600, 0, BuildAction::Build(NEXUS)),
            bo_entry(2000, 0, BuildAction::Build(GATEWAY)),
        ];
        let c = classify_opening(&bo, 0, &Race::Protoss);
        assert!(c.tag.starts_with("forge_fe"));
    }

    #[test]
    fn test_classify_2gate() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(PYLON)),
            bo_entry(800, 0, BuildAction::Build(GATEWAY)),
            bo_entry(1000, 0, BuildAction::Build(GATEWAY)),
        ];
        let c = classify_opening(&bo, 0, &Race::Protoss);
        assert_eq!(c.tag, "2gate");
    }

    // -- General --

    #[test]
    fn test_classify_all() {
        let bo = vec![
            bo_entry(500, 0, BuildAction::Build(BARRACKS)),
            bo_entry(600, 1, BuildAction::Build(SPAWNING_POOL)),
        ];
        let results = classify_all(&bo, &[(0, Race::Terran), (1, Race::Zerg)]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].race, "T");
        assert_eq!(results[1].race, "Z");
    }

    #[test]
    fn test_classify_empty() {
        let c = classify_opening(&[], 0, &Race::Terran);
        assert_eq!(c.tag, "unknown");
    }
}
