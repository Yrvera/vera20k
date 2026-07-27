//! Emit per-tick parity digests from a headless simulation run.
//!
//! The in-game path cannot currently produce a digest: the client dies building the unit
//! atlas (height exceeds the GPU limit) before the first sim tick. The simulation itself
//! has no such dependency, so this drives it directly and writes the same JSONL stream
//! the client would.
//!
//! **This validates the pipeline, not parity.** The scenario here is a small synthetic
//! setup, not a stock skirmish, so comparing its output against a captured gamemd session
//! will report differences that mean nothing about engine fidelity. What it does prove
//! end to end is that real `Simulation` state serialises, lands on disk, and survives the
//! consumer's strict parsing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vera20k::map::entities::EntityCategory;
use vera20k::sim::components::Health;
use vera20k::sim::game_entity::GameEntity;
use vera20k::sim::house_state::HouseState;
use vera20k::sim::parity_digest::ParityDigestSink;
use vera20k::sim::world::Simulation;

/// Matches the client's simulation cadence so tick numbering is comparable.
const SIM_TICK_MS: u32 = 1000 / 15;
const DEFAULT_TICKS: u64 = 600;

struct Args {
    out: PathBuf,
    ticks: u64,
}

fn parse_args() -> Result<Args, String> {
    let mut out: Option<PathBuf> = None;
    let mut ticks = DEFAULT_TICKS;
    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--out" => {
                out = Some(PathBuf::from(
                    argv.next().ok_or("--out needs a path".to_string())?,
                ));
            }
            "--ticks" => {
                ticks = argv
                    .next()
                    .ok_or("--ticks needs a count".to_string())?
                    .parse()
                    .map_err(|_| "--ticks must be a positive integer".to_string())?;
            }
            other => return Err(format!("unrecognised argument {other}")),
        }
    }
    Ok(Args {
        out: out.ok_or("--out <path> is required".to_string())?,
        ticks,
    })
}

/// Two houses with credits and a handful of entities.
///
/// Enough for every digest field to carry a non-trivial value; no rules are loaded, so
/// `advance_tick` runs without rules-driven behaviour.
fn build_simulation() -> Simulation {
    let mut sim = Simulation::new();

    for (index, (owner, side)) in [("Americans", 0u8), ("Russians", 1u8)].iter().enumerate() {
        let owner_id = sim.interner.intern(owner);
        sim.houses.insert(
            owner_id,
            HouseState::new(owner_id, *side, None, index == 0, 10_000, 10),
        );

        let type_ref = sim.interner.intern("GACNST");
        let base_x = 10 + (index as u16) * 20;
        for slot in 0..3u16 {
            let entity = GameEntity::new_at_frame(
                (index as u64) * 10 + slot as u64 + 1,
                base_x + slot * 2,
                12,
                0,
                0,
                owner_id,
                Health {
                    current: 1000,
                    max: 1000,
                },
                type_ref,
                EntityCategory::Structure,
                0,
                6,
                false,
                0,
            );
            sim.entities_mut().insert(entity);
        }
    }
    sim
}

fn main() -> Result<(), String> {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("usage: parity-digest --out <path.jsonl> [--ticks N]");
            std::process::exit(2);
        }
    };

    let mut sim = build_simulation();
    let mut sink = ParityDigestSink::create(&args.out)
        .map_err(|error| format!("could not open {}: {error}", args.out.display()))?;

    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..args.ticks {
        sim.advance_tick(&[], None, &heights, None, None, SIM_TICK_MS);
        let digest = sim.parity_digest();
        sink.write(&digest)
            .map_err(|error| format!("digest write failed: {error}"))?;
    }

    println!("wrote {} digests to {}", sink.written(), args.out.display());
    Ok(())
}
