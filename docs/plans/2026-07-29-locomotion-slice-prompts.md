---
title: Locomotion substrate — implementation prompts (S1–S8)
date: 2026-07-29
source: docs/plans/2026-07-29-locomotion-substrate-design.md
purpose: paste-ready agent prompts, one per migration slice
---

# Locomotion substrate — implementation prompts

Each block below is a self-contained prompt. Paste one into a fresh session or agent.
The design document is the spec; these prompts are the work orders.

## How to run these

**Dependency chain:** `S1 → S2 → S3 → S4 → S5`. `S6`, `S7` and `S8` are independent.

**Can start right now, in any order:** **S1** (recommended first — purely additive, machine-derived
golden, unblocks everything), **S6**, **S7**.

**Concurrency — read this before launching more than one.** ENGINE.md forbids running multiple
cargo commands in parallel from one session, and a second session's build will collide. Two agents
may *edit* concurrently, but only one may hold Cargo. Each prompt below instructs its agent to check
`Get-Process cargo,rustc` and wait. If you want genuine parallelism, run at most one cargo-holding
agent and let the others do read/design work — do **not** reach for git worktrees here: a fresh
worktree needs the gitignored `ini/` copied in, and `git worktree remove --force` follows symlinks
and has wiped this repo's gitignored `docs/` and `ini/` before.

**Sequencing suggestion:** S1 first and alone (it defines the module everything else imports). Then
S7 and S6 together with S2. Then S3, then S4, then S5. S8 whenever.

---

## Shared preamble

Every prompt below already contains this. Repeated here so you can see what each agent is told.

- Read `ENGINE.md` and `CLAUDE.md` in full first — they are the project contract.
- Read the named sections of `docs/plans/2026-07-29-locomotion-substrate-design.md`.
- **Rust-native structure, gamemd-native semantics.** Do not port COM, vtables, refcounting or an
  inheritance tree. Do reproduce the verified ordering and state-commit contract.
- **Never invent gamemd semantics.** If implementation requires a fact the design doc marks `[U]` /
  `UNCHECKED` / `UNKNOWN`, **stop and report** rather than guessing. An honest halt is the correct
  outcome; a plausible invention is the failure this project has paid for repeatedly.
- **Never introduce a gate, sentinel, clamp, fallback or heuristic that gamemd does not have**,
  unless it is labelled VERA-internal with the gamemd equivalent `UNCHECKED`.
- `sim/` must never depend on `render/`, `ui/`, `sidebar/`, `audio/`, `net/`. All sim math is
  fixed-point — no `f32`/`f64` in game logic.
- **Labelling discipline.** A test may only be described as parity/`VERIFIED` if its golden is
  machine-derived (retail INI bytes, `read_memory`, binary emulation) or exhaustive over the input
  space. A hand-transcribed sequence or a Rust-vs-prior-Rust comparison is a **ratchet** and must be
  labelled `UNCHECKED` in the module doc and test comment. Prose never upgrades a status.
- **Cargo discipline.** Check `Get-Process cargo,rustc -ErrorAction SilentlyContinue` first; if
  another session owns Cargo, wait. Use `cargo check -p vera20k` while working and
  `cargo test -p vera20k --lib <module>::` for the touched module. Do **not** run the full suite per
  commit. Report the literal `test result:` line — never infer success from completion. Never kill a
  build mid-compile.
- **Formatting.** `rustfmt --edition 2024 <file>` on leaf files you edited only. Never a `mod.rs`,
  never crate-wide.
- **Git.** Commit to `dev` directly. Every commit that changes sim behavior names its gamemd source
  in the message (a cited address or the design doc section), or states the rule is VERA-internal
  with the gamemd equivalent `UNCHECKED`.
- Do not propose style or cleanup edits alongside the substantive work.

---

## S1 — Substrate skeleton *(start here)*

