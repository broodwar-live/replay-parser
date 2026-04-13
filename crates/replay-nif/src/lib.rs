//! Erlang NIF bindings for the BW replay parser via Rustler.
//!
//! All NIF functions run on dirty CPU schedulers to avoid blocking the BEAM.
//! Returns are Elixir-native terms (maps, lists, atoms) — not serialized JSON.
//!
//! ## Elixir Usage
//!
//! ```elixir
//! defmodule BroodwarNif.ReplayParser do
//!   use Rustler, otp_app: :broodwar, crate: "replay_nif"
//!
//!   def parse_replay(_data), do: :erlang.nif_error(:not_loaded)
//!   def parse_header(_data), do: :erlang.nif_error(:not_loaded)
//!   def extract_build_order(_data), do: :erlang.nif_error(:not_loaded)
//!   def calculate_apm(_data), do: :erlang.nif_error(:not_loaded)
//!   def apm_over_time(_data, _window_secs, _step_secs), do: :erlang.nif_error(:not_loaded)
//! end
//! ```

use rustler::{Encoder, Env, NifResult, Term};

mod atoms {
    rustler::atoms! {
        ok,
        error,

        // Engine
        starcraft,
        brood_war,

        // Speed
        slowest,
        slower,
        slow,
        normal,
        fast,
        faster,
        fastest,

        // Race
        zerg,
        terran,
        protoss,
        unknown,

        // Player type
        inactive,
        computer,
        human,
        rescue_passive,
        computer_controlled,
        open,
        neutral,
        closed,

        // Build actions
        build,
        train,
        unit_morph,
        building_morph,
        research,
        upgrade,
        train_fighter,

        // Map keys
        header,
        build_order,
        player_apm,
        timeline,
        command_count,
        map_data,
        engine,
        frame_count,
        duration_secs,
        start_time,
        game_title,
        map_name,
        map_width,
        map_height,
        game_speed,
        game_type,
        host_name,
        players,
        slot_id,
        player_id,
        player_type,
        race,
        race_code,
        team,
        name,
        color,
        frame,
        real_seconds,
        action,
        apm,
        eapm,
        minerals_invested,
        gas_invested,
        supply_used,
        supply_max,
        units,
        buildings,
        techs,
        upgrades,
    }
}

// ---------------------------------------------------------------------------
// NIF: parse_replay
// ---------------------------------------------------------------------------

/// Parse a full replay from raw `.rep` bytes.
///
/// Returns `{:ok, replay_map}` or `{:error, reason}`.
#[rustler::nif(schedule = "DirtyCpu")]
fn parse_replay<'a>(env: Env<'a>, data: rustler::Binary) -> NifResult<Term<'a>> {
    match replay_core::parse(data.as_slice()) {
        Ok(replay) => {
            let map = encode_replay(env, &replay);
            Ok((atoms::ok(), map).encode(env))
        }
        Err(e) => Ok((atoms::error(), e.to_string()).encode(env)),
    }
}

// ---------------------------------------------------------------------------
// NIF: parse_header
// ---------------------------------------------------------------------------

/// Parse only the replay header (lightweight — no command stream or build order).
///
/// Returns `{:ok, header_map}` or `{:error, reason}`.
#[rustler::nif(schedule = "DirtyCpu")]
fn parse_header<'a>(env: Env<'a>, data: rustler::Binary) -> NifResult<Term<'a>> {
    match replay_core::parse(data.as_slice()) {
        Ok(replay) => {
            let map = encode_header(env, &replay.header);
            Ok((atoms::ok(), map).encode(env))
        }
        Err(e) => Ok((atoms::error(), e.to_string()).encode(env)),
    }
}

// ---------------------------------------------------------------------------
// NIF: extract_build_order
// ---------------------------------------------------------------------------

