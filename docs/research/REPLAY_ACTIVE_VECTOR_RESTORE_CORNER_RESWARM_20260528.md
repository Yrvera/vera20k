# Replay Active Vector Restore Corner - Reswarm 2026-05-28

**Address(es):** `Main_Game @ 0x0052D9A0`, replay open/read helpers `0x00473C50` / `0x00473D10` / `0x00473B10` / `0x00473AE0`, `Main_Tick @ 0x0055D360`, pre-object-pass order helper `0x00551A30`, `ScenarioClass::Read_Scenario @ 0x00684620`, `ScenarioClass::Full_Init @ 0x00686B20`, savegame contrast path `FUN_0067E440` / `FUN_0067E730` / `FUN_00551B90`.
**Investigation Mode:** exhaustive-slice for replay startup/playback active-list restore corner cases.
**Claimed Scope:** replay-recording startup and per-frame playback paths that could restore, rebuild, or repair the `LogicClass` active-object vector or `ObjectClass+0x98`, distinct from standard savegame load.
**Non-Scope:** normal savegame vector serialization/load, standard post-load `Object+0x98` byte sampling, ordinary map-load reveal order, complete replay input/event semantics, and full meanings of every replay stream helper.
**Confidence:** High for replay startup not using the savegame vector-load path and for per-frame replay not reconstructing active-list membership; Medium for final `Object+0x98` byte parity in all replayed runtime cases because runtime watchpoints were not taken.
**Active in YR:** Yes, conditional on replay flags in `DAT_00A8D5F8`. Replay playback is flag-driven (`& 2`), not `g_GameMode == 5`; `g_GameMode == 5` is the standard Skirmish branch in the verified `Main_Game`, `ScenarioClass`, and `Main_Tick` bodies.

## 0. Investigation Contract

**Target question:** Does replay restore/playback have a separate active-list reconstruction path for `LogicClass+0x04/+0x10` or `ObjectClass+0x98`, or does replay startup rely on normal scenario initialization and per-frame replay input/sync playback?

**Non-goals:** Do not redo savegame `FUN_0067E440`/`FUN_0067E730` vector-load work; do not re-prove ordinary `ObjectClass::Reveal -> FUN_0055BAA0`; do not implement Rust; do not investigate every replay packet/event type.

**Evidence needed to mark COMPLETE:** decompile plus disassembly-range proof for replay mode flags and startup path, decompile plus binary context for scenario launch path, decompile plus Main_Tick evidence for per-frame playback, and contrast evidence that savegame vector load is a separate path.

**Stop conditions:** Stop once replay startup/playback is proven to either call a vector restore path or not call one, and once remaining uncertainty is limited to runtime-only byte sampling or unrelated replay-event decoding.

## 1. Overview

Replay playback does not load a native savegame snapshot and does not call the savegame active-vector loader. `Main_Game` opens the replay recording, reads a small header into seed/scenario/session globals, then jumps to the normal scenario launch path. The active vector is therefore seeded by normal scenario/map initialization and ordinary reveal/register paths, with later runtime mutations coming from normal object lifecycle behavior.

Per-frame replay playback in `Main_Tick` reads sync/selection/cursor records and renders, then still reaches the existing pre-object-pass vector order helper and `LogicClass::PerTickUpdate`. It does not call `FUN_00551B90`, `FUN_0055BAA0`, or the savegame swizzle wrapper as a replay-specific restore/rebuild step.