```
Implement slice S1 of the locomotion substrate migration.

READ FIRST, in this order:
1. ENGINE.md and CLAUDE.md in full — the project contract.
2. docs/plans/2026-07-29-locomotion-substrate-design.md sections 1.5, 2.4, 2.6, 6.4, and 8 slice S1.
3. src/sim/substrate/direction_tables/ — this is the established substrate pattern and S1 must
   follow it exactly: const tables, free functions, majority-test files, dependency floor of
   util/ + rules/ only.

SCOPE. Create src/sim/substrate/locomotion/{mod,class,capability,defaults}.rs.
NO consumer changes. Nothing outside the new directory may be edited except adding the module
declaration. This slice is purely additive and must not alter behaviour.

WHAT LANDS.
- `LocomotorClass` — EIGHT live variants only. Mech, DropPod and Tunnel are deliberately ABSENT:
  they are Tiberian Sun legacy with zero live INI references and must never be added.
- The CLSID <-> class table, from design doc section 2.6.
- `piggyback_capable()`.
- The eight live base-default bodies of section 1.5, as pure functions.
- The per-class override map for those eight slots.

ACCEPTANCE TESTS (both goldens are machine-derived, so these are real parity checks — see below).
1. `substrate::locomotion::class::tests::clsid_table_matches_retail_ini`
   Parse ini/rulesmd.ini, strip trailing `;` comments, and assert this exact histogram of live
   `Locomotor=` keys — 155 total: Walk 60, Drive 52, Ship 13, Jumpjet 9, Fly 8, Teleport 6,
   Hover 4, Rocket 3. Also assert the three dormant GUIDs appear ZERO times outside `;` comments.
   NOTE: comment-stripping is load-bearing. Two Drive lines carry a trailing
   `;<-drive mech->{55D141B8-...}` comment; counting raw occurrences yields the wrong histogram.
2. `substrate::locomotion::defaults::tests::base_default_map_matches_vtables`
   Assert the inherit/override pattern for slots 6, 7, 19, 20, 28, 30, 31, 32, 39. Encode the
   expected pattern as data, and cite the `read_memory` addresses from design doc section 2.4 in
   the test comment.

LABELLING. Both goldens are machine-derived — retail INI bytes and the byte-decoded vtable matrix —
so these tests MAY be described as parity checks. Cite the evidence in the test comments.

IF YOU HIT AN UNKNOWN: the design doc marks uncertain facts [U] or UNCHECKED. Do not guess past one.
Stop and report what you needed and could not get.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib substrate::locomotion::
Report the literal `test result:` line.

ROLLBACK: delete the directory. Nothing consumes it yet.

COMMIT: to dev, message citing design doc section 2.6 and 1.5 as the gamemd source.
```

---

## S7 — Delete the dormant-TS modules *(independent, safe, immediate win)*

