# Chrono Miner Current Fix Queue - System Model Synthesis

**Date:** 2026-05-26  
**Mode:** conflict-map / implementation triage  
**Scope:** stock YR chrono miner return, refinery docking/unload, facing/turn handoff, far-return staging, and locomotor handoff surfaces visible in current Rust.  
**Non-scope:** fresh Ghidra decompilation, runtime video capture, implementation patches.

## Current Model

The broad chrono miner return model is mostly past the older high-level failures. Current Rust parses `ChronoHarvTooFarDistance`, stores it in `MinerConfig::too_far_threshold_chrono`, uses a strict far-return `>` test, separates far/refused `QueueingCell` staging from accepted `CAN_DOCK` `NW+(3,1)`, and avoids normal stock post-unload `Force_Track(0x47)`.

The remaining work is narrower mechanism parity. The highest-priority active mismatch is dock radio `0x16`: Rust still treats it as an East-facing pivot/unload gate, while newer audited research says first ordinary `0x16` only syncs the locomotor/facing timer and returns; a later resend can trigger the `0x15` handoff.

The previous ROT mismatch is no longer current: `src/rules/object_type.rs` now preserves parsed `ROT=5` for stock CMIN and has a regression test.

## Claim Table

| Claim | Best evidence | Status | Safe? |
|---|---|---|---|
| Stock CMIN close/far return uses `ChronoHarvTooFarDistance=50`, 3D lepton distance, inclusive close, strict far `>` | `CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`; source scan | confirmed/aligned | implementation-safe |
| Accepted dock target is hardcoded `NW+(3,1)`, distinct from art `QueueingCell` | same plus dock acceptance audits | confirmed/aligned | implementation-safe |
| Normal stock zero-link unload completion does not use `ReleaseDockedHarvester` or `Force_Track(0x47)` | 2026-05-21/24 dock audits; source scan | confirmed/aligned | implementation-safe |
| First ordinary radio `0x16` is not an East-facing deploy gate | `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`; `.swarm-claims.md` 2026-05-24 entries | repo delta | implementation-safe to fix |
| Stock CMIN effective `ROT=` remains 5 | `CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES_GHIDRA_REPORT.md`; `object_type.rs` | confirmed/aligned | no fix needed |
| Far-return fallback nearby-passable search uses gamemd candidate collection and radius semantics | `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`; source scan | partial repo delta | implementation-safe with care |
| Unload state 3/4 refinery rediscovery uses miner current cell + `(-1,0)`, not stored `reserved_refinery` as authority | `DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE_GHIDRA_REPORT.md` | repo delta | implementation-safe to fix |
| Normal Drive-over-Teleport piggyback lifecycle exactly matches `Is_Ok_To_End` restore timing | teleport/locomotor docs plus source scan | partially modeled, not proven exact | needs contract/reinvestigation |
| Teleport side effects beyond relocation/sound/occupancy are complete | `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`; source scan | unknown | needs focused audit |

## What To Fix First

1. Fix dock `0x16` semantics in `src/sim/miner/miner_dock_sequence.rs`.
   - Remove the current assumption that `0x16` means "rotate to East, then start unload".
   - Model first `0x16` as timer/locomotor sync returning success without immediate unload.
   - Preserve the stock resend path through Mission_Enter retry cadence before later `0x16`/`0x15` handoff.
   - Update tests that currently expect `dock_pivot_facing` or immediate East-facing unload.

2. Fix far-return fallback passable-cell search details.
   - `EXIT_SEARCH_MAX_RADIUS` is still 16, while research says the effective helper cap is 32.
   - `find_nearby_passable_cell_with_index` collects a full ring but does not enforce the verified 24-candidate cap/direct-vs-indirect behavior.
   - This affects CMIN far/refused staging and conditional release fallback positions.

3. Fix unload state 3/4 refinery rediscovery.
   - Current dock flow uses `reserved_refinery` through `phase_unloading`/`phase_departing`.
   - Verified stock zero-link unload rediscovers the building at `miner_cell + (-1,0)` through the `DAT_0089F6A0` offset path.
   - Keep `reserved_refinery` for Rust bookkeeping only after verifying it cannot change stock-visible behavior.

4. Audit or contract the normal CMIN Drive/Teleport piggyback lifecycle.
   - Current source has `begin_drive_piggyback_for_teleporter`, `restore_primary_from_piggyback`, and a Set_Destination bridge.
   - Exact `Is_Ok_To_End` timing, active-locomotor ownership, and restore side effects are not yet proven equivalent.

5. Audit teleport side effects.
   - Current implementation covers relocation, occupancy, sounds, and cleanup.
   - Research points to additional target/animation/mission interactions that need a focused source-vs-doc audit before claiming parity.

## Disagreements To Retire

- "Rust still uses a 2-cell chrono return threshold" is stale. Current Rust uses parsed `ChronoHarvTooFarDistance`.
- "Normal stock CMIN unload exit calls `Force_Track(0x47)`" is stale for zero-link GAREFN/NAREFN completion. Conditional reciprocal-link and interrupt paths remain separate.
- "Stock CMIN ROT is still forced to 10 in current Rust" is stale. Current parser keeps parsed `ROT=5`.

## Source Ledger

- `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`
- `docs/research/miner/DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE_GHIDRA_REPORT.md`
- `docs/research/miner/CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `docs/research/.swarm-claims.md`
- Current source scan: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/movement/locomotor.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/teleport_movement.rs`, `src/rules/object_type.rs`.
