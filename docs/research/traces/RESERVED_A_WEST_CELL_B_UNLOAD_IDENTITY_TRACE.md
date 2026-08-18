# Reserved A vs West-Cell B Unload Identity Trace

**Scenario:** Miner is already in refinery unload state 3 with `reserved_refinery=A`, while the first building in the miner current cell + `(-1,0)` is `B`. Verify state-3 unload effects use `B`; `A` remains Rust reservation/contact bookkeeping only.

**Scope:** One concrete state-3 HARV unload identity case. No radio `0x16`, far-return fallback, teleport lifecycle, or fixes.

**Ghidra status:** Ghidra MCP was available but no running Ghidra instance was connected in this subagent run (`list_instances` returned none). Binary evidence below therefore cites existing checked-in verified Ghidra research reports, not a fresh decompile. Any point requiring a fresh live/runtime computation is marked `UNCHECKED`.

## Pipeline

1. Trigger: `HARV` at `(13,11)`, `Dock/Unloading`, `reserved_refinery=2` (`A`), one ore cargo slot worth `100`.
2. Rust west-cell lookup: `mission_deploy_unload_building` reads miner cell `(13,11)`, computes `(12,11)`, scans the miner's occupancy-list layer, and returns the first live structure there, stable id `3` (`B`).
3. State-3 drain: `phase_unloading` removes the ore slot only after the west-cell building is found.
4. Unload effects: credit owner, purifier owner context, and `BaleDepositEvent.building_id` use the rediscovered building id `3`.
5. Bookkeeping cleanup: completion/release paths still release reservation/contact against `ref_sid=2` (`A`) and clear `reserved_refinery`.

## Stage Verdicts

### Stage 1 - Active YR Path

- Rust: `phase_unloading` is the `RefineryDockPhase::Unloading` handler at `src/sim/miner/miner_dock_sequence.rs:1008`.
- gamemd: verified active YR stock path is `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `[HARV] Harvester=yes`, `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes` are active stock YR keys.
- Evidence: `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md:97-116`, `:152`.
- Verdict: `PASS`.

### Stage 2 - West-Cell Building Identity

- Rust computation: miner cell `(13,11)` enters `mission_deploy_unload_building`; because `rx != 0`, it computes `lookup_rx=12`, `lookup_ry=11`, scans `sim.occupancy.get(12,11).iter_layer(Ground)`, and returns the first live `Structure`, stable id `3` in this scenario (`src/sim/miner/miner_dock_sequence.rs:432-456`; fixture at `src/sim/miner/miner_tests.rs:4648-4650`).
- gamemd computation: runtime `DAT_0089F6A0/A2` is `(-1,0)`, and state-3 dump lookup adds it to current cell before `Look_up_building_in_cell`; for current `(13,11)`, lookup cell is `(12,11)`, so first building `B` is selected.
- Evidence: `DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE_GHIDRA_REPORT.md:27-31`, `:113-116`, `:191-193`; `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md:48-66`.
- Verdict: `PASS` for this concrete "first building is B" case. Broader multi-building ordering remains outside this slot.

### Stage 3 - Credit Owner Uses B, Not A

- Rust computation: after `unload_building_id=3` is found, `phase_unloading` derives `refinery_owner` from entity `3`, then credits that owner (`src/sim/miner/miner_dock_sequence.rs:1068-1077`). The focused fixture expects `Americans` (`A`) unchanged and `Germans` (`B`) +100 (`src/sim/miner/miner_tests.rs:4662-4667`).
- gamemd computation: state-3 lookup result `this_00` drives credits/anims; base credit call is `HouseClass::Add_Tiberium_Credits @ 0x0073E4A9`.
- Evidence: `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md:65`; `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md:672-675`, `:755-758`.
- Verdict: `PASS`.

### Stage 4 - Purifier Context Uses B Owner

- Rust computation: `effective_purifier_count(sim, rules, &refinery_owner)` uses the same `refinery_owner` derived from `unload_building_id=3` (`src/sim/miner/miner_dock_sequence.rs:1079-1094`).
- gamemd evidence: the existing unload report records separate base/bonus credit calls and a single building-owner path for the deposit logic, but this run did not fresh-decompile the exact owner value feeding purifier count for the A/B mismatch.
- Evidence: `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md:672-675`, `:757-758`; Rust lines above.
- Verdict: `UNCHECKED` for strict mechanism equality in this A/B mismatch because no fresh Ghidra/runtime computation was possible. Rust is shaped to use B.

### Stage 5 - BaleDepositEvent Building Identity Uses B

- Rust computation: after crediting, Rust emits exactly one `BaleDepositEvent { building_id: unload_building_id, tick }`, so this scenario emits `building_id=3` (`src/sim/miner/miner_dock_sequence.rs:1096-1101`; event field definition at `src/sim/components.rs:673-681`; fixture assertion at `src/sim/miner/miner_tests.rs:4668-4670`).
- gamemd computation: gamemd has no `BaleDepositEvent`; the parity target is the building identity used for deposit visuals/effects, and state-3 `this_00` comes from the west-cell lookup before deposit effects.
- Evidence: `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md:65`; `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md:750-758`.
- Verdict: `PASS` for Rust event identity matching the gamemd deposit-effect building identity.

### Stage 6 - Reserved A Is Bookkeeping Only

- Rust computation: state-3 effects do not read `ref_sid` for owner, purifier context, or event identity; `ref_sid` is passed only to abort/completion bookkeeping. Completion uses `release_on_pad(ref_sid, miner)`, `release_contact(ref_sid, miner)`, then clears `reserved_refinery` (`src/sim/miner/miner_dock_sequence.rs:1046-1101`, `:1132-1168`). The focused test seeds contact/on-pad for `A=2`, then expects German credits and release of A contact/on-pad (`src/sim/miner/miner_tests.rs:4775-4801`).
- gamemd computation: the active zero-link unload loop does not use `unit/building +0x2E4` to find the state-3 unload building; the nonzero `+0x2E4` branch is a separate release branch.
- Evidence: `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md:12-14`, `:84-95`, `:146-150`.
- Verdict: `PASS` for the requested Rust partitioning: A is not used for unload identity; it is used only for Rust dock bookkeeping.

## Failures / Not Implemented

None found in this concrete slot.

## Unchecked

- Purifier bonus owner/context is Rust-shaped to use B, but strict gamemd equality for an A/B mismatch was not freshly computed because no Ghidra instance was connected.
- Tests were not executed in this run because the subagent was constrained to write exactly this one trace report and no other filesystem outputs; cargo test would write under `target/`.

## Adjacent Findings

- The null west-cell branch, display latch behavior, and state-4 non-refinery guard are covered by sibling slots and were not traced here.
- Broader multi-building object-list ordering is outside this slot. This trace only covers the concrete input where the first west-cell building is B.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0