```
Implement slice S7 of the locomotion substrate migration: remove two Tiberian Sun legacy movement
systems that no stock Yuri's Revenge unit can select.

READ FIRST: ENGINE.md and CLAUDE.md in full; docs/plans/2026-07-29-locomotion-substrate-design.md
section 8 slice S7 and section 3 (the ACTIVE vs INACTIVE table).

WHY THIS IS SAFE. Neither TunnelLocomotion {4A582743-...} nor DropPodLocomotion {4A582745-...}
appears in ini/rulesmd.ini or ini/rules.ini outside comments. The class factories are registered in
the binary but never instantiated in stock YR. `[DRON]` uses Drive, not DropPod.

SCOPE — remove:
- src/sim/movement/tunnel_movement.rs
- src/sim/movement/droppod_movement.rs
- their spine passes (src/sim/world/mod.rs around :2201 and :2223 — verify the current line numbers,
  do not trust these)
- their `Option<XState>` fields (src/sim/game_entity.rs around :354 and :367 — verify)
- their world_hash and snapshot arms
- `LocomotorKind::{Tunnel, DropPod}` IF nothing else references them after the above

CRITICAL — DO NOT TOUCH src/sim/movement/tube_movement.rs. Low-bridge TubeClass movement is ACTIVE
Yuri's Revenge behaviour. It is a recurring project error to conflate it with subterranean/tunnel
locomotion. If you find yourself editing tube_movement.rs, stop — you have the wrong file.

WHAT LANDS: two fewer full-entity-order scans per tick, two fewer dead systems in the tree.

ACCEPTANCE TESTS.
1. `cargo test -p vera20k --lib` green. Report the literal `test result:` line.
2. New: `substrate::locomotion::tests::dormant_clsids_absent_from_ini` — assert {4A582743-...} and
   {4A582745-...} appear zero times in ini/rulesmd.ini and ini/rules.ini.
   (If slice S1 has not landed yet, put this test in a sensible existing module instead and note it.)

LABELLING. The golden is retail INI bytes, so this is a real parity check. Record in the test comment
that ONLY rulesmd.ini and rules.ini were checked — campaign and map INIs are UNCHECKED, so a future
mod reintroducing those GUIDs would correctly turn this test red.

WATCH FOR: removing these may unmask bugs elsewhere that the dead code was masking. Trace every
consumer before deleting. A change that compiles but was never exercised is not done. If snapshot or
world-hash goldens shift, and the tree also carries another session's unmerged shifts, do NOT
re-baseline — record a line in docs/scans/PENDING_REBASELINES.md and leave the test red.

VERIFY: cargo check -p vera20k first, then the full --lib suite (this slice touches the spine, so the
full suite is justified here).

COMMIT: to dev, message citing design doc section 3 and the INI absence as the gamemd source.
```

---

## S6 — Retire the two invented gates *(independent; real player-visible win)*

```
Implement slice S6 of the locomotion substrate migration: delete two VERA-invented movement gates
that gamemd does not have.

READ FIRST: ENGINE.md and CLAUDE.md in full (especially the "Native-to-Rust translation" section —
it cites this exact bug class as a paid-for lesson); docs/plans/2026-07-29-locomotion-substrate-design.md
section 8 slice S6, and items R1 and R3 in section 7.

BACKGROUND. A 2026-07-28 trace swarm established at instruction level that gamemd has NO movement
kill gate, and that path nodes are direction octants plus a sentinel-8 tube entry. Our tree invented
two gates that have no native counterpart:
- R1: sharp-turn fallback silently DROPS a path node (src/sim/movement/movement_step.rs ~:133-153)
- R3: tube-step kill gate DESTROYS move orders (src/sim/movement/tube_movement.rs ~:177-235 plus
  src/sim/movement/movement_tick.rs ~:1144-1147)
Verify the current line numbers — do not trust these.

SCOPE. Replace both with the substrate's path-node contract: direction octants plus the sentinel-8
tube entry, so that non-adjacency is UNREPRESENTABLE and no abort path exists.

WHAT LANDS.
- The `cur_dir x 9` straight-track-then-rotate-in-place behaviour, with the path queue UNMODIFIED.
- Deletion of the `Blocked -> finished_entities` kill.

ACCEPTANCE TESTS.
1. `movement::locomotion::process::tests::sharp_turn_preserves_path_node_count` — a Drive unit takes
   a turn of at least 135 degrees; the queue length before and after differs by exactly the nodes
   consumed, never one extra.
2. A low-bridge scenario asserting the unit never lands in `finished_entities` without reaching its
   goal.

LABELLING. Test 1's node-count assertion is derived from the native contract (the trace swarm's
instruction-level finding), not from prior Rust output — cite the scan directory in the test comment
and it may be described as a parity check. Test 2 is a RATCHET; label it UNCHECKED.

THE MOST IMPORTANT INSTRUCTION IN THIS PROMPT. If removing these gates makes a unit stall, that is a
FINDING about the substrate path-node contract — report it. It is NOT a reason to re-add the gate,
and it is NOT a reason to add a new guard that suppresses the symptom. The question is never "what
check would stop this happening"; it is "what does gamemd do here". This exact gate was patched twice
across a full night before a binary trace showed gamemd has no such gate at all.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib movement::
Expect golden shifts — this changes behaviour by design. If the tree carries another session's
unmerged shifts, record a line in docs/scans/PENDING_REBASELINES.md and leave the test red.

ROLLBACK: both gates are small and localized; two-hunk revert.

COMMIT: to dev, message citing the 2026-07-28 trace swarm as the gamemd source.
```

