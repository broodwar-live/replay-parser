use replay_core::header::{Engine, Race, Speed};

fn fixture(name: &str) -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest_dir}/../../tests/fixtures/{name}");
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn test_parse_1v1_melee() {
    let data = fixture("1v1melee.rep");
    let header = replay_core::parse(&data).expect("failed to parse 1v1melee.rep");

    assert_eq!(header.engine, Engine::BroodWar);
    assert_eq!(header.game_speed, Speed::Fastest);
    assert!(header.frame_count > 0, "frame_count should be non-zero");
    assert!(header.duration_secs() > 0.0, "duration should be positive");
    assert!(!header.map_name.is_empty(), "map_name should not be empty");
    assert!(
        header.players.len() >= 2,
        "expected at least 2 players, got {}",
        header.players.len()
    );

    // Every active player should have a non-empty name and a known race.
    for player in &header.players {
        assert!(!player.name.is_empty(), "player name should not be empty");
        assert!(
            matches!(player.race, Race::Terran | Race::Protoss | Race::Zerg),
            "unexpected race {:?} for player {}",
            player.race,
            player.name
        );
    }

    println!("=== 1v1melee.rep ===");
    println!("Map: {}", header.map_name);
    println!(
        "Duration: {:.0}s ({} frames)",
        header.duration_secs(),
        header.frame_count
    );
    println!("Speed: {:?}", header.game_speed);
    for p in &header.players {
        println!("  {} ({}) — {:?}", p.name, p.race.code(), p.player_type);
    }
}

#[test]
fn test_parse_larva_vs_mini() {
    let data = fixture("larva_vs_mini.rep");
    let header = replay_core::parse(&data).expect("failed to parse larva_vs_mini.rep");

    assert_eq!(header.engine, Engine::BroodWar);
    assert!(header.players.len() >= 2);
    assert!(header.frame_count > 0);
    assert!(!header.map_name.is_empty());

    println!("=== larva_vs_mini.rep ===");
    println!("Map: {}", header.map_name);
    println!(
        "Duration: {:.0}s ({} frames)",
        header.duration_secs(),
        header.frame_count
    );
    for p in &header.players {
        println!("  {} ({}) — {:?}", p.name, p.race.code(), p.player_type);
    }
}

#[test]
fn test_parse_polypoid() {
    let data = fixture("polypoid.rep");
    let header = replay_core::parse(&data).expect("failed to parse polypoid.rep");

    assert_eq!(header.engine, Engine::BroodWar);
    assert!(header.players.len() >= 2);

    // This replay is on Polypoid — verify the map name contains it.
    let map_lower = header.map_name.to_lowercase();
    assert!(
        map_lower.contains("polypoid"),
        "expected map name to contain 'polypoid', got '{}'",
        header.map_name
    );

    println!("=== polypoid.rep ===");
    println!("Map: {}", header.map_name);
    println!(
        "Duration: {:.0}s ({} frames)",
        header.duration_secs(),
        header.frame_count
    );
    for p in &header.players {
        println!("  {} ({}) — {:?}", p.name, p.race.code(), p.player_type);
    }
}