/// Parse a replay and return only the build order.
///
/// Returns `{:ok, [build_order_entry]}` or `{:error, reason}`.
#[rustler::nif(schedule = "DirtyCpu")]
fn extract_build_order<'a>(env: Env<'a>, data: rustler::Binary) -> NifResult<Term<'a>> {
    match replay_core::parse(data.as_slice()) {
        Ok(replay) => {
            let list = encode_build_order(env, &replay.build_order);
            Ok((atoms::ok(), list).encode(env))
        }
        Err(e) => Ok((atoms::error(), e.to_string()).encode(env)),
    }
}

// ---------------------------------------------------------------------------
// NIF: calculate_apm
// ---------------------------------------------------------------------------

/// Parse a replay and return per-player APM/EAPM.
///
/// Returns `{:ok, [%{player_id, apm, eapm}]}` or `{:error, reason}`.
#[rustler::nif(schedule = "DirtyCpu")]
fn calculate_apm<'a>(env: Env<'a>, data: rustler::Binary) -> NifResult<Term<'a>> {
    match replay_core::parse(data.as_slice()) {
        Ok(replay) => {
            let list = encode_player_apm(env, &replay.player_apm);
            Ok((atoms::ok(), list).encode(env))
        }
        Err(e) => Ok((atoms::error(), e.to_string()).encode(env)),
    }
}

// ---------------------------------------------------------------------------
// NIF: apm_over_time
// ---------------------------------------------------------------------------

/// Parse a replay and return APM samples over time for graphing.
///
/// Returns `{:ok, [%{frame, real_seconds, player_id, apm, eapm}]}` or `{:error, reason}`.
#[rustler::nif(schedule = "DirtyCpu")]
fn apm_over_time<'a>(
    env: Env<'a>,
    data: rustler::Binary,
    window_secs: f64,
    step_secs: f64,
) -> NifResult<Term<'a>> {
    match replay_core::parse(data.as_slice()) {
        Ok(replay) => {
            let samples = replay.apm_over_time(window_secs, step_secs);
            let list = encode_apm_samples(env, &samples);
            Ok((atoms::ok(), list).encode(env))
        }
        Err(e) => Ok((atoms::error(), e.to_string()).encode(env)),
    }
}

// ---------------------------------------------------------------------------
// Encoding helpers — Rust types → Elixir maps/lists
// ---------------------------------------------------------------------------

fn encode_replay<'a>(env: Env<'a>, replay: &replay_core::Replay) -> Term<'a> {
    let header = encode_header(env, &replay.header);
    let build_order = encode_build_order(env, &replay.build_order);
    let player_apm = encode_player_apm(env, &replay.player_apm);
    let timeline = encode_timeline(env, &replay.timeline);
    let command_count = replay.commands.len();
    let map_data = replay.map_data.as_slice().encode(env);

    Term::map_from_pairs(
        env,
        &[
            (atoms::header().encode(env), header),
            (atoms::build_order().encode(env), build_order),
            (atoms::player_apm().encode(env), player_apm),
            (atoms::timeline().encode(env), timeline),
            (
                atoms::command_count().encode(env),
                command_count.encode(env),
            ),
            (atoms::map_data().encode(env), map_data),
        ],
    )
    .unwrap()
}

fn encode_header<'a>(env: Env<'a>, header: &replay_core::header::Header) -> Term<'a> {
    use replay_core::header::*;

    let engine_atom = match header.engine {
        Engine::StarCraft => atoms::starcraft().encode(env),
        Engine::BroodWar => atoms::brood_war().encode(env),
    };

    let speed = encode_speed(env, &header.game_speed);
    let game_type_str = format!("{:?}", header.game_type);
    let players_list: Vec<Term<'a>> = header
        .players
        .iter()
        .map(|p| encode_player(env, p))
        .collect();

    Term::map_from_pairs(
        env,
        &[
            (atoms::engine().encode(env), engine_atom),
            (
                atoms::frame_count().encode(env),
                header.frame_count.encode(env),
            ),
            (
                atoms::duration_secs().encode(env),
                header.duration_secs().encode(env),
            ),
            (
                atoms::start_time().encode(env),
                header.start_time.encode(env),
            ),
            (
                atoms::game_title().encode(env),
                header.game_title.as_str().encode(env),
            ),
            (
                atoms::map_name().encode(env),
                header.map_name.as_str().encode(env),
            ),
            (atoms::map_width().encode(env), header.map_width.encode(env)),
            (
                atoms::map_height().encode(env),
                header.map_height.encode(env),
            ),
            (atoms::game_speed().encode(env), speed),
            (atoms::game_type().encode(env), game_type_str.encode(env)),
            (
                atoms::host_name().encode(env),
                header.host_name.as_str().encode(env),
            ),
            (atoms::players().encode(env), players_list.encode(env)),
        ],
    )
    .unwrap()
}