---

## S2 — `Is_Moving_Now` producers *(needs S1)*

```
Implement slice S2 of the locomotion substrate migration.

READ FIRST: ENGINE.md and CLAUDE.md in full; docs/plans/2026-07-29-locomotion-substrate-design.md
section 8 slice S2 and item R2 in section 7.

THE PROBLEM. src/sim/movement/locomotor_ready.rs contains an exact, exhaustively-tested evaluator for
the `Is_Moving_Now` predicate — and it has NO PRODUCER. The production mission gate therefore always
answers "not moving". A correct evaluator wired to nothing.

SCOPE — note the two symbols live in a DIFFERENT file than the design doc implies; verified
2026-07-29:
- Move src/sim/movement/locomotor_ready.rs to src/sim/substrate/locomotion/ready.rs UNCHANGED.
  (Requires slice S1 to have created that directory.) The file is 303 lines; `mod tests` starts at
  line 103, so the truth tables are roughly :103-:303, NOT the :148-:302 the design doc states.
- Wire real per-class inputs from the existing `LocomotorState` fields.
- Delete `DEGRADED_NOT_MOVING` and the `degraded_moving_gate` parameter. These are NOT in
  locomotor_ready.rs — they are in src/sim/mission/authority.rs (const at :201, parameter at :214,
  the `.or(...)` fallback that forces the gate at :226-:227). That `.or()` is the thing making the
  mission gate answer "not moving" unconditionally; it is the actual target of this slice.

WHAT LANDS: a production producer for `mission_ready_state`, so the mission gate stops answering "no"
unconditionally.

ACCEPTANCE TESTS.
1. The existing full-input-space truth-table tests move with the file and must stay green. These ARE
   a real proof of the predicate — they are exhaustive over the input space. Note the file classifies
   f64 BIT PATTERNS (sign/exponent/fraction masks, subnormals, both NaN kinds) rather than doing
   float arithmetic — that is deliberate and deterministic. Do not "clean it up" into float compares.
2. New: `mission::readiness::tests::moving_unit_is_not_ready_to_commence` — a production-path test
   asserting a unit with a live `MovementTarget` reports moving.
3. Live-observe smoke: RA2_QUICKPLAY=minerloop.map, confirm the miner still docks. Note that live
   observe smokes need roughly 200 seconds of machine idle (ForegroundLockTimeout).

LABELLING — READ CAREFULLY. The truth tables are gamemd-derived and exhaustive, so the PREDICATE
claim may say VERIFIED. The PRODUCER mapping — which Rust field feeds which native input — is
UNCHECKED until each input is traced to its native field. Put exactly that distinction in the module
doc. Do not let the predicate's strength launder the producer's weakness.

RISK. This changes a gate that currently always answers one way, so it WILL move behaviour. Expect a
golden shift. If the tree carries another session's unmerged shifts, do NOT re-baseline — record a
line in docs/scans/PENDING_REBASELINES.md and leave the test red.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib substrate::locomotion::ready:: and
cargo test -p vera20k --lib mission::

ROLLBACK: restore the constant. Single-commit revert.

COMMIT: to dev, citing the design doc section 8 S2 and the truth-table provenance.
```

---

## S3 — `LocomotorSlot`, install, and the host position surface *(needs S1)*