## 2. Key Fields / Flags

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8D5F8 & 1` | replay recording write path | `Main_Tick` write block; `RNG_SYSTEM_GHIDRA_REPORT.md` | Yes, conditional |
| `DAT_00A8D5F8 & 2` | replay playback read path | `Main_Game @ 0x0052D9A0`; `Main_Tick @ 0x0055D360`; disassembly `0x0052DC9F..0x0052DD29`, `0x0055D940..0x0055DBAF` | Yes, conditional |
| `DAT_00A8D5F8 & 4` | post-game replay-available gate before setting playback bit | `Main_Game` case `-1` | Yes, conditional |
| `g_GameMode == 5` | Skirmish branch, not replay mode | `Main_Game` case `0x10` calls `FUN_006AE2C0`; `ScenarioClass::Full_Init` and `Main_Tick` both pair `0/5` as single-player/skirmish-style branches | Yes for Skirmish |
| `LogicClass+0x04/+0x10` | active object vector data/count | prior save/load reports; `FUN_00551B90` contrast | Yes |
| `ObjectClass+0x98` | active-list membership byte | helper/remover reports; no replay-specific writer found in scoped replay bodies | Yes for normal lifecycle; no replay restore writer found |

## 3. Verified Replay Findings

### 3.1 Replay Startup Reads Header, Then Launches Scenario

In `Main_Game @ 0x0052D9A0`, two replay startup entries exist:

- at function entry, if `DAT_00A8D5F8 & 2` is already set and `FUN_00473C50(0)` succeeds;
- in switch case `-1`, if `DAT_00A8D5F8 & 4` is set, `FUN_00473C50(0)` succeeds, and the code then sets `DAT_00A8D5F8 |= 2`.

Both entries call `FUN_00473D10(1)`, read only these startup fields from the recording stream, log `"Loaded recording values for scenario : %s"`, stop music, and jump to the same scenario launch label:

1. 4-byte magic/version at `0x00822CF4`;
2. 4-byte seed at `0x00A8ED94`;
3. 4 bytes at `ScenarioClass+0x1254`;
4. 0x104-byte scenario filename/string at `ScenarioClass+0x125C`;
5. 4 bytes at `0x00A8EC90`;
6. 4 bytes at `0x00A8E960`;
7. 0xB8 bytes at `0x00A8EB60`.

Evidence: `Main_Game` decompile; replay-entry disassembly range `0x0052DC9F..0x0052DD29`; scenario-launch disassembly range `0x0052E356..0x0052E660`. Active in YR: Yes, conditional replay playback.

No object table, `LogicClass` vector, swizzle queue, or `Object+0x98` byte is loaded in this replay header block.

### 3.2 Replay Scenario Launch Uses Normal Scenario Initialization

After the replay header, `Main_Game` reaches `FUN_0054F720`, `FUN_0052FC20`, `FUN_006370B0`, then calls `ScenarioClass::Start_Scenario`. When replay playback bit `& 2` is set, the scenario call uses `-1`, the same non-campaign/default launch selector used outside the campaign-specific `g_GameMode == 0 && !replay` branch.

`ScenarioClass::Read_Scenario @ 0x00684620` reads the scenario file named in `ScenarioClass+0x125C`, then `ScenarioClass::Full_Init @ 0x00686B20` performs the normal map/object initialization path. The verified `Full_Init` body reaches the known object section readers and normal reveal paths. Evidence: decompiles of `0x0052D9A0`, `0x00684620`, `0x00686B20`; map-loader disassembly range `0x006876F0..0x00687B1F`. Active in YR: Yes, replay playback starts a normal scenario from the recorded scenario filename.

This is not savegame restore: it does not open COM structured storage, does not open a `CONTENTS` stream, and does not call `FUN_0067E440` or `FUN_0067E730`.

### 3.3 Savegame Vector Load Is a Separate Path

The savegame load wrapper `FUN_0067E440` opens `"LOADING GAME [%s]"` storage and calls `FUN_0067E730`; `FUN_0067E730` calls `FUN_00551B90` to load the `LogicClass` vector. That path is not called by the replay startup branches in `Main_Game`.

Evidence: `FUN_0067E440` decompile; `FUN_0067E730` decompile; `FUN_00551B90` decompile; prior `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md` and `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`. Active in YR: Yes for standard savegame load; not active for replay startup.

### 3.4 Per-Frame Replay Playback Does Not Reconstruct Membership

In `Main_Tick @ 0x0055D360`, replay playback bit `& 2` skips the normal input/AI/map/render block, then the replay read block:

- reads 8-byte state hash and validates it through `FUN_006D6000`;
- reads selected object count;
- recomputes a current selected-object sum;
- reads expected sum and clears selection on mismatch;
- reselects objects by replayed type/id tokens;
- reads two cursor/mouse globals;
- calls display refresh/render.

After that, execution reaches `FUN_00551A30` and then `LogicClass::PerTickUpdate`. The replay read block does not call `FUN_00551B90`, `FUN_0055BAA0`, or `FUN_0055BAE0`, and it does not write `Object+0x98`.

Evidence: `Main_Tick` decompile; disassembly ranges `0x0055D860..0x0055D930`, `0x0055D940..0x0055DBAF`, and `0x0055DBB0..0x0055DCA7`. Active in YR: Yes, conditional replay playback.

### 3.5 Existing Per-Tick Order Helper Still Runs During Replay

`FUN_00551A30` is called after replay record/playback bookkeeping and before `LogicClass::PerTickUpdate`. Its decompile shows a single adjacent pass over `LogicClass+0x04` entries: compare `items[i+1].vtable+0xB8()` against `items[i].vtable+0xB8()`, swap the adjacent pair if the later key is smaller, then advance one index. It does not set or clear `Object+0x98`, and it is not replay-specific.

Evidence: `FUN_00551A30` decompile; disassembly `0x00551A30..0x00551A84`; caller placement in `Main_Tick`. Active in YR: Yes, including replay playback because it is after the replay block and before the object pass. (corrected 2026-05-29: disassembly end was `0x00551A8E`; binary shows final `RET` at `0x00551A84` and function body `0x00551a30 - 0x00551a84` via `disassemble_function 0x00551A30` — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

This is a scoped order-maintenance fact, not a restore/reconstruction path. The exact virtual key semantics of `vtable+0xB8` are a separate active-order follow-up if the parent wants to revise older "tail append only" wording globally.

## 4. Current Rust Implementation Status

| Rust surface | Current shape | Replay delta |
|---|---|---|
| `src/sim/replay.rs` | JSON replay log stores header metadata plus per-tick commands and hashes; `ReplayRunner` replays commands into an existing `Simulation`. | No native replay file/header model; no native scenario re-launch from replay header; no special active-vector restore, which matches the negative binary finding. |
| `src/sim/snapshot.rs` | Bincode serializes full `Simulation` for mid-match save/load. | Save/load snapshot is distinct from replay, but Rust can blur the two if replay playback is initialized from a snapshot. Native replay playback is scenario-header based, not savegame-vector based. |
| `src/sim/world/mod.rs:289` | `live_object_order` is serialized by default. | Directionally useful for Rust snapshots, but not a native replay file field. |
| `src/sim/world/mod.rs:612..619` | register/unregister use vector contains/retain, not object-local byte. | No `Object+0x98` equivalent; replay does not provide a separate byte repair path. |
| `src/sim/world/mod.rs:622..638` | `live_object_order_snapshot` appends sorted missing `EntityStore` IDs. | Native replay startup/playback has no sorted missing-ID repair. This fallback must not be cited as replay parity. |
| `src/app_sim_tick.rs:255..263`, `:672..674` | App auto-creates a replay log for an existing sim and records tick hashes. | Current replay recording is a Rust determinism aid, not native `.YRO`/recording parity. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Game` replay-entry bit `& 2` | verified | decompile `0x0052D9A0`; disassembly `0x0052DC9F..0x0052DD29` | none |
| `Main_Game` case `-1`, bit `& 4 -> |= 2` | verified | decompile `0x0052D9A0` | exact UI/source that sets bit `& 4` outside this slice |
| Replay header field reads | verified | `Main_Game` decompile; `RNG_SYSTEM_GHIDRA_REPORT.md` | full replay packet format outside startup header |
| Replay scenario launch | verified | `Main_Game`, `ScenarioClass::Read_Scenario`, `ScenarioClass::Full_Init` | none for active-vector restore claim |
| Savegame vector-load contrast | verified | `FUN_0067E440`, `FUN_0067E730`, `FUN_00551B90`; prior save/load reports | none for replay distinction |
| Main_Tick replay playback block | verified | `Main_Tick` decompile; disassembly `0x0055D940..0x0055DBAF` | full event/command virtualization outside scope |
| Replay-specific active vector reconstruction | verified negative | no call to `FUN_00551B90`/`FUN_0055BAA0` in replay startup or per-frame replay blocks | runtime watchpoint can further confirm no indirect byte writes in unusual replay events |
| `FUN_00551A30` pre-object-pass helper | touched-not-exhausted | decompile/disassembly `0x00551A30..0x00551A84` | exact `vtable+0xB8` key semantics | (corrected 2026-05-29: end was `0x00551A8E`; actual `0x00551A84`)
| Per-class final `Object+0x98` byte during replay | deferred | static replay path has no special writer | runtime debugger/watchpoints if needed |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-RAVR-001 - What identifies replay playback? -> `DAT_00A8D5F8 & 2`, set directly or from case `-1` after `& 4`; not `g_GameMode == 5`.` (evidence: `Main_Game @ 0x0052D9A0`)
- `[RESOLVED] OQ-RAVR-002 - Is `g_GameMode == 5` replay? -> No; verified bodies use it for Skirmish setup/start branches.` (evidence: `Main_Game`, `ScenarioClass::Read_Scenario`, `Main_Tick`)
- `[RESOLVED] OQ-RAVR-003 - Does replay startup call savegame load wrapper `FUN_0067E440`? -> No; it calls replay helpers and jumps to scenario launch.` (evidence: `Main_Game`; contrast `FUN_0067E440`)
- `[RESOLVED] OQ-RAVR-004 - Does replay startup call active-vector load `FUN_00551B90`? -> No; the header block reads seed/scenario/session fields only.` (evidence: `Main_Game`; `FUN_00551B90` contrast)
- `[RESOLVED] OQ-RAVR-005 - How is replay scenario state created? -> From normal scenario read/full init using recorded scenario filename and seed/session globals.` (evidence: `Main_Game`, `0x00684620`, `0x00686B20`)
- `[RESOLVED] OQ-RAVR-006 - Does per-frame playback rebuild active list? -> No; it reads sync/selection/cursor data and renders, then continues to normal late helpers.` (evidence: `Main_Tick @ 0x0055D360`)
- `[RESOLVED] OQ-RAVR-007 - Does replay playback write `Object+0x98` directly? -> No direct write found in scoped replay startup/playback bodies; normal lifecycle writers remain ordinary reveal/register/removal paths.` (evidence: `Main_Game`, `Main_Tick`, helper reports)
- `[RESOLVED] OQ-RAVR-008 - Does any active-list order mutation still run in replay playback? -> Yes; `FUN_00551A30` runs before `PerTickUpdate` and can swap adjacent active-vector entries by virtual key.` (evidence: `0x00551A30`, `Main_Tick`)
- `[RESOLVED] OQ-RAVR-009 - Does replay use savegame swizzle fixup? -> Not for replay startup. Normal scenario full init calls generic swizzle fixup as part of map init, but not the savegame vector-load path.` (evidence: `0x00686B20`; `0x0067E440`)
- `[RESOLVED] OQ-RAVR-010 - Does current Rust have a native replay restore model? -> No; replay is JSON commands/hash over an existing `Simulation`, and snapshots are separate bincode saves.` (evidence: `src/sim/replay.rs`, `src/sim/snapshot.rs`)
- `[DEFERRED] OQ-RAVR-011 - Which UI/file path sets `DAT_00A8D5F8 & 4` before case `-1`?` (category: `out-of-scope`; reason: not needed to prove active-vector restore absence once `Main_Game` case is reached; next-step-if-pursued: trace writers to `DAT_00A8D5F8`)
- `[DEFERRED] OQ-RAVR-012 - What exact key does `vtable+0xB8` return for every object class in `FUN_00551A30`?` (category: `requires-different-system-context`; reason: this report only distinguishes replay restore from per-tick order maintenance; next-step-if-pursued: investigate `FUN_00551A30` ordering helper as its own active-order target)
- `[DEFERRED] OQ-RAVR-013 - What is the final runtime `Object+0x98` byte for every object class during replay playback?` (category: `needs-runtime-debugger`; reason: static replay paths have no special writer, but exhaustive per-class byte sampling requires watchpoints)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native replay playback starts from a replay header and normal scenario initialization, not from savegame active-vector restore. Active in YR: Yes, conditional replay playback. | `Main_Game @ 0x0052D9A0`; `ScenarioClass::Read_Scenario @ 0x00684620`; `ScenarioClass::Full_Init @ 0x00686B20` | Rust replay runner consumes an already-built `Simulation`; no native replay startup model. | `src/sim/replay.rs`, app replay-load surface, map/scenario launch code | If native replay parity is implemented, initialize a fresh scenario from replay header metadata and normal map-load order, then feed recorded ticks; do not deserialize a mid-match snapshot as the native replay start. | Load a replay header for a map where native scenario order differs from Rust stable ID order; first active-order consumer sees normal map-load/reveal order, not saved snapshot order. Proposed test: `native_replay_start_rebuilds_from_scenario_header_not_snapshot`. | Do not reuse savegame vector-load or Rust snapshot restore as the native replay startup mechanism. |
| Replay per-frame playback reads sync/selection/cursor records and does not repair/rebuild active-list membership. Active in YR: Yes, conditional replay playback. | `Main_Tick @ 0x0055D360`; disassembly `0x0055D940..0x0055DBAF` | Rust replay only applies commands and checks hashes, but `live_object_order_snapshot` can silently append sorted missing IDs. | `src/sim/replay.rs::ReplayRunner`, `src/sim/world/mod.rs::live_object_order_snapshot` | Replay playback should expose missing active-order state as a deterministic mismatch/loader error, not silently sorted repair. | Replay a fixture with an entity present in storage but absent from active order; a parity-mode replay check must fail or preserve absence rather than append it sorted. Proposed test: `replay_playback_does_not_sorted_repair_missing_live_order_member`. | Do not use deterministic sorted fallback as a replay parity claim. |
| `FUN_00551A30` still runs before `LogicClass::PerTickUpdate` during replay playback, but it is ordinary per-tick order maintenance, not restore. Active in YR: Yes. | `Main_Tick` caller placement; `FUN_00551A30 @ 0x00551A30..0x00551A8E` | Rust has no equivalent one-pass adjacent active-vector ordering helper. | future active object scheduler/order service | Keep replay restore and per-tick active-order maintenance separate in design; a future exact scheduler should model the helper based on verified key semantics. | Given three active objects with known native `vtable+0xB8` keys, one native tick performs one adjacent pass before object AI, including during replay playback. Proposed test: `replay_tick_runs_active_order_adjacent_pass_before_object_ai`. | Do not implement replay startup as a sort pass; this helper is per-tick and its key semantics are not fully decoded here. |

## 8. Negative Facts / Do Not Do

- Do not treat `g_GameMode == 5` as replay playback. Active in YR: `g_GameMode == 5` is Skirmish in verified branches; replay playback is `DAT_00A8D5F8 & 2`.
- Do not call or emulate `FUN_00551B90` for native replay startup. That helper belongs to savegame content load, not replay.
- Do not add a replay-specific pass that re-registers every active object through `FUN_0055BAA0`; no such path was found in replay startup or per-frame playback.
- Do not let `live_object_order_snapshot` sorted missing-ID fallback stand in for native replay order. The binary replay path neither sorts nor repairs active-vector members from object storage.
- Do not update stale docs by saying "replay restore is unresolved in the same way as save/load." Save/load vector order is settled separately; replay startup is normal scenario init plus recording playback.

## 9. Remaining Uncertainty

- Exact UI/file path that sets `DAT_00A8D5F8 & 4` before `Main_Game` case `-1` was not traced.
- Exact `FUN_00551A30` virtual sort key semantics are not decoded; this is an active-order maintenance follow-up, not replay restore.
- Runtime watchpoints would be needed to sample final `Object+0x98` for every Object-derived class during replay playback, but no static replay-specific writer/rebuilder was found.

## 10. Stale Docs / Replacement Wording

- Replace any wording equivalent to "`g_GameMode == 5` is replay playback" with: "`g_GameMode == 5` is the standard Skirmish branch in the verified `Main_Game`, `ScenarioClass`, and `Main_Tick` bodies. Replay playback is controlled by `DAT_00A8D5F8 & 2`; recording is `& 1`; case `-1` can set playback after `& 4` and replay availability checks."
- `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md` OQ-AOOS-013 replacement: "Replay playback does not use the savegame active-vector load path. `Main_Game` reads replay header fields, then launches normal scenario initialization; per-frame `Main_Tick` replay reads sync/selection/cursor data and does not rebuild or re-register `LogicClass` members. Remaining replay uncertainty is runtime byte sampling and full replay-event decoding, not active-vector restore ownership."
- `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` OQ-LOGICREG-022 replacement: "Savegame vector order and replay startup are separate: savegame load directly loads the `LogicClass` vector through `FUN_00551B90`; replay startup does not. Replay uses normal scenario initialization plus ordinary lifecycle registration, with no replay-specific `FUN_0055BAA0` re-registration pass found."
- Any "tail append order only" active-order wording should be qualified if it discusses per-tick order: "`ObjectClass::Reveal` tail-appends to the active vector, but `Main_Tick` also calls `FUN_00551A30` before `LogicClass::PerTickUpdate`, performing a one-pass adjacent swap by an object virtual key. The key semantics require a separate active-order helper investigation before making global final-order claims."

## Sources

- Ghidra decompile/read-only: `Main_Game @ 0x0052D9A0`, `FUN_00473C50`, `FUN_00473D10`, `FUN_00473B10`, `FUN_00473AE0`, `Main_Tick @ 0x0055D360`, `FUN_00551A30`, `ScenarioClass::Read_Scenario @ 0x00684620`, `ScenarioClass::Full_Init @ 0x00686B20`, `FUN_0067E440`, `FUN_0067E730`, `FUN_00551B90`, `FUN_00551B20`.
- Ghidra disassembly ranges successfully checked: `0x0052DC9F..0x0052DD29`, `0x0052E356..0x0052E660`, `0x0055D860..0x0055D930`, `0x0055D940..0x0055DBAF`, `0x0055DBB0..0x0055DCA7`, `0x00551A30..0x00551A84`, `0x006876F0..0x00687B1F`. (corrected 2026-05-29: was `0x00551A30..0x00551A8E`; actual function end `0x00551A84` via `disassemble_function 0x00551A30`)
- Prior docs: `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`, `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`, `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`, `RNG_SYSTEM_GHIDRA_REPORT.md`, `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/replay.rs`, `src/sim/snapshot.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`.