fn encode_player<'a>(env: Env<'a>, player: &replay_core::header::Player) -> Term<'a> {
    use replay_core::header::*;

    let race_atom = match player.race {
        Race::Zerg => atoms::zerg().encode(env),
        Race::Terran => atoms::terran().encode(env),
        Race::Protoss => atoms::protoss().encode(env),
        Race::Unknown(_) => atoms::unknown().encode(env),
    };

    let type_atom = match player.player_type {
        PlayerType::Human => atoms::human().encode(env),
        PlayerType::Computer => atoms::computer().encode(env),
        PlayerType::Inactive => atoms::inactive().encode(env),
        PlayerType::RescuePassive => atoms::rescue_passive().encode(env),
        PlayerType::ComputerControlled => atoms::computer_controlled().encode(env),
        PlayerType::Open => atoms::open().encode(env),
        PlayerType::Neutral => atoms::neutral().encode(env),
        PlayerType::Closed => atoms::closed().encode(env),
        PlayerType::Unknown(_) => atoms::unknown().encode(env),
    };

    Term::map_from_pairs(
        env,
        &[
            (atoms::slot_id().encode(env), player.slot_id.encode(env)),
            (atoms::player_id().encode(env), player.player_id.encode(env)),
            (atoms::player_type().encode(env), type_atom),
            (atoms::race().encode(env), race_atom),
            (
                atoms::race_code().encode(env),
                player.race.code().encode(env),
            ),
            (atoms::team().encode(env), player.team.encode(env)),
            (atoms::name().encode(env), player.name.as_str().encode(env)),
            (atoms::color().encode(env), player.color.encode(env)),
        ],
    )
    .unwrap()
}

fn encode_speed<'a>(env: Env<'a>, speed: &replay_core::header::Speed) -> Term<'a> {
    use replay_core::header::Speed;
    match speed {
        Speed::Slowest => atoms::slowest().encode(env),
        Speed::Slower => atoms::slower().encode(env),
        Speed::Slow => atoms::slow().encode(env),
        Speed::Normal => atoms::normal().encode(env),
        Speed::Fast => atoms::fast().encode(env),
        Speed::Faster => atoms::faster().encode(env),
        Speed::Fastest => atoms::fastest().encode(env),
        Speed::Unknown(v) => v.encode(env),
    }
}

fn encode_build_order<'a>(
    env: Env<'a>,
    entries: &[replay_core::analysis::BuildOrderEntry],
) -> Term<'a> {
    let list: Vec<Term<'a>> = entries
        .iter()
        .map(|entry| {
            let action_term = encode_build_action(env, &entry.action);
            Term::map_from_pairs(
                env,
                &[
                    (atoms::frame().encode(env), entry.frame.encode(env)),
                    (
                        atoms::real_seconds().encode(env),
                        entry.real_seconds.encode(env),
                    ),
                    (atoms::player_id().encode(env), entry.player_id.encode(env)),
                    (atoms::action().encode(env), action_term),
                    (atoms::name().encode(env), entry.action.name().encode(env)),
                ],
            )
            .unwrap()
        })
        .collect();
    list.encode(env)
}