```
Implement slice S3 of the locomotion substrate migration.

READ FIRST: ENGINE.md and CLAUDE.md in full; docs/plans/2026-07-29-locomotion-substrate-design.md
sections 5.1, 5.6.1, 5.7, 6.4, and section 8 slice S3 IN FULL. S3's spec was corrected twice by
follow-up investigation — read the current text, not any summary of it.

SCOPE. src/sim/movement/locomotion/{slot,instance,install}.rs. Introduce `LocomotorSlot` as the
single authority for "which locomotor is installed", superseding the selection role of
`LocomotorState.kind` and the twelve sibling `Option<XState>` fields. Add the slot to world_hash and
to the snapshot.

WHY THE HASH MATTERS: today the field that decides which locomotor runs is NOT in world_hash, so a
lockstep divergence there is invisible. Fixing that is a core purpose of this slice.

WHAT LANDS — install at spawn, which is simpler than earlier drafts assumed.
One CLSID per type, straight from `Locomotor=`, constructed once at spawn, linked to the host. No
stock YR unit is constructed with one locomotor and then permanently swapped. The Rust equivalent is:
resolve the kind from the type at spawn, store it, done.
- Parse-or-default. THE DEFAULT IS TELEPORT, NOT DRIVE (verified at 0x00710C21).
- Silent fallback on an unparseable CLSID.
- create -> link -> store -> drop-old, guarded by `new != old`.

ACCEPTANCE TESTS.
1. `movement::locomotion::install::tests::locomotor_beam_stashes_and_installs_jumpjet`
   From ini/rulesmd.ini: `[LocomotorBeam] IsLocomotor=yes Locomotor={92612C46-...}`, reached from
   `[TELE]` Magnetron via Primary=MagneticBeam -> Warhead=LocomotorBeam.
   Assert the victim's EFFECTIVE class becomes Jumpjet AND THE PREVIOUS LOCOMOTOR IS STASHED, NOT
   DROPPED. The Magnetron performs a full Begin_Piggyback at 0x007102D8, in canonical
   B4-before-B5 order. An earlier draft of this document called it a raw replacement; that was
   REFUTED, and a test asserting a drop would encode the opposite of the real contract.
   Also assert the victim is a `What_Am_I() in {1,2}` host — infantry and buildings can never be
   lifted, by code.
2. `install::tests::missing_locomotor_key_defaults_to_teleport`
3. `install::tests::unparseable_clsid_falls_back_silently`
4. `install::tests::six_stock_sections_resolve_to_teleport` — [CLEG], [CCOMAND], [CIVAN], [CMIN],
   [CMON], [SMON].

RAW REPLACE: S3 has lost its live example. After investigation, no stock-YR trigger is a raw swap
except Carryall pickup, whose reachability rests on an unverified `[HIND] TechLevel=-1`. EITHER drop
raw-replace from scope, OR land it labelled VERA-internal with the stock trigger recorded as
UNCHECKED — not as "none".

SAVE/LOAD: implement the section 5.7 mechanism (the snapshot SHAPE is validated: discriminant + full
runtime state + nested stash, host-then-stash order; section 5.7 also rules three things out of the
snapshot). But write UNVERIFIED in the module doc — a byte comparison against a gamemd save is
impossible, and semantic correspondence argued in prose does not upgrade a status.

THE HOST POSITION SURFACE — four constraints, ALL [UNCHECKED] as parity, all from section 5.6.1:
1. TWO commit entry points, not one: a full-coordinate commit and a height-only commit. A single
   XY-style setter has nowhere to put the three Z-only committers. Hover (live on [ROBO] Robot Tank,
   stock buildable) uses each within a single Move.
2. The occupancy bracket belongs INSIDE the commit, conditioned on the placed flag, on BOTH entry
   points. Earlier guidance to keep commit and occupancy separate was written on an unchecked gap and
   MUST NOT be carried forward.
3. The commit is host-owned, not a public field: compare against the current triple, bracket
   conditionally, store all three components, cascade to attached followers ONLY when the coordinate
   changed. No locomotor module may write the entity coordinate directly.
4. The position field sits at the GENERIC-ENTITY level, not on a foot/unit struct — anims, bullets,
   particles and buildings share it.
Also: distinguish intra-cell from cell-crossing AT COMMIT TIME, and make the redraw a dirty flag the
render layer polls — never a call outward from sim/.

LABELLING. The install goldens are retail INI bytes plus a verified constructor default — parity-grade,
non-exhaustive. The position-surface work is UNCHECKED: `emulate_function` provably cannot witness a
store-only function's result, so a Rust `set_position` test ships as a well-provenanced ratchet citing
0x004DB810, 0x005F6940, 0x004D3780 and 0x005F6060.

SHADOW WINDOW: `LocomotorSlot` may coexist with `LocomotorState.kind` for AT MOST ONE SESSION while
consumers migrate. It flips authoritative next session or the slice is reverted. Do not park it.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib movement::locomotion:: and
cargo test -p vera20k --lib sim::world::world_hash::

COMMIT: to dev, citing 0x00710C21, 0x007102D8 and design doc section 5.1.
```

