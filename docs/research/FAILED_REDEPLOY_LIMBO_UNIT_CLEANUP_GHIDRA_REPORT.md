# Failed Redeploy Limbo Unit Cleanup - Ghidra Research Report

**Address(es):** `0x00449C30` (`BuildingClass__Sell` failed `UndeploysInto` unlimbo branch), `0x007353C0` (`UnitClass` full constructor), `0x00735780` (`UnitClass` destructor body, mislabeled constructor), `0x00737BA0` (`UnitClass__Unlimbo`), `0x005F4EC0` (`ObjectClass__Reveal`), `0x005F65F0` (`ObjectClass__UnInit`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Lifetime and cleanup of the constructed AMCV object after failed `UnitClass::Unlimbo` during stock YR `GACNST -> AMCV` redeploy. This report only answers whether the failed limbo unit is explicitly deleted, swept from global arrays, queued for normal pending-delete cleanup, or left live in limbo.  
**Non-Scope:** Visible money/object result already settled by `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`; normal successful redeploy transfer; AMCV forward deploy validation; non-ConYard `UndeploysInto` custom behavior beyond shared branch structure.  
**Confidence:** High for the absence of same-path cleanup and the arrays that remain registered; Medium for long-horizon consequences of a live limbo AMCV because runtime observation was not performed.  
**Active in YR:** Yes. Stock `rulesmd.ini` has `[GACNST] ConstructionYard=yes`, `[GACNST] UndeploysInto=AMCV`, and `[MultiplayerDialogSettings] MCVRedeploys=yes`.

## 1. Overview

On the failed redeploy path, `BuildingClass__Sell` constructs a real `UnitClass` before calling `UnitClass__Unlimbo`. If that unlimbo call returns false, the branch refunds the owner and destroys/uninitializes the source building, but it does not clean up the newly constructed AMCV.

The failed AMCV remains an allocated, alive, in-limbo UnitClass registered in the normal constructor global arrays. It is not queued in `ObjectClass__UnInit`'s pending-delete list and is not swept from `g_UnitClass_Array` or the RTTI/id directory on this path.

## 2. Class Layout / Key Offsets

| Offset / global / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass+0x81` | `InLimbo` byte, constructor initializes to `1` | `ObjectClass__Constructor @ 0x005F3900`; `ObjectClass__Reveal @ 0x005F4EC0` | Yes |
| `ObjectClass+0x90` | alive/active byte, constructor initializes to `1`; `ObjectClass__UnInit` clears it | `0x005F3900`; `0x005F65F0` | Yes |
| `UnitClass` vtable `+0xD8` | `UnitClass__Unlimbo`, called by redeploy branch | `BuildingClass__Sell @ 0x0044A002`; `UnitClass__Unlimbo @ 0x00737BA0` | Yes |
| `UnitClass` vtable `+0x20` | scalar deleting destructor slot | used elsewhere as cleanup slot; not called for failed AMCV in `0x0044A16B..0x0044A1D2` | Yes, negative fact here |
| `ObjectClass` vtable `+0xF8` | `ObjectClass__UnInit`/Destroy cleanup slot on many object paths | source building cleanup at `0x0044A1CC`; not called on `pBVar6` | Yes |
| `g_UnitClass_Array` / `g_UnitClass_Array_Count` | UnitClass global array | add in `UnitClass` full constructor `0x0073546x`; remove in destructor body `0x0073588x..0x007358B8` | Yes |
| `DAT_00B0E840` / `DAT_00B0E844` | RTTI id -> object pointer directory | add in `UnitClass` full constructor tail; remove in `UnitClass` destructor body | Yes |
| `g_ObjectClass_Array` / `g_ObjectClass_Array_Count` | ObjectClass global array | add in `ObjectClass__Constructor @ 0x005F3900`; remove in `ObjectClass__Destructor @ 0x005F3B80` | Yes |
| `DAT_00B0F69C` / `DAT_00B0F6A8` | pending-delete list/count | append in `ObjectClass__UnInit @ 0x005F65F0`; no append for failed AMCV branch | Yes |

## 3. Core Logic

The relevant redeploy state-2 sequence in `BuildingClass__Sell @ 0x00449C30` is:

1. Allocate `0x8E8` bytes and call `UnitClass` full constructor from `Type+0x408` (`UndeploysInto=AMCV`).
2. Constructor registers the AMCV immediately in global tracking:
   - `ObjectClass__Constructor @ 0x005F3900` sets `InLimbo=1`, alive byte `+0x90=1`, and pushes to `g_ObjectClass_Array`, secondary object/listener arrays, and master abstract registry.
   - `FootClass__Constructor @ 0x004D31E0` pushes to foot/team-listener style arrays.
   - `UnitClass` full constructor `0x007353C0` pushes to `g_UnitClass_Array` and the RTTI id directory `DAT_00B0E840`.
3. `BuildingClass__Sell` detaches/conceals the source building via `param_1->vtable+0xD4`, computes facing, then calls the new unit's vtable `+0xD8`.
4. `UnitClass__Unlimbo @ 0x00737BA0` calls `FootClass__Unlimbo`, which calls `TechnoClass__Unlimbo`, which calls `ObjectClass__Reveal`.
5. `ObjectClass__Reveal @ 0x005F4EC0` returns false without deleting the object if the coordinates are invalid, game inactive, the object cannot enter the target cell, or `Mark(PUT)` fails. In the early blocker case, `InLimbo` remains as initialized (`1`); in the Mark-fail case, it is explicitly restored to `1`.
6. The failed branch in `BuildingClass__Sell` only calls `HouseClass__Add_Credits`, frees the temporary vector if allocated, then cleans up the source building by calling building `vtable+0xDC(1)`, `SoundEvent__Release`, and building `vtable+0xF8`.
7. No call in the failed branch targets `pBVar6` with `vtable+0x20`, `vtable+0xD4`, `vtable+0xE0`, `vtable+0xF8`, `ObjectClass__UnInit`, or the `UnitClass` destructor body at `0x00735780`.

The destructor/removal contrast is important:

- `UnitClass` destructor body `0x00735780` calls `FootClass__Limbo` when appropriate, `Detach_From_All_Lists`, removes self from `g_UnitClass_Array`, removes its RTTI id entry from `DAT_00B0E840`, then chains to the `FootClass` destructor.
- `ObjectClass__UnInit @ 0x005F65F0` calls `Detach_From_All_Lists`, calls virtual Limbo, clears alive byte `+0x90`, and appends the object to pending-delete list `DAT_00B0F69C`.
- Neither path is invoked for the failed AMCV in `BuildingClass__Sell`.

## 4. INI Keys

| Section/key | Stock value | Use in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[MultiplayerDialogSettings] MCVRedeploys` | `yes` | Enables stock ConYard redeploy gate | `ini/rulesmd.ini:3041`; `0x00449BC0`, `0x00449C30` | Yes |
| `[GACNST] ConstructionYard` | `yes` | Selects ConYard-special MCV redeploy chain | `ini/rulesmd.ini:11625`; `0x00449CFE..0x00449D5E` | Yes |
| `[GACNST] UndeploysInto` | `AMCV` | Target `UnitTypeClass` used by constructor | `ini/rulesmd.ini:11631`; `0x00449E34..0x00449E44` | Yes |
| `[AMCV]` | stock unit type | Constructed UnitClass type after failed redeploy | `ini/rulesmd.ini:6969`; `UnitClass` constructor `0x007353C0` | Yes |

## 5. Integration Points

`BuildingClass__Sell @ 0x00449C30` is the only function in this slice that owns the failed redeploy branch. It already removed the source building from the map before trying `UnitClass__Unlimbo`; failure does not unwind that source-building cleanup.

`UnitClass__Unlimbo @ 0x00737BA0` is a pure placement attempt plus success initialization. Its failure return does not self-destruct the unit. All deletion/removal behavior lives in separate object cleanup paths such as `ObjectClass__UnInit @ 0x005F65F0` and the destructor chain.

`ObjectClass__Reveal @ 0x005F4EC0` is the actual failure source for normal blocked-cell placement. It either leaves `InLimbo` unchanged or restores it, then returns `0`; it does not queue pending delete.

## 6. Current Rust Implementation Status

Current Rust does not model this leaked limbo object. `src/sim/world/mod.rs:989..1002` despawns the building, calls `spawn_object_at_height`, and if that returns `None`, no replacement entity exists. `src/sim/world/world_spawn.rs:423..443` inserts only the successfully created entity and returns `Some(stable_id)`.

That is acceptable for the current high-level object result, because no visible AMCV appears. If a future object lifecycle model introduces preallocated limbo objects before placement, it should either intentionally model gamemd's live limbo leak or consciously collapse it away while preserving all observable object counts/queries that can see limbo units.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__Sell` failed `UnitClass__Unlimbo` branch | verified | `0x0044A002..0x0044A1D2` | none |
| Same-branch AMCV cleanup calls | verified absent | no `pBVar6` call to `+0x20`, `+0xD4`, `+0xE0`, `+0xF8`, or `ObjectClass__UnInit` in `0x0044A16B..0x0044A1D2` | none |
| `UnitClass` constructor registration | verified | `0x007353C0`; `0x005F3900`; `0x004D31E0` | exact human-readable names of every secondary vector not needed |
| `UnitClass__Unlimbo` failure contract | verified | `0x00737BA0`; `0x004D7170`; `0x006F6CA0`; `0x005F4EC0` | none for cleanup semantics |
| `ObjectClass__Reveal` blocked/Mark-fail behavior | verified | `0x005F4EC0` | exact passability reason depends on caller scenario |
| Pending-delete path contrast | verified | `ObjectClass__UnInit @ 0x005F65F0` | exact end-of-tick sweep function not re-decompiled in this slot |
| Destructor/global-array removal contrast | verified | `UnitClass` destructor body `0x00735780`; `ObjectClass__Destructor @ 0x005F3B80` | full destructor naming cleanup outside scope |
| Current Rust no-limbo-object behavior | verified | `src/sim/world/mod.rs:989..1002`; `src/sim/world/world_spawn.rs:423..443` | no Rust edits in this investigation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is the target path active in stock YR? -> Yes; stock GACNST is a ConstructionYard with UndeploysInto=AMCV and MCVRedeploys defaults yes.` (evidence: `ini/rulesmd.ini:3041,11625,11631`; `0x00449BC0`; `0x00449C30`)
- `[RESOLVED] OQ2 - Does `UnitClass` construction register the AMCV before placement? -> Yes; constructor chain pushes into ObjectClass/Foot/Unit arrays and the RTTI id directory before `Unlimbo` is called.` (evidence: `0x005F3900`; `0x004D31E0`; `0x007353C0`)
- `[RESOLVED] OQ3 - Does `UnitClass__Unlimbo` delete or uninit itself on failed placement? -> No; it returns 0 when `FootClass__Unlimbo`/`ObjectClass__Reveal` fails and contains no self-cleanup call.` (evidence: `0x00737BA0`; `0x004D7170`; `0x006F6CA0`; `0x005F4EC0`)
- `[RESOLVED] OQ4 - Does `ObjectClass__Reveal` queue pending delete when placement fails? -> No; it returns false with `InLimbo` left/restored to 1 and does not call `ObjectClass__UnInit`.` (evidence: `0x005F4EC0`; contrast `0x005F65F0`)
- `[RESOLVED] OQ5 - Does `BuildingClass__Sell` clean up the failed AMCV after `Unlimbo` returns false? -> No; the failed branch refunds, frees only the temporary contact vector if allocated, and cleans up the source building.` (evidence: `0x0044A16B..0x0044A1D2`)
- `[RESOLVED] OQ6 - Where would global UnitClass/RTTI array removal occur if cleanup did happen? -> In the `UnitClass` destructor body at `0x00735780`, which is not invoked on this failed branch.` (evidence: `0x00735780`)
- `[RESOLVED] OQ7 - Where would pending-delete normal cleanup be scheduled if cleanup did happen? -> `ObjectClass__UnInit @ 0x005F65F0` appends to `DAT_00B0F69C`, but the failed AMCV never reaches it.` (evidence: `0x005F65F0`; negative branch evidence `0x0044A16B..0x0044A1D2`)
- `[RESOLVED] OQ8 - Is the leaked AMCV alive or dead-limbo? -> Alive limbo: constructor sets `InLimbo=1` and alive byte `+0x90=1`; failure path does not clear alive or queue pending delete.` (evidence: `0x005F3900`; `0x005F4EC0`; `0x005F65F0`)
- `[RESOLVED] OQ9 - Does the source GACNST cleanup imply AMCV cleanup? -> No; the final `vtable+0xDC`, `SoundEvent__Release`, and `vtable+0xF8` calls are on `param_1` (building), not `pBVar6` (new unit).` (evidence: `0x0044A1B5..0x0044A1D2`)
- `[RESOLVED] OQ10 - Does current Rust model a failed limbo object? -> No; failed `spawn_object_at_height` produces no entity, because insertion happens only on success.` (evidence: `src/sim/world/mod.rs:989..1002`; `src/sim/world/world_spawn.rs:423..443`)
- `[DEFERRED] OQ11 - Can the live limbo AMCV affect any later stock-YR house/global scans after failed redeploy?` (category: `needs-runtime-debugger`; reason: binary proves the object remains registered, but this slot did not run a retail scenario to observe later count/AI side effects; next-step-if-pursued: force failed redeploy in debugger and inspect `g_UnitClass_Array`, house tracking, and save/load state after several ticks)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Failed redeploy leaves the constructed AMCV allocated, alive, and in limbo; it is not visible because `Unlimbo` failed | `0x0044A002..0x0044A1D2`; `0x00737BA0`; `0x005F4EC0` | currently collapsed away: no entity exists when spawn fails | future object lifecycle / limbo-object model; current high-level surface `src/sim/world/mod.rs::tick_building_down` | If limbo lifecycle is modeled, do not automatically delete the failed AMCV unless choosing a deliberate simplification with documented parity risk | Forced blocked AMCV redeploy: no visible AMCV or GACNST; if internal limbo arrays are modeled, one alive limbo AMCV remains registered | Do not call destructor/UnInit on failed `Unlimbo` as though placement failure rolls back construction |
| The failed AMCV is not in pending-delete cleanup | `ObjectClass__UnInit @ 0x005F65F0`; absence in `0x0044A16B..0x0044A1D2` | current Rust has no pending-delete equivalent for this object because object is not created | future pending-delete/despawn queue | Keep pending-delete semantics reserved for explicit UnInit/death/destruction paths, not failed placement return | Internal test differentiates failed placement from death: failed limbo object remains alive if modeled; dead objects enter pending-delete | Do not reuse dead-limbo cleanup for this branch |
| Constructor registration happens before placement attempt, and destructor removal is the only verified same-class array sweep | `0x007353C0`; `0x00735780`; `0x005F3B80` | current Rust inserts only after `spawn_object_at_height` success | future preallocation path; entity store insertion timing | If matching gamemd internals, register constructed objects before unlimbo; failed unlimbo does not unwind registration | Instrumented lifecycle test: constructor registers; failed Unlimbo does not remove from Unit/Object/RTTI registries | Do not make global arrays imply "visible on map"; limbo objects can be registered but not placed |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` section 11 should replace its uncertainty sentence with: `Resolved by FAILED_REDEPLOY_LIMBO_UNIT_CLEANUP_GHIDRA_REPORT.md: the constructed AMCV is not deleted, uninitialized, pending-delete queued, or swept from Unit/Object/RTTI arrays on the failed UnitClass::Unlimbo branch. It remains an alive, in-limbo registered UnitClass; later gameplay consequences beyond the immediate no-visible-AMCV result require runtime probing.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` coverage row "New unit deletion after failed unlimbo in same function" should replace "deeper global limbo cleanup deferred" with: `deeper global cleanup resolved by FAILED_REDEPLOY_LIMBO_UNIT_CLEANUP_GHIDRA_REPORT.md: no destructor/UnInit/pending-delete/global-array removal is reached for the failed AMCV.`

## 10. Negative Facts / Do Not Do

- Do not say failed `UnitClass::Unlimbo` deletes the AMCV; neither `UnitClass__Unlimbo` nor its `ObjectClass__Reveal` failure path does that.
- Do not say `BuildingClass__Sell` cleans up `pBVar6` after failure; the cleanup calls after `0x0044A16B` are for the source building.
- Do not say the failed AMCV is queued for normal pending-delete cleanup; `ObjectClass__UnInit @ 0x005F65F0` is not called.
- Do not say the failed AMCV is swept from `g_UnitClass_Array` or `DAT_00B0E840`; the verified removal code is in the UnitClass destructor body, which is not reached.
- Do not conflate registered/global with visible/on-map. The AMCV is registered and alive but remains `InLimbo=1`, so the visible result is still no AMCV.

## 11. Remaining Uncertainty

Runtime side effects of the live registered limbo AMCV after several ticks are not fully proven here. The binary evidence proves non-cleanup on the failure path; a debugger probe would be needed to determine whether stock YR later counts, saves, AI-updates, or otherwise notices the ghost limbo unit in a player-visible way.

## Sources

- Ghidra read-only decompiled: `BuildingClass__Sell @ 0x00449C30`, `UnitClass` full constructor `0x007353C0`, `UnitClass` destructor body `0x00735780`, `UnitClass__Unlimbo @ 0x00737BA0`, `FootClass__Unlimbo @ 0x004D7170`, `TechnoClass__Unlimbo @ 0x006F6CA0`, `ObjectClass__Reveal @ 0x005F4EC0`, `ObjectClass__Constructor @ 0x005F3900`, `ObjectClass__UnInit @ 0x005F65F0`, `ObjectClass__Destructor @ 0x005F3B80`, `FootClass__Constructor @ 0x004D31E0`, `FootClass` destructor body `0x004D3590`, `Detach_From_All_Lists`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/YAREFN_UNDEPLOY_TO_SMIN_SLAVEMANAGER_PATH_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini:3041`, `ini/rulesmd.ini:6969..6983`, `ini/rulesmd.ini:11622..11634`.
- Rust scanned for handoff only: `src/sim/world/mod.rs:958..1004`, `src/sim/world/world_spawn.rs:292..443`, `src/sim/world/world_spawn.rs:590..631`, `src/sim/world/world_spawn.rs:679..691`.