fn encode_build_action<'a>(env: Env<'a>, action: &replay_core::analysis::BuildAction) -> Term<'a> {
    use replay_core::analysis::BuildAction;
    match action {
        BuildAction::Build(id) => (atoms::build(), *id).encode(env),
        BuildAction::Train(id) => (atoms::train(), *id).encode(env),
        BuildAction::UnitMorph(id) => (atoms::unit_morph(), *id).encode(env),
        BuildAction::BuildingMorph(id) => (atoms::building_morph(), *id).encode(env),
        BuildAction::Research(id) => (atoms::research(), *id as u16).encode(env),
        BuildAction::Upgrade(id) => (atoms::upgrade(), *id as u16).encode(env),
        BuildAction::TrainFighter => atoms::train_fighter().encode(env),
    }
}

fn encode_player_apm<'a>(env: Env<'a>, apms: &[replay_core::analysis::PlayerApm]) -> Term<'a> {
    let list: Vec<Term<'a>> = apms
        .iter()
        .map(|a| {
            Term::map_from_pairs(
                env,
                &[
                    (atoms::player_id().encode(env), a.player_id.encode(env)),
                    (atoms::apm().encode(env), a.apm.encode(env)),
                    (atoms::eapm().encode(env), a.eapm.encode(env)),
                ],
            )
            .unwrap()
        })
        .collect();
    list.encode(env)
}

fn encode_apm_samples<'a>(env: Env<'a>, samples: &[replay_core::analysis::ApmSample]) -> Term<'a> {
    let list: Vec<Term<'a>> = samples
        .iter()
        .map(|s| {
            Term::map_from_pairs(
                env,
                &[
                    (atoms::frame().encode(env), s.frame.encode(env)),
                    (
                        atoms::real_seconds().encode(env),
                        s.real_seconds.encode(env),
                    ),
                    (atoms::player_id().encode(env), s.player_id.encode(env)),
                    (atoms::apm().encode(env), s.apm.encode(env)),
                    (atoms::eapm().encode(env), s.eapm.encode(env)),
                ],
            )
            .unwrap()
        })
        .collect();
    list.encode(env)
}

fn encode_timeline<'a>(
    env: Env<'a>,
    snapshots: &[replay_core::timeline::TimelineSnapshot],
) -> Term<'a> {
    let list: Vec<Term<'a>> = snapshots
        .iter()
        .map(|snap| {
            let player_states: Vec<Term<'a>> = snap
                .players
                .iter()
                .map(|ps| encode_player_state(env, ps))
                .collect();

            Term::map_from_pairs(
                env,
                &[
                    (atoms::frame().encode(env), snap.frame.encode(env)),
                    (
                        atoms::real_seconds().encode(env),
                        snap.real_seconds.encode(env),
                    ),
                    (atoms::players().encode(env), player_states.encode(env)),
                ],
            )
            .unwrap()
        })
        .collect();
    list.encode(env)
}

fn encode_player_state<'a>(env: Env<'a>, state: &replay_core::timeline::PlayerState) -> Term<'a> {
    // Convert BTreeMaps to lists of {key, value} tuples for Elixir.
    let units_list: Vec<(u16, u32)> = state.units.iter().map(|(&k, &v)| (k, v)).collect();
    let buildings_list: Vec<(u16, u32)> = state.buildings.iter().map(|(&k, &v)| (k, v)).collect();
    let upgrades_list: Vec<(u8, u8)> = state.upgrades.iter().map(|(&k, &v)| (k, v)).collect();

    Term::map_from_pairs(
        env,
        &[
            (atoms::player_id().encode(env), state.player_id.encode(env)),
            (
                atoms::minerals_invested().encode(env),
                state.minerals_invested.encode(env),
            ),
            (
                atoms::gas_invested().encode(env),
                state.gas_invested.encode(env),
            ),
            (
                atoms::supply_used().encode(env),
                state.supply_used.encode(env),
            ),
            (
                atoms::supply_max().encode(env),
                state.supply_max.encode(env),
            ),
            (atoms::units().encode(env), units_list.encode(env)),
            (atoms::buildings().encode(env), buildings_list.encode(env)),
            (atoms::techs().encode(env), state.techs.encode(env)),
            (atoms::upgrades().encode(env), upgrades_list.encode(env)),
        ],
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

rustler::init!("Elixir.BroodwarNif.ReplayParser");
