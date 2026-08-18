# House Result Bytes Save/Load Persistence `+0x1F6/+0x1F7/+0x1F8` - Ghidra Research Report

**Address(es):** `HouseClass` vtable `0x007EA8A0`; load slot `+0x14 -> 0x00503040`; save slot `+0x18 -> 0x00504080`; size slot `+0x30 -> 0x00504730`; raw persistence owner `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`; load-specific constructor `HouseClass::Constructor @ 0x004F5190`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** Save/load serialization, load-time reinitialization, and persistence semantics for `HouseClass+0x1F6/+0x1F7/+0x1F8` plus directly adjacent result timing fields `+0x298/+0x2A0`.
**Non-Scope:** Win/loss setter lifecycle, trigger/script caller taxonomy, full `HouseClass` field map, and victory/defeat UI. Those are covered by `HOUSE_RESULT_BYTES_0X1F7_0X1F8_LIFECYCLE_RESWARM_20260528.md`.
**Confidence:** High for raw save/load persistence and no load-specific reset in the inspected load body. Medium for full data-reference census outside the save/load functions because this slot did not run a whole-program struct-offset xref script.
**Active in YR:** Conditional. The path is active in standard YR save/load; the persisted nonzero values require saving while one of the result/pending bytes or borrowed-time fields is live.

## 0. Investigation Contract

**Target question:** Are `HouseClass+0x1F6/+0x1F7/+0x1F8` and directly adjacent borrowed-time fields serialized, reset, or reconstructed during native HouseClass save/load?

**Non-goals:** Do not redo `Flag_To_Win`, `Flag_To_Lose`, `Flag_To_Win_Check`, `Check_Win_Condition`, or `HouseClass::Update` result-to-global mapping.

**Evidence needed to mark COMPLETE:** House vtable evidence for save/load/size slots; raw `AbstractClass::Save/Load` decompile plus assembly; House load/save assembly ranges; load-specific constructor decompile; a bounded binary scan showing no post-raw-load direct writes to `+0x1F6/+0x1F7/+0x1F8/+0x298/+0x2A0`.

**Stop conditions:** Stop after the raw persistence mechanism and lack of load reset are proven for this slice, Rust snapshot/hash implications are recorded, and remaining non-save/load lifecycle questions are left to their owning reports.

## 1. Overview

`HouseClass` save/load does not special-case the result bytes. The normal IPersist stream path first writes/reads the entire `HouseClass` raw body using the class-size virtual (`0x160B8` bytes), so `+0x1F6`, `+0x1F7`, `+0x1F8`, `+0x298`, and `+0x2A0` persist byte-for-byte when present in a save.

The load wrapper then runs a load-specific House constructor and swizzle/dynamic-container fixups. That constructor restores vtables and dynamic-vector scaffolding but does not reset the result bytes or the borrowed-time fields in this slice. Active in YR: Conditional save/load path; standard savegame content uses the `g_HouseClass_Array` OLE loop.

## 2. Key Offsets