---

## S4 — One piggyback mechanism *(needs S3; the highest-value slice)*

```
Implement slice S4 of the locomotion substrate migration.

READ FIRST: ENGINE.md and CLAUDE.md in full; docs/plans/2026-07-29-locomotion-substrate-design.md
sections 5.3, 5.4, and section 8 slice S4 IN FULL. This slice's spec grew after two follow-up
investigation waves — read the current text.

THE PROBLEM. The tree has TWO rival piggyback mechanisms that do not know about each other:
`PiggybackLocomotor` and `OverrideLocomotor`. Both select which locomotor runs; NEITHER is in
world_hash; `end_override` silently drops ten fields. This is a live lockstep-divergence risk.

SCOPE. src/sim/movement/locomotion/piggyback.rs. Delete `PiggybackLocomotor` and `OverrideLocomotor`
and their two rival APIs. Delete the dead `OverrideKind::Parachute`. Rewire the current callers
(movement_commands.rs ~:184/:208, miner/miner_system.rs ~:1572, teleport_movement.rs ~:172/:365,
droppod_movement.rs ~:78/:198, world_commands.rs ~:314-315 — verify current line numbers; note
droppod may already be gone if slice S7 landed).

FREQUENCY CONTEXT — this is not an edge case. A Chrono Miner is permanently a Teleport locomotor that
PIGGYBACKS A DRIVE to do its ordinary ground movement. It crosses the swap on every factory exit and
every move order. This is routine Allied economy, not a Chronosphere curiosity.

SCOPE GREW BY TWO LIVE MECHANISMS: the ChronoWarp superweapon BEGIN (section 5.4 row 5) and the
Magnetron warhead BEGIN/END (rows 6/6b) — the latter writes the +0x2AC/+0x2B0 mutual link and
host[+0x6AD], which the whole Is_Ok_To_End gate table depends on.
DO NOT scope this slice on "infantry contribute zero piggyback traffic" — that claim was REFUTED;
the infantry path is UNRESOLVED-REACHABILITY, not dead.

WHAT LANDS — the ordered BEGIN and END protocols of section 5.3:
- non-nesting enforced by type
- `Is_Ok_To_End` dominated by `!is_moving()`
- the observable null window
- the popped locomotor destroyed AFTER transport-entry processing
- ownership transferred on `end` with no AddRef/Release analogue
- `effective_class()` implementing see-through identity, so harvest/scatter/set-destination ask
  through it. Independently motivated: `UnitClass::Set_Destination` branches on the ACTIVE
  locomotor's class while mission logic branches on the see-through identity, so both must be
  distinguishable.

FOUR GATE-MODEL REQUIREMENTS, all non-optional:
1. PER-KIND gate state, NOT a shared `in_critical_section: bool`. Teleport's gate is two fields, and
   obj[+0x38] is an EIGHT-VALUED PHASE INDEX. A single-bool model makes Teleport's gate constant-true
   and lets the piggyback unwind mid-warp — exactly what the gate exists to prevent. Teleport carries
   a phase enum.
2. Model the phases-3-to-7 window. host[+0x27C] alone is NOT the warp gate; phase 2 clears it while
   phases 3-7 still run.
3. Jumpjet's host clause is an OR: `(host[+0x6AD] == 0 || host[+0x6AE] != 0)`. Omitting it means a
   lifted unit can never be released.
4. Clear +0x428/+0x42C on ALL THREE paths — End_Piggyback AND warp completion. Clearing in one place
   leaves a stale HouseClass credit that a later failed warp uses.

BOTH BEGIN SHAPES MUST EXIST: abandon-on-E_FAIL, and END-FIRST (0x00742608, 0x0044DFE0), which two of
the four live BEGIN sites use. `begin_refuses_to_nest` is load-bearing, not decorative.

ACCEPTANCE TESTS.
1. `piggyback::tests::end_order_matches_native` — the exact E5->E11 sequence via an ordered event log:
   current dropped BEFORE the slot is cleared; slot observably empty before restore; restore before
   the +0x6B4 equivalent; destroy AFTER transport entry.
2. `piggyback::tests::begin_refuses_to_nest`
3. `piggyback::tests::begin_ends_first_on_the_two_end_first_sites`
4. `piggyback::tests::teleport_gate_holds_through_phase_7`
5. `piggyback::tests::effective_class_sees_through_stash` — a Chrono Miner with Drive piggybacked
   still reports Teleport.
6. Live-observe run: a Chrono Miner exits a war factory, drives out, and pops the Drive.

LABELLING — IMPORTANT. `end_order_matches_native` is a HAND-TRANSCRIBED golden with a prose citation.
Ship it as a WELL-PROVENANCED RATCHET labelled UNCHECKED, not VERIFIED. Cite 0x004DAE5F-0x004DAF07,
0x0044E10E, 0x0065F174, 0x006CC98C and 0x007102D8 in the test comment. The route to a VERIFIED
ordering claim is `emulate_function` over 0x007192F0 / 0x004DAE5F with a recorded machine-derived
call trace — not more prose. A world-hash comparison against previous Rust is a ratchet and must not
be called parity.

POSITION IS NOT PIGGYBACK-SENSITIVE. Position commit and occupancy are host-side, so an inner stashed
locomotor commits through the same host slots with no handoff. Consequences: a Rust
`PiggybackLocomotor` that owned or cached a position would be DRIFT, and the observable null window is
safe with respect to position — nothing in BEGIN/END needs to save or restore the host coordinate.
ONE CAVEAT [UNCHECKED]: the commit's occupancy gate reads the INSTALLED locomotor's layer, so a model
that caches the layer rather than resolving it from the currently-installed locomotor would diverge
exactly during a piggyback.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib movement::locomotion::piggyback::
and cargo test -p vera20k --lib miner::

ROLLBACK: single revert — delete both old mechanisms in the same commit that adds the new one, so
there is no half-migrated state.

COMMIT: to dev, citing section 5.3 and the addresses above.
```

