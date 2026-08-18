# Garrison Owner/EVA/SFX Timing Reswarm - Ghidra Research Report

**Address(es):** `0x00522910` (`BuildingClass::AddGarrisonOccupant`), `0x00458200` (`BuildingClass::CheckAutoSellOrCivilian`), `0x0043FB20` (`BuildingClass::Update`), `0x0055AFB0` (`LogicClassPerTickUpdateLiveVector`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** first-occupant boarding side effects, owner identity used for garrisoned/abandoned EVA/SFX gates, ownership reconciliation order after boarding, and empty captured-garrison revert owner semantics.
**Non-Scope:** sell/ejection cell choice, garrison weapon fire, rendering/body frame state, tank bunker behavior, full generic passenger transport behavior, and full sell command eligibility.
**Confidence:** High for scoped binary order/owner/cue claims; Medium for concrete retail map object indices because runtime object-vector indices were not sampled.
**Active in YR:** Yes for stock `CanBeOccupied=yes` civilian garrisons and `Occupier=yes` infantry; conditional on the building passing the `CheckAutoSellOrCivilian` type gate.

## 0. Working Notes Gate

- Target question: Does `AddGarrisonOccupant` transfer owner or only emit first-occupant cues, which owner gates/labels those cues, and how does later building reconciliation transfer/revert ownership?
- Non-goals: Do not re-study sell/ejection, weapon fire, render state, bunker logic, or broad transport behavior.
- Evidence needed to mark COMPLETE: decompile plus assembly context/ranges for `AddGarrisonOccupant`, `CheckAutoSellOrCivilian`, `BuildingClass::Update`, live object update order, first-occupant cue owner, abandoned cue owner/order, and Rust-facing surfaces.
- Stop conditions: stop once owner/cue ordering and empty revert semantics are proven for the active YR path; leave concrete runtime vector indices to debugger/runtime traces.

## 1. Overview

`BuildingClass::AddGarrisonOccupant` appends/limbos the infantry and, only when the occupant count becomes exactly `1`, performs the first-garrison mission/EVA/SFX side effects. It does not change the building owner.

Ownership transfer and empty captured-garrison revert are later `BuildingClass::Update -> CheckAutoSellOrCivilian` reconciliation effects. The transfer can occur later in the same global frame only if the infantry entry runs before the target building's update in the live `LogicClass` object-vector pass; otherwise it waits for the next target-building reconciliation pass.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|
| `Building+0x684..0x697` | occupant dynamic-vector header; `+0x688` items, `+0x694` count | `0x00522910`, `0x00458200` decompile; assembly contexts `0x00522970..0x00522992`, `0x0045830E..0x00458323` | Yes |
| `Techno+0x21C` | owner/house pointer; used for entering infantry cue gate and later first occupant transfer | assembly `0x005229A8`, `0x0045831A` | Yes |
| `Building+0x21C` | current building owner/house pointer; used for abandoned cue human gate before revert | assembly `0x0045827E..0x00458288` | Yes |
| `BuildingType+0x157B` | `CanBeOccupied`; guards `CheckAutoSellOrCivilian` call from `BuildingClass::Update` | `0x0043FB20` decompile; disassembly range `0x00440190..0x004401B7` | Conditional; Yes for stock UC buildings |
| `BuildingType+0x634` | extra `CheckAutoSellOrCivilian` gate, must equal `-1` | `0x00458200` decompile; disassembly range `0x00458200..0x00458328` | Conditional; Yes for scoped civilian-garrison path |
| vtable `+0x124` | mission set; called with `2` on first occupant | assembly `0x0052299B..0x005229A2` | Yes |
| vtable `+0x3D4` | `ChangeOwner(new_house, 0)` from reconciliation only | assembly `0x004582E6..0x004582EB`, `0x00458316..0x00458323` | Yes |

## 3. Core Logic

### 3.1 Boarding side effects

Active in YR: Yes. `BuildingClass::AddGarrisonOccupant @ 0x00522910` is reached from the infantry entry path; its xrefs include `FUN_00519710 @ 0x00519710`, and assembly at `0x00519710..0x00519733` shows the entry helper calling `BuildingClass__AddGarrisonOccupant`.

For normal `Occupier` infantry, `AddGarrisonOccupant` limbos the infantry, appends the infantry pointer to the building occupant vector, increments count, refreshes threat/power through `FUN_0070F6E0`, then checks whether `Building+0x694 == 1`. Assembly context:

- `0x00522970..0x0052297C`: reads count, increments it, writes count, writes the infantry pointer into the item array.
- `0x00522982..0x0052298D`: calls the building refresh helper chain.
- `0x00522992`: compares `Building+0x694` to `1`; if not exactly first occupant, it skips the cue block.
- `0x0052299E..0x005229A2`: calls building mission setter with `2` before the EVA/SFX gate.

Material finding: first-occupant cues are a `0->1` transition side effect of `AddGarrisonOccupant`, not a later ownership reconciliation effect. Subsequent occupants are silent in this path.

### 3.2 First-occupant EVA/SFX owner identity

Active in YR: Yes. The first-occupant cue block gates on the entering infantry owner, not the building owner. Assembly at `0x005229A8..0x005229AE` loads `ECX = [ESI + 0x21C]` before `HouseClass__IsHumanPlayer`; in this function/register context `ESI` is the infantry/occupant, while `EBP` is the building (`EBP+0x694` was the occupant count).

If that owner is human, the code calls `VoxClass__PlayEVA` at `0x005229C1`. Existing EVA research identifies this call as `EVA_StructureGarrisoned`; `ini/evamd.ini` defines it as `csof107`/`ceva107`/`cyur107` with `Type=QUEUE`, `Priority=NORMAL`.

The same gated block then calls the entering infantry object's vtable `+0x48` coordinate getter and `VocClass__PlayAt` using the configured global sound. Assembly at `0x005229C6..0x005229E0` sets `ECX=ESI`, calls `vtable+0x48`, then loads `g_RulesClass_Instance+0x1BC` and calls `VocClass__PlayAt`. `GLOBAL_SOUNDS_GHIDRA_REPORT.md` verifies `RulesClass__ReadAudioVisual @ 0x006691E0` reads `[AudioVisual] BuildingGarrisonedSound` into offset `0x1BC`; stock `rulesmd.ini` has `BuildingGarrisonedSound=BuildingGarrisoned`, and `soundmd.ini` maps that to `ugarris`.

Material finding: both first-garrison EVA and positional SFX are owner-gated by the entering infantry's owner. Current building owner may still be Civilian/Neutral at this point.

### 3.3 Ownership transfer order

Active in YR: Yes. `AddGarrisonOccupant` does not call `ChangeOwner` and has no owner write in the normal occupier path. `BuildingClass::Update @ 0x0043FB20` calls `CheckAutoSellOrCivilian @ 0x00458200` only when `BuildingType+0x157B` is nonzero; disassembly range `0x00440190..0x004401B7` covers that guard/call region.

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` walks the main object vector forward and reloads the live count after each object update. Disassembly range `0x0055B5F8..0x0055B61F` covers the forward-index loop and `vtable+0x5C` dispatch. Therefore ownership transfer is "next target-building reconciliation pass after the occupant vector mutation," not a fixed always-next-frame or always-same-frame rule.

`CheckAutoSellOrCivilian` transfer condition is `occupant_count > 0 && building_owner == resolved_civilian_house`. It calls the anim refresh helper, then reads occupant item slot `0`, reads that occupant's owner at `+0x21C`, and calls `ChangeOwner(first_occupant_owner, 0)`. Assembly at `0x0045830E..0x00458323` covers the first-slot load and `ChangeOwner` call.

### 3.4 Empty captured-garrison revert and abandoned cues

Active in YR: Yes. `CheckAutoSellOrCivilian` recomputes the Civilian-side house each invocation by calling `FUN_006A46D0` and scanning `g_HouseClass_Array` for a house whose `House->CountryType+0xBC` side index matches. No per-building original-owner field participates in this revert path.

The empty branch condition is `occupant_count == 0 && building_owner != resolved_civilian_house`. Before `ChangeOwner(civilian_house, 0)`, the branch gates cues on the current building owner: assembly at `0x0045827E..0x00458288` loads `ECX=[ESI+0x21C]` and calls `HouseClass__IsHumanPlayer`. If true, the branch plays positional audio, creates a radar event, may call `VoxClass__PlayEVA` for `EVA_StructureAbandoned` at `0x004582D8`, then calls `FUN_00458330`, then `ChangeOwner(civilian_house, 0)` at `0x004582E6..0x004582EB`.

Material finding: abandoned cue owner is the pre-revert building owner. Revert target is the resolved Civilian-side house, not a saved map-authored/original owner.

### 3.5 Normal unload ordering spot-check

Active in YR: Yes. Building mission slot 26 at `0x0044D880` starts by calling `GetOccupantCount`; if positive, it calls `BuildingClass__SellBuilding` (`0x0044D880..0x0044D8A3`). Because `BuildingClass::Update` later reaches `CheckAutoSellOrCivilian` in the same update body, normal last-occupant unload can clear cargo and then emit abandoned cues/revert during that same building update. This spot-check is only for owner/cue timing; exit-cell mechanics are out of scope.

## 4. INI Keys

| Key / source | Stock value | Binary role | Active in YR |
|---|---|---|---|
| `CanBeOccupied=` (`rulesmd.ini`) | `CAGAS01` has `CanBeOccupied=yes` | maps to `BuildingType+0x157B`, enabling building reconciliation call | Conditional; Yes for stock UC buildings |
| `MaxNumberOccupants=` (`rulesmd.ini`) | `CAGAS01` has `10` | capacity, not owner/cue timing | Conditional |
| `Occupier=` (`rulesmd.ini`) | `E1` has `Occupier=yes` | normal branch of `AddGarrisonOccupant` | Yes for stock GI |
| `[AudioVisual] BuildingGarrisonedSound` (`rulesmd.ini`) | `BuildingGarrisoned` | `RulesClass+0x1BC`, played from first-occupant block | Yes; reader `RulesClass__ReadAudioVisual @ 0x006691E0` |
| `[EVA_StructureGarrisoned]` (`evamd.ini`) | Allied `ceva107`, Russian `csof107`, Yuri `cyur107`, `QUEUE`/`NORMAL` | first-occupant EVA at `0x005229C1` | Yes |
| `[EVA_StructureAbandoned]` (`evamd.ini`) | Allied `ceva108`, Russian `csof108`, Yuri `cyur108`, `QUEUE`/`NORMAL` | empty-revert EVA at `0x004582D8` | Yes |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00519710` -> `AddGarrisonOccupant` | infantry entry commit path reaches boarding side effects | caller xref; decompile `0x00519710`; disassembly `0x00519710..0x00519733` | Yes |
| `AddGarrisonOccupant` -> `HouseClass__IsHumanPlayer`, `VoxClass__PlayEVA`, `VocClass__PlayAt` | first occupant cue gate/playback | callees of `0x00522910`; assembly `0x005229A8..0x005229E0` | Yes |
| `BuildingClass::Update` -> `CheckAutoSellOrCivilian` | later transfer/revert reconciliation | caller xref; decompile `0x0043FB20`; disassembly `0x00440190..0x004401B7` | Conditional on `CanBeOccupied`; Yes for stock UC |
| `CheckAutoSellOrCivilian` -> `ChangeOwner` | transfer and revert owner mutation | decompile `0x00458200`; disassembly `0x00458200..0x00458328` | Yes |
| `LogicClassPerTickUpdateLiveVector` | relative same-frame vs next-frame order source | decompile `0x0055AFB0`; disassembly `0x0055B5F8..0x0055B61F` | Yes |
| mission slot 26 `0x0044D880` -> `SellBuilding` | normal unload can empty cargo before same-update reconciliation | decompile `0x0044D880`; disassembly `0x0044D880..0x0044D8A3` | Yes when unload mission runs |

## 6. Current Rust Implementation Status

Current Rust has a live-order surrogate in `src/sim/passenger.rs:354..390`: `tick_passenger_system` gets `Simulation::live_object_order_snapshot()`, processes boarding/unloading for an entity, then reconciles that same entity if it is a civilian garrison building. This matches the scoped "building reconciles on its own turn after cargo mutation" model for pre-existing passenger/building relative-order tests.

Current Rust emits first-occupant `StructureGarrisoned` and `BuildingGarrisonedSfx` in `process_boarding_passenger` using the building owner and building tile (`src/sim/passenger.rs:456..475`). The duplicate boarding code path at `src/sim/passenger.rs:700..724` does the same. Native gates these first-occupant cues on the entering infantry owner (`0x005229A8`) and gets the position from the entering infantry object (`0x005229CE..0x005229D0`). For a neutral building, using building owner can suppress local-player EVA/SFX in the app layer before the later owner transfer.

Current Rust empty revert in `reconcile_civilian_garrison_owner_for_building` emits `StructureAbandoned { owner: current_owner }` before writing the resolved civilian owner (`src/sim/passenger.rs:602..610`), which matches the native pre-revert owner order. Its resolver chooses `Neutral`/`Special` by name (`src/sim/passenger.rs:514..526`), which is still a mechanism approximation of native side/house scanning.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AddGarrisonOccupant @ 0x00522910` normal occupier branch | verified | decompile; assembly `0x00522970..0x005229E0`; callees | none for owner/EVA/SFX timing |
| first-occupant cue owner gate | verified | assembly `0x005229A8..0x005229AE` | none |
| first-occupant cue sound source | verified | assembly `0x005229D5..0x005229E0`; `GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `rulesmd.ini` | none |
| `BuildingClass::Update` reconciliation call | verified | decompile; disassembly `0x00440190..0x004401B7` | none for this slice |
| live object order same-frame/next-frame source | verified | decompile `0x0055AFB0`; disassembly `0x0055B5F8..0x0055B61F`; prior trace docs | concrete retail indices not sampled |
| `CheckAutoSellOrCivilian @ 0x00458200` transfer branch | verified | decompile; assembly `0x0045830E..0x00458323` | none |
| `CheckAutoSellOrCivilian` empty branch cues/revert | verified | decompile; assembly `0x0045827E..0x004582EB`; EVA docs | none for owner/order |
| mission slot 26 unload before reconciliation | touched-not-exhausted | decompile `0x0044D880`; assembly `0x0044D880..0x0044D8A3` | exit-cell/ejection mechanics out of scope |
| sell/destruction ejection | deferred | user non-scope | covered by slot 3 |
| fire/render/bunker behavior | deferred | user non-scope | separate slots |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does AddGarrisonOccupant transfer owner? -> No; no ChangeOwner call in `0x00522910`, owner mutation is in `0x00458200`.` (evidence: `0x00522910`, `0x00458200`)
- `[RESOLVED] OQ-02 - When does first-garrison EVA/SFX fire? -> Only when count becomes exactly 1 after append.` (evidence: `0x00522970..0x00522992`)
- `[RESOLVED] OQ-03 - Which owner gates first-garrison EVA/SFX? -> Entering infantry owner at `Techno+0x21C`.` (evidence: `0x005229A8..0x005229AE`)
- `[RESOLVED] OQ-04 - Which audio assets are used? -> `EVA_StructureGarrisoned` and `[AudioVisual] BuildingGarrisonedSound` from `RulesClass+0x1BC`.` (evidence: `0x005229C1`, `0x005229D5..0x005229E0`, `0x006691E0`, `evamd.ini`, `rulesmd.ini`)
- `[RESOLVED] OQ-05 - Does AddGarrisonOccupant create a radar event for StructureGarrisoned? -> No CreateRadarEvent callee or call in `0x00522910`.` (evidence: callees of `0x00522910`)
- `[RESOLVED] OQ-06 - Where does occupied civilian transfer happen? -> `CheckAutoSellOrCivilian` transfer branch, called from building update.` (evidence: `0x0043FB20`, `0x00458200`)
- `[RESOLVED] OQ-07 - Which occupant supplies transfer owner? -> occupant slot 0 owner.` (evidence: `0x0045830E..0x00458323`)
- `[RESOLVED] OQ-08 - Is transfer always next frame? -> No; it is next target-building reconciliation pass, same global frame if building update is later in the live object vector.` (evidence: `0x0055AFB0`, `0x0055B5F8..0x0055B61F`)
- `[RESOLVED] OQ-09 - Which owner gates abandoned EVA/SFX? -> Current building owner before revert.` (evidence: `0x0045827E..0x00458288`)
- `[RESOLVED] OQ-10 - Does abandoned cue happen before or after ChangeOwner? -> Before ChangeOwner.` (evidence: `0x0045827E..0x004582EB`)
- `[RESOLVED] OQ-11 - What is the empty revert target? -> resolved Civilian-side house from side/house scan, not stored original owner.` (evidence: `0x00458200`, `FUN_006A46D0`)
- `[RESOLVED] OQ-12 - Is the path active in standard YR? -> Yes for stock `CanBeOccupied=yes` civilian buildings and `Occupier=yes` infantry.` (evidence: `rulesmd.ini`, `0x0043FB20`, `0x00519710`)
- `[DEFERRED] OQ-13 - What are concrete object-vector indices for a specific stock map entry?` (category: `needs-runtime-debugger`; reason: static decompile proves mechanism but not one scenario's runtime vector indices; next-step-if-pursued: log `LogicClass` vector indices at `0x0055B5F8..0x0055B61F`)
- `[DEFERRED] OQ-14 - Does native first-garrison SFX position ever differ visibly from building tile after limbo?` (category: `needs-runtime-debugger`; reason: assembly shows the entering infantry coordinate getter; valid entry should be at building cell, but exact post-limbo coordinate value was not sampled; next-step-if-pursued: runtime watch coordinate returned at `0x005229D0`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First occupant `0->1` cues run inside `AddGarrisonOccupant`, before owner transfer, and are gated by entering infantry owner. | decompile `0x00522910`; assembly `0x00522992..0x005229E0`; `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` | Rust emits `StructureGarrisoned`/`BuildingGarrisonedSfx` with building owner at `src/sim/passenger.rs:467..474` and `714..724` | `src/sim/passenger.rs`, `src/app_sim_tick.rs` local-owner gate | Queue first-garrison EVA/SFX with entering passenger owner, not current building owner; preserve first-occupant-only gating. | Neutral CAGAS01 first occupied by local GI should emit `StructureGarrisoned` and `BuildingGarrisonedSfx` for `Americans` before the building owner changes. Proposed test: `garrison_first_occupant_cues_use_entering_infantry_owner` | Do not gate first-garrison cues on Neutral/Special building owner; that suppresses the player cue. |
| Ownership transfer is later building reconciliation: `count > 0 && owner == CivilianHouse` -> `ChangeOwner(slot0.owner, 0)`. | decompile `0x0043FB20`, `0x00458200`; assembly `0x0045830E..0x00458323`; scheduler `0x0055B5F8..0x0055B61F` | Rust live-order surrogate now models relative-order pass; concrete runtime order equality remains only partially proven | `src/sim/passenger.rs`, `src/sim/world/mod.rs::live_object_order_snapshot` | Keep owner mutation out of boarding; building owner changes only when that building's reconciliation turn runs after cargo mutation. | Entry before building update transfers same pass; building update before entry waits one pass. Proposed test: `garrison_owner_transfer_follows_building_reconciliation_turn` | Do not implement a fixed one-tick delay or unconditional same-frame transfer. |
| Empty captured garrison abandoned cues are emitted for pre-revert building owner, then owner reverts to resolved Civilian-side house. | decompile `0x00458200`; assembly `0x0045827E..0x004582EB`; `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` | Rust emits pre-revert owner event, but civilian resolver is a Neutral/Special name approximation | `src/sim/passenger.rs::reconcile_civilian_garrison_owner_for_building`, house/side model | Preserve pre-revert event owner; replace stored-original/name preference with Civilian-side house resolution when side/country model is available. | Captured building emptied by local player should play `EVA_StructureAbandoned` for that player and then become Civilian-side owner. Proposed test: `empty_captured_garrison_abandoned_event_precedes_civilian_revert` | Do not restore `garrison_original_owner` as native revert semantics. |

## 10. Negative Facts / Do Not Do

- Do not use current building owner for first-garrison EVA/SFX. Active in YR: Yes. Evidence: `0x005229A8` loads entering infantry owner before `HouseClass__IsHumanPlayer`.
- Do not fire `StructureGarrisoned` for every occupant. Active in YR: Yes. Evidence: count compare to exactly `1` at `0x00522992`.
- Do not transfer owner in `AddGarrisonOccupant`. Active in YR: Yes. Evidence: no `ChangeOwner` callee in `0x00522910`; transfer branch is `0x0045830E..0x00458323`.
- Do not add a `StructureGarrisoned` radar event based on `AddGarrisonOccupant`; the function does not call `CreateRadarEvent`. Active in YR: Yes. Evidence: callees of `0x00522910` are `FUN_0070F6E0`, `HouseClass__IsHumanPlayer`, `SpawnUnitsWithParachute`, `VocClass__PlayAt`, and `VoxClass__PlayEVA`.
- Do not treat `garrison_original_owner` as native revert state. Active in YR: Yes. Evidence: `CheckAutoSellOrCivilian` resolves Civilian side/house each invocation and calls `ChangeOwner(civilian_house, 0)`.

## 11. Stale Docs / Follow-up Docs

- `docs/research/BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` replacement wording for first-garrison cue owner: "The first `AddGarrisonOccupant` cue block gates `EVA_StructureGarrisoned` and `BuildingGarrisonedSound` on the entering infantry's owner (`Techno+0x21C`), not on the building owner. The building owner may still be Civilian/Neutral until later `CheckAutoSellOrCivilian` reconciliation."
- `docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording for StructureGarrisoned radar wording: "`AddGarrisonOccupant` plays `EVA_StructureGarrisoned` and the positional `BuildingGarrisonedSound` for a human entering-infantry owner when count becomes 1; it does not call `CreateRadarEvent` in this first-garrison cue path."
- `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` replacement wording for frame delay: "Ownership transfer occurs on the target building's next `BuildingClass::Update -> CheckAutoSellOrCivilian` reconciliation pass after the occupant vector mutation. That pass can be later in the same global frame or in a later frame depending on live `LogicClass` object-vector order."

## 12. Remaining Uncertainty

- Concrete retail map/replay object-vector indices for a specific infantry/building pair were not runtime-sampled. The mechanism is verified; instance-specific same-frame vs next-frame outcome still needs debugger logging if required.
- The exact coordinate returned by the entering infantry vtable `+0x48` after limbo was not runtime-sampled. Assembly proves source object for the SFX position; valid garrison entry should already be at the target building cell.

## Sources

- Ghidra read-only decompile: `0x00522910`, `0x00458200`, `0x0043FB20`, `0x0055AFB0`, `0x00519710`, `0x0044D880`.
- Ghidra read-only xrefs/callees: callers of `0x00522910`, callers/callees of `0x00458200`, callees of `0x00522910`.
- Ghidra read-only assembly context/ranges: `0x00522970..0x005229E0`, `0x0045827E..0x00458323`, `0x00440190..0x004401B7`, `0x0055B5F8..0x0055B61F`, `0x00519710..0x00519733`, `0x0044D880..0x0044D8A3`.
- Existing docs used as maps/corroboration: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`, `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `traces/GARRISON_OWNER_LIVE_OBJECT_ORDER_POSTFIX_TRACE.md`, `traces/GARRISON_ABANDONED_EVA_PLAYBACK_POSTFIX_TRACE.md`, `traces/EMPTY_CAPTURED_GARRISON_REVERT_ABANDONED_TIMING_TRACE.md`, `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md`, `GLOBAL_SOUNDS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/evamd.ini`, `ini/soundmd.ini`.
- Rust scanned read-only: `src/sim/passenger.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/app_building_anim.rs`, `src/rules/ruleset.rs`.

## Status

COMPLETE for the scoped AddGarrisonOccupant owner/EVA/SFX timing, later ownership reconciliation order, and empty captured-garrison revert owner semantics. No Rust, INI, or tracked docs outside `docs/research/` were modified.