| Offset | Type | Save/load result | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x1F6` | byte | raw-saved and raw-loaded; no post-load direct reset found | `AbstractClass::Save/Load`, House load range scan `0x00503040..0x00504070` | Conditional |
| `+0x1F7` | byte | raw-saved and raw-loaded; no post-load direct reset found | same | Conditional |
| `+0x1F8` | byte | raw-saved and raw-loaded; no post-load direct reset found | same | Conditional |
| `+0x298` | dword | raw-saved and raw-loaded; no post-load direct reset found | same | Conditional |
| `+0x2A0` | dword | raw-saved and raw-loaded; no post-load direct reset found | same | Conditional |
| class size | dword | `0x160B8` bytes | House vtable `+0x30 -> 0x00504730`; assembly `MOV EAX,0x160B8; RET` | Yes |

## 3. Core Logic

### 3.1 Standard save emits HouseClass objects

`FUN_0067D300` writes `g_HouseClass_Array_Count`, then iterates `g_HouseClass_Array[i]`, queries `IPersistStream`, and calls `OleSaveToStream`. Active in YR: Conditional, standard savegame content stream.

Material effect: each HouseClass object reaches its vtable `+0x18` save slot (`0x00504080`) through COM/OLE persistence, not a custom result-byte serializer.

### 3.2 HouseClass save starts with raw body persistence

House vtable memory from `gamemd.exe`:

| Vtable offset | Function | Role |
|---:|---:|---|
| `0x14` | `0x00503040` | HouseClass load |
| `0x18` | `0x00504080` | HouseClass save |
| `0x30` | `0x00504730` | class raw-body size |

At `0x00504080`, HouseClass save immediately calls `AbstractClass::Save`:

```text
0x00504090 push stream
0x00504091 push this
0x00504092 push saved-this/object
0x00504093 call 0x00410320
```

`AbstractClass::Save @ 0x00410320` writes a 4-byte pointer token, calls the saved object's vtable `+0x30`, and writes `size` bytes from the object pointer to the stream. For HouseClass, the vtable `+0x30` target is `0x00504730`, which returns `0x160B8`. Active in YR: Conditional save.

Tiny detail: raw save happens before the HouseClass save body serializes dynamic vectors and pointer-list extras (`0x005040A2+`). Therefore the result bytes are saved in their current in-memory state at the beginning of the HouseClass save call.

### 3.3 HouseClass load raw-loads before load-specific reconstruction

At `0x00503040`, HouseClass load tears down/clears several dynamic containers, then calls raw `AbstractClass::Load`:

```text
0x005031C7 push stream
0x005031C8 push this
0x005031C9 call 0x00410380
0x005031CE test eax,eax
0x005031D0 jl failure
0x005031E1 call 0x004F5190
```

`AbstractClass::Load @ 0x00410380` reads a 4-byte old-pointer token, registers it with the global swizzle manager, calls the object's vtable `+0x30`, and reads `size` bytes into the object. For HouseClass, the size is again `0x160B8`. Active in YR: Conditional load.

The post-raw-load call to `HouseClass::Constructor @ 0x004F5190` is a load-specific/vtables-and-containers constructor. Its decompile initializes vtables, dynamic vector vtables/counts, base nodes, and a few container fields; it does not contain the full-constructor contiguous zeroing of `+0x1F5..+0x1F8`, nor the full-constructor writes to `+0x298/+0x2A0`.

### 3.4 Bounded reset scan

A bounded byte-pattern scan of the HouseClass load body `0x00503040..0x00504070` found no direct displacement references to:

```text
+0x1F5, +0x1F6, +0x1F7, +0x1F8, +0x294, +0x298, +0x29C, +0x2A0
```

A parallel scan of the save body `0x00504080..0x005046E2` found no real direct references to those offsets either. One raw byte-pattern hit for `+0x1F6` landed inside a branch instruction immediate at `0x005044E2`, not a field access; Ghidra disassembly shows it is `JL 0x005046DE`.

Material effect: within the verified HouseClass load/save bodies, these fields are handled by the raw object-body read/write, not by later clear/recompute code.

## 4. INI Keys

None. These fields are runtime HouseClass state and savegame stream state. Active in YR: Yes as engine state; conditional for save/load.

## 5. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_0067D300` | standard content save owner; emits HouseClass OLE records | decompile shows `g_HouseClass_Array_Count` loop and `OleSaveToStream` | Conditional save |
| `FUN_0067E730` | standard content load owner; loads HouseClass OLE records in matching content order | prior save/load docs and decompile of content loader | Conditional load |
| `FUN_0067E440` | savegame load wrapper; runs content load then swizzle fixup and post-load refresh | decompile; calls `FUN_0067E730` then `FUN_006CF230` | Conditional load |
| `AbstractClass::Save/Load` | raw-body persistence owner | decompile plus assembly `0x00410320..0x00410374`, `0x00410380..0x004103D6` | Conditional save/load |
| `HouseClass::Constructor @ 0x004F5190` | post-raw-load vtable/container reconstruction | decompile; called at `0x005031E1` | Conditional load |

## 6. Current Rust Implementation Status

Rust currently has `is_defeated`, `has_won`, and `has_lost` in `src/sim/house_state.rs:32..36`; these fields serialize through the whole `Simulation` snapshot in `src/sim/snapshot.rs:84..107` and contribute to `state_hash` in `src/sim/world/world_hash.rs:117..119`.