---

## S5 — Power *(needs S3)*

```
Implement slice S5 of the locomotion substrate migration.

READ FIRST: ENGINE.md and CLAUDE.md in full; docs/plans/2026-07-29-locomotion-substrate-design.md
section 8 slice S5 and item R4 in section 7.

SCOPE. `LocomotorSlot.powered`, default true; `power_on()` / `power_off()` / `is_powered()`; the
Hover-only observable effect; wiring the verified edges.

WHAT LANDS: deploy-begin -> off; undeploy-complete -> on; undock and release-docked-harvester -> on;
per-cell-process -> off; and set-destination on an unpowered unit -> on (the player-facing recovery
edge). `hover_vertical_tick`'s `powered` parameter stops being a hard-coded `true`.

EXPLICITLY DOES NOT LAND — do not implement any of these, they are dead or unverified in stock YR:
- EMP-drives-power (the EMPulseClass constructor is unreferenced)
- Fly's Power_Off RNG draws (unreachable; porting them would ADD RNG consumption gamemd never does,
  which would break lockstep determinism)
- any Is_Ion_Sensitive, IonStorms or [SpecialFlags] ion path (Tiberian Sun legacy — note that YR's
  Lightning Storm is a DIFFERENT mechanism and is not coupled to locomotor power)
- the Power_On/Power_Off -> Is_Powered re-dispatch (a structural artefact with no verified effect)

ACCEPTANCE TESTS.
- `movement::locomotion::power::tests::move_order_repowers_locomotor`
- `movement::locomotion::power::tests::hover_unpowered_sinks`

LABELLING. The EDGES are VERIFIED from callsites with receivers confirmed. HOW OFTEN a stock skirmish
actually reaches the powered-off state is UNCHECKED — with EMP dead, the surviving producers are
deploy-begin and per-cell-process, and neither was traced to a frequency. Put that sentence verbatim
in the module doc.

VERIFY: cargo check -p vera20k, then cargo test -p vera20k --lib movement::locomotion::power::

ROLLBACK: default the flag to true and stop writing it — behaviourally identical to today.

COMMIT: to dev, citing design doc section 8 S5.
```