Rust does not model native `House+0x1F6` pending-win byte, borrowed-time start frame `+0x298`, or borrowed-time duration `+0x2A0`. `Simulation::check_defeat` still writes direct `has_won`/`is_defeated` style state at `src/sim/world/mod.rs:706..762`, so a save during native pending/borrowed-time windows has no equivalent Rust state to persist yet.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| House vtable save/load/size slots | verified | binary vtable dump `0x007EA8B4/+0x14`, `0x007EA8B8/+0x18`, `0x007EA8D0/+0x30` | none |
| class-size virtual | verified | assembly `0x00504730: MOV EAX,0x160B8; RET` | none |
| raw save includes result bytes/timers | verified | `AbstractClass::Save @ 0x00410320` plus class size | none |
| raw load includes result bytes/timers | verified | `AbstractClass::Load @ 0x00410380` plus class size | none |
| House save wrapper starts with raw save | verified | assembly `0x00504090..0x00504098` | none |
| House load wrapper calls raw load before load-specific constructor | verified | assembly `0x005031C7..0x005031E1` | none |
| load-specific constructor reset of target fields | verified negative | `0x004F5190` decompile plus no direct target displacement hits in `0x00503040..0x00504070` | none for this slice |
| full whole-program data-xref census | deferred | no mutating script; scoped static scan only | future whole-program offset scan if needed |

## 8. Open Questions - Final State