---

## S8 — The render split *(independent; defer freely)*

```
Implement slice S8 of the locomotion substrate migration: move render-only locomotor concerns out of
sim/.

READ FIRST: ENGINE.md and CLAUDE.md in full (the #1 invariant: sim/ must never depend on render/,
ui/, sidebar/, audio/, net/); docs/plans/2026-07-29-locomotion-substrate-design.md section 6.3 and
section 8 slice S8.

SCOPE.
- Move slots 8, 9, 10, 11, 12, 14, 15 and 34 out of any sim-held type into
  render::locomotor_visual, reading sim state read-only.
- Split slot 21: tilt STATE stays in sim/ and is snapshotted; the matrix BUILD moves to render/.
- Move screen_x/screen_y writes and ALTITUDE_VISUAL_SCALE out of sim/.
- Remove the two f32 fields from `LocomotorState`.

ACCEPTANCE TESTS.
- A compile-time or CI guard asserting sim/ contains no `screen_` writes and no f32/f64 in locomotion
  state.
- The existing render goldens stay green.

LABELLING. This slice's boundary rests on what these slots WRITE — a matrix plus a render cache key,
no sim state. It does NOT rest on the wall-clock argument, which was refuted. Whether
`linked_object + 0x388` holds ticks or wall-clock is UNCHECKED; decode it before claiming anything
stronger than "these slots write no sim state".

VERIFY: cargo check -p vera20k, then the full cargo test -p vera20k --lib (this touches types shared
with render). Report the literal `test result:` line.

ROLLBACK: mechanical revert.

COMMIT: to dev, citing design doc section 6.3.
```

---

## What none of these slices do

Recorded so it is not mistaken for an omission:

- **None produces a VERIFIED parity claim for the piggyback END ordering or the position commit.**
  Both ship as well-provenanced ratchets. The blocker is instrumental, not analytical: `emulate_function`
  returns registers only, so a store-only function's effect is unobservable, and there is currently no
  known instrument for a parity check on the committer. More investigation will not close this.
- **The infantry chrono-move mechanism is UNKNOWN**, not merely undocumented. Do not port anything
  there from any lane report.
- **S3's raw-replace path has no verified live stock trigger.** Land it VERA-internal or drop it.