- `[RESOLVED] HRB-SL-001 - Which House vtable slots own load/save? -> load is `+0x14 -> 0x00503040`, save is `+0x18 -> 0x00504080`.` (evidence: vtable dump from `gamemd.exe` at `0x007EA8A0`)
- `[RESOLVED] HRB-SL-002 - What byte size does raw House persistence use? -> `0x160B8`.` (evidence: `0x00504730`)
- `[RESOLVED] HRB-SL-003 - Does save raw-write the result bytes? -> Yes, as part of the `0x160B8` object body before extras.` (evidence: `0x00504090..0x00504098`, `0x00410320..0x00410374`)
- `[RESOLVED] HRB-SL-004 - Does load raw-read the result bytes? -> Yes, as part of the `0x160B8` object body before load fixups.` (evidence: `0x005031C7..0x005031E1`, `0x00410380..0x004103D6`)
- `[RESOLVED] HRB-SL-005 - Does the post-load House constructor clear `+0x1F6/+0x1F7/+0x1F8`? -> No direct clear found; `0x004F5190` is not the full constructor and does not contain the full-constructor result-byte zeroing.` (evidence: `0x004F5190` decompile; bounded scan)
- `[RESOLVED] HRB-SL-006 - Does the post-load House body clear `+0x298/+0x2A0`? -> No direct clear found in the inspected load body.` (evidence: bounded scan `0x00503040..0x00504070`)
- `[RESOLVED] HRB-SL-007 - Is this active in standard YR? -> Yes conditionally: standard save/load uses the content/OLE persistence path, but nonzero result-byte persistence only matters when saving during an active pending/result window.` (evidence: `FUN_0067D300`, `FUN_0067E440`)
- `[DEFERRED] HRB-SL-008 - Are there non-save/load writers elsewhere beyond lifecycle report?` (category: `out-of-scope`; reason: this slot is save/load only; next-step-if-pursued: whole-program struct-offset scan for `+0x1F6/+0x1F7/+0x1F8`)
- `[DEFERRED] HRB-SL-009 - What exact UI/session behavior occurs when loading with expired borrowed time?` (category: `requires-different-system-context`; reason: this needs runtime save/load fixture timing around `HouseClass::Update`; next-step-if-pursued: native save at pending frame then load and observe first `HouseClass::Update`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native HouseClass raw save/load persists `+0x1F6/+0x1F7/+0x1F8` byte-for-byte. Active in YR: Conditional save/load. | `0x00504090..0x00504098`; `0x005031C7..0x005031E1`; `AbstractClass::Save/Load`; `0x00504730` size `0x160B8` | missing `+0x1F6`; `has_won/has_lost` exist but are direct booleans | `src/sim/house_state.rs`, `src/sim/snapshot.rs`, `src/sim/world/world_hash.rs` | Add/persist native-equivalent pending/result bytes before claiming save/load parity for victory/defeat state. | Save with pending win byte set and load before `HouseClass::Update`; loaded state must still block `Flag_To_Lose` from setting loss exactly as native. | proposed test `house_snapshot_roundtrip_preserves_pending_result_bytes`; risk: reconstructing from `is_defeated/has_won/has_lost` loses transient branches |
| Native raw save/load persists borrowed-time start/duration fields `+0x298/+0x2A0`; load-specific constructor does not reset them in this slice. Active in YR: Conditional save/load. | `AbstractClass::Save/Load`; no direct target-offset hits in `0x00503040..0x00504070`; `0x004F5190` decompile | missing fields | `src/sim/house_state.rs`, native frame/session-end state, snapshot/hash tests | Store the native borrowed-time frame base and duration for pending win/loss resolution. | Save with `has_lost` set and remaining borrowed time nonzero; after load, first ticks continue countdown from saved frame/duration, not from load tick. | proposed test `house_snapshot_roundtrip_preserves_borrowed_time_window`; risk: restarting borrowed time on load shifts session-end by ticks |
| House load fixup reconstructs containers and swizzles after raw load but does not recompute result bytes from current map state. Active in YR: Conditional load. | `0x005031C7..0x00503339`; `0x004F5190`; `FUN_0067E440 -> FUN_006CF230` | Rust snapshot load is structural and currently has no native result-byte lifecycle | `src/sim/snapshot.rs`, `src/sim/world/mod.rs::check_defeat` | Keep saved result/pending state authoritative across load; do not call defeat/victory recomputation as a substitute for loading these bytes. | Load a save where a house owns no buildings but has not yet resolved its saved result byte; loaded state follows saved pending/result bytes until the next native-equivalent update point. | proposed test `house_load_does_not_recompute_result_bytes_from_current_counts`; risk: immediate recomputation hides pending-window parity bugs |

### Negative Facts / Do Not Do

- Do not treat `House+0x1F6` as a transient-only byte that can be dropped from snapshots. Active in YR: Conditional; evidence: raw `0x160B8` HouseClass persistence includes it.
- Do not reset `+0x1F7/+0x1F8` on load from current game outcome. Active in YR: No; evidence: load raw-reads then no direct reset found in `0x00503040..0x00504070`.
- Do not restart borrowed time at load. Active in YR: No evidence for restart; raw `+0x298/+0x2A0` persist and no load reset was found.
- Do not infer save/load behavior from the full constructor `0x004F54A0`. Active in YR: No for load-specific reset; the load path calls `0x004F5190`, not the full constructor, after raw load.
- Do not serialize only final `has_won/has_lost` booleans and call it native parity. Active in YR: No; `+0x1F6`, `+0x298`, and `+0x2A0` carry state that affects later branches.

### Stale Docs / Follow-up Docs

- `docs/research/HOUSE_RESULT_BYTES_0X1F7_0X1F8_LIFECYCLE_RESWARM_20260528.md`: replace "Save/load persistence semantics for these exact bytes are not proven in this slice" with "`HouseClass` IPersist save/load raw-writes/raw-reads the full `0x160B8` HouseClass body through `AbstractClass::Save/Load`, so `+0x1F6/+0x1F7/+0x1F8` and borrowed-time fields `+0x298/+0x2A0` persist byte-for-byte; the load-specific constructor `0x004F5190` and House load fixup body do not reset them in the verified load range."
- `docs/contracts/NATIVE_FRAME_TICK_TIMING_IMPLEMENTATION_CONTRACT.md` or any contract still listing House result-byte save/load as unknown: replace with "House result/pending bytes and borrowed-time fields are native save/load state and must survive snapshot restore before the next House update/session-end gate evaluation."

## 10. Remaining Uncertainty

- No runtime fixture was captured for loading at the exact frame where borrowed time expires; this report proves persistence, not the first post-load frame's visible session UI/audio timing.
- A full whole-program direct data-reference census was not run; this report is complete for HouseClass save/load treatment and intentionally leaves unrelated writers/readers to the lifecycle report or a future whole-program offset scan.

## Sources

- Read-only Ghidra decompile: `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`, `HouseClass::Constructor @ 0x004F5190`, `FUN_0067D300`, `FUN_0067E440`.
- Read-only Ghidra assembly context: `0x00503040`, `0x005031C7..0x00503339`, `0x00504080..0x00504184`, `0x00504730`.
- Binary vtable dump from retail `gamemd.exe`: `vtable__HouseClass @ 0x007EA8A0`.
- Prior docs: `HOUSE_RESULT_BYTES_0X1F7_0X1F8_LIFECYCLE_RESWARM_20260528.md`, `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`, `FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`.
- Rust scan: `src/sim/house_state.rs`, `src/sim/snapshot.rs`, `src/sim/world/world_hash.rs`, `src/sim/world/mod.rs`.
