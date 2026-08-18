# BuildingClass Damage-Fire Selector/RNG - Ghidra Research Report

**Address(es):** `0x0043FB20`, `0x0043C0D0`, `0x004600CB..0x004600DF`, `0x0045E07D`, `0x0065C7E0`  
**Investigation Mode:** exhaustive-slice for the requested damage-fire selector/RNG surface  
**Scope:** `BuildingClass::Update` damage-fire threshold selector, `BuildingClass::CreateDamageFireAnims` RNG/slot write contract, and the `BuildingTypeClass+0x157B` parser/default source.  
**Non-Scope:** render depth, full `AnimClass::DrawIt`, destruction debris, `CreateFireAnim`, terrain fire, or generic `AnimClass` lifecycle.  
**Confidence:** High for selector source, threshold choice, RNG call order/bounds, slot scan/stop, and writes. Medium only for literal scenario RNG outputs because no live runtime state snapshot was captured.  
**Active in YR:** Yes for ordinary building update and damaged-building fire; conditional on building health crossing the selected threshold and `[General] DamageFireTypes` count being nonzero.

Working notes required by swarm prompt:

- Target question: What exact active-YR selector/default/parser and RNG/slot-write contract drives persistent building damage-fire creation?
- Non-goals: Do not investigate render depth, destruction debris, terrain fire, generic `AnimClass` lifecycle, or unrelated garrison behavior except the shared `+0x157B` byte identity.
- Evidence needed to mark COMPLETE: Binary proof for `+0x157B` default/parser/use, binary proof for yellow/red threshold selection, binary proof for `DamageFireTypes` and per-slot frame RNG bounds/order, binary proof for slot scan stop/write behavior, INI/default evidence, Rust deltas, and implementation handoff.
- Stop conditions: Stop after the selector identity and RNG/slot contract are proven; record concrete RNG-output capture as remaining uncertainty if no live scenario RNG state is sampled.

## 1. Executive Summary

`BuildingTypeClass+0x157B` is not a second damage-fire-specific field. It is the `CanBeOccupied=` byte: constructor default false, parsed from the `CanBeOccupied` INI key, and used by garrison systems. `BuildingClass::Update` also reuses this exact byte as the damage-fire threshold selector: `0` selects `ConditionYellow`; nonzero selects `ConditionRed`.

That dual use is active in standard YR. Ordinary non-garrisonable buildings such as stock `GACNST` keep default `CanBeOccupied=0` and spawn persistent damage fires at yellow health. `CanBeOccupied=yes` buildings spawn persistent damage fires only at red health, because the same byte selects `ConditionRed`.

`CreateDamageFireAnims` consumes one scenario RNG draw for the initial `DamageFireTypes` index, then creates all valid contiguous empty fire slots in one call. Each successful constructed anim consumes one additional scenario RNG draw for its starting frame only when the chosen `AnimType` frame count is positive. It stores real `AnimClass*` pointers into `BuildingClass+0x5C8 + slot*4`, not app-side overlays.

## 2. Verified Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `BuildingTypeClass+0x157B` defaults to false/zero. | Constructor assembly `0x0045E07D`: `MOV byte ptr [ESI + 0x157b], BL`; surrounding constructor writes show `BL` is the zero/default byte used for nearby bools. | Yes. Every `BuildingTypeClass` construction gets this default before INI reads. |
| `BuildingTypeClass+0x157B` is parsed from the `CanBeOccupied` key, preserving the previous value as the default argument to `ReadBool`. | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; assembly `0x004600CB..0x004600DF`: load `[EBP+0x157B]`, push string `0x81ADBC` (`s_CanBeOccupied`), call `CCINIClass__ReadBool @ 0x005295F0`, store `AL` back to `[EBP+0x157B]`. | Yes. Standard YR building type INI reader. |
| `BuildingClass::Update` uses the same byte as damage-fire threshold selector. Zero uses `Rules+0x1700` (`ConditionYellow`); nonzero uses `Rules+0x1708` (`ConditionRed`). | Decompile `0x0043FB20`; assembly `0x0043FC39..0x0043FC82`: reads `Type+0x157B`, branches to `FCOMP [Rules+0x1700]` for zero and `FCOMP [Rules+0x1708]` for nonzero. | Yes. This is in the normal per-building update. |
| Threshold comparison is inclusive for spawning/removal state: the damaged flag becomes true when `GetHealthRatio() <= selected_threshold`; false when ratio is greater. | Decompile `0x0043FB20`; assembly compare windows `0x0043FC4B..0x0043FC64` and `0x0043FC66..0x0043FC82` set `BL=1` only when the FPU comparison does not take the greater-than skip to `XOR BL,BL`. | Yes. Applies each update before later garrison reconciliation and repair/power phases. |
| Damage-fire active state is cached at `BuildingClass+0x5E8`; false-to-true calls `CreateDamageFireAnims`, true-to-false loops 8 slots and calls vtable `+0xF8`, then clears pointers. | Decompile `0x0043FB20`; damage-fire block after selector, slot clear loop over `this+0x5C8..+0x5E4`. | Yes. Normal update and health transition path. |
| If `Rules+0x2B0` (`DamageFireTypes` count) is zero, `CreateDamageFireAnims` returns before any RNG draw or slot scan. | Decompile `0x0043C0D0`; assembly `0x0043C0DB..0x0043C0EA`: load count, test, jump to return if zero. | Yes. Conditional on modded/loaded rules data. |
| Initial fire type RNG draw is exactly `RandomRanged(0, count - 1)` on the scenario RNG at `(*0x00A8B230)+0x218`. | Decompile `0x0043C0D0`; assembly `0x0043C0F0..0x0043C100`: load scenario pointer, `DEC EAX`, push upper, push `0`, add `0x218`, call `0x0065C7E0`. | Yes when count is nonzero. |
| Slot scan starts at `BuildingClass+0x5C8`, type offset `BuildingTypeClass+0x15D8`, and advances both by one slot until the type offset reaches `0x1618` (8 slots total). | Decompile `0x0043C0D0`; assembly `0x0043C105..0x0043C118` initializes slot pointer and `0x15D8`; loop advances by `+4` slot pointer and `+8` type offset until `< 0x1618`. | Yes. |
| Slot scan stops immediately at the first sentinel pair read from `DAT_0089C848/DAT_0089C84C`. It does not skip gaps. | Assembly `0x0043C11E..0x0043C137`: compare X to `0x0089C848`, compare Y to `0x0089C84C`, jump to return if both match. Parser missing `DamageFireOffsetN` writes sentinel from `DAT_0089C8D0/DAT_0089C8D4` in `BuildingTypeClass_ReadINI_Water`. | Yes. Standard contiguous offset contract. |
| Slot scan also stops immediately at the first occupied fire slot. It does not continue to fill later empty slots. | Assembly `0x0043C13D..0x0043C140`: `CMP dword ptr [EDI],0`; `JNZ` return. | Yes. Prevents partial refill beyond an existing slot. |
| Constructor arguments for each damage-fire slot are `AnimType`, coord, `delay=0`, `loop=1`, `drawFlags=0x600`, `facing=0`, `z=0`; after successful construction the pointer is stored to the current slot. | Assembly `0x0043C1B4..0x0043C1DC` pushes `0,0,0x600,1,0,&coord,type` and calls `AnimClass__Constructor @ 0x00421EA0`; `0x0043C1E1..0x0043C1EB` stores returned pointer to `[EDI]`. | Yes when allocation/constructor succeeds. |
| Per-slot start-frame RNG draw is exactly `RandomRanged(0, frame_count - 1)` using the constructed anim's type frame count at `AnimType+0x2C0`; no draw occurs when frame count is `<= 0`. | Assembly `0x0043C237..0x0043C25B`: load `Anim+0xC8`, read `[type+0x2C0]`, `TEST/JLE`, decrement upper, call `0x0065C7E0` on scenario RNG `+0x218`, store result at `Anim+0xAC`. | Yes for positive-frame anim types. |
| Fire type selection wraps sequentially after each successful constructed slot, not by per-slot random type. | Decompile `0x0043C0D0`: `local_2c++`; if `local_2c >= count`, reset to zero. The increment is inside the successful constructor branch. | Yes. Failed allocation/constructor advances to the next offset/slot without consuming frame RNG or advancing type index. |
| `RandomRanged @ 0x0065C7E0` uses inclusive sorted bounds, equal-bounds no-draw, and mask/rejection sampling over the scenario RNG, not modulo/exclusive bounds. | Decompile `0x0065C7E0`; disassembly range `0x0065C7E0..0x0065C88A`; prior exhaustive report `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`. | Yes. Representative gameplay callers and this path use scenario RNG `+0x218`. |

## 3. Concrete Retail Data

| Data | Evidence | Active in YR |
|---|---|---|
| `[General] DamageFireTypes=FIRE01,FIRE02,FIRE03`, count 3 in stock YR data. | `ini/rulesmd.ini:519`; consumed by `Rules+0x2A4/+0x2B0` in `0x0043C0D0`. | Yes. |
| `ConditionRed=25%`, `ConditionYellow=50%`. | `ini/rulesmd.ini:752-753`; consumed from `Rules+0x1708/+0x1700` in `0x0043FB20`. | Yes. |
| Stock `GACNST` has no `CanBeOccupied=` override in its `rulesmd.ini` section and therefore uses default `+0x157B=0`. | `ini/rulesmd.ini:11622-11652`; constructor default `0x0045E07D`; parser only changes the byte when the key is present. | Yes. `GACNST` uses yellow threshold. |
| Stock civilian/garrisonable buildings do set `CanBeOccupied=yes`, e.g. active sections around `rulesmd.ini:13002`, `14108`, `19322`. | INI grep and parser `0x004600CB..0x004600DF`. | Conditional by building type; Yes for those stock buildings. These use red threshold for persistent damage fires. |
| Stock `GACNST` has two contiguous damage-fire offsets: `(-24,-1)` and `(64,36)`, then no third offset in that section. | `ini/artmd.ini:1599-1621`; parser sentinel on missing next offset; create scan stops at the first sentinel after slot 1. | Yes. |
| `FIRE01/FIRE02/FIRE03` have `Rate=450` and `LoopCount=-1`. | `ini/artmd.ini:16018-16034`; frame count still comes from loaded SHP/AnimType metadata at `AnimType+0x2C0`. | Yes. |

## 4. Open Questions Log - Final State

- `[RESOLVED] OQ-01 - Is `+0x157B` really `CanBeOccupied` or an unrelated damage-fire selector? -> It is parsed from `CanBeOccupied=` and is also reused as the damage-fire selector.` Evidence: parser `0x004600CB..0x004600DF`; update selector `0x0043FC39..0x0043FC82`.
- `[RESOLVED] OQ-02 - What is the default selector value? -> Zero/false.` Evidence: constructor write `0x0045E07D`.
- `[RESOLVED] OQ-03 - Which threshold does each value select? -> zero selects `ConditionYellow`; nonzero selects `ConditionRed`.` Evidence: `0x0043FB20` decompile and assembly compare windows.
- `[RESOLVED] OQ-04 - Does the threshold compare include equality? -> Yes, ratio equal to the selected threshold is damaged/active.` Evidence: decompile branch and FPU compare skip only for ratio greater than threshold.
- `[RESOLVED] OQ-05 - What is the initial fire type RNG call? -> one scenario RNG call `RandomRanged(0,count-1)` before slot scan, only when count > 0.` Evidence: `0x0043C0DB..0x0043C100`.
- `[RESOLVED] OQ-06 - What are per-slot frame RNG bounds/order? -> after successful constructor and zAdjust write, if frame count > 0, call `RandomRanged(0,frame_count-1)` and store to `Anim+0xAC`.` Evidence: `0x0043C237..0x0043C25B`.
- `[RESOLVED] OQ-07 - Does scan skip invalid/occupied slots? -> No. It returns at first sentinel or first occupied slot.` Evidence: `0x0043C11E..0x0043C140`.
- `[RESOLVED] OQ-08 - Are later fire types independently random? -> No. Only the initial type index is random; successful slots increment and wrap.` Evidence: `0x0043C0D0` decompile.
- `[DEFERRED] OQ-09 - What exact numeric fire-type/start-frame outputs occur in a particular live match seed?` Category: runtime-state-capture. Reason: static binary proves call order/bounds/state owner, but no live debugger snapshot of `Scenario+0x218` before a concrete spawn was captured in this slot.

## 5. Current Rust Status

| Rust surface | Status vs verified behavior |
|---|---|
| `src/app_building_anim.rs::damage_fire_threshold_for_current_surface` | Drift: always returns `ConditionYellow` because raw `+0x157B`/`CanBeOccupied` is not threaded into this surface. Garrisonable buildings should use `ConditionRed`. |
| `src/rules/object_type.rs::can_be_occupied` | Directionally correct parser surface for the byte; implementation should reuse this value as the damage-fire selector instead of inventing a new field. |
| `src/app_building_anim.rs::create_damage_fire_slot_anims` | Mostly matches call order at the overlay level: one initial type draw through exclusive `next_range_u32(count)` equals native `0..count-1`, then one frame draw through exclusive `next_range_u32(total_frames)` equals native `0..frame_count-1`. Still not native `AnimClass` slot objects. |
| `src/sim/rng.rs` | Current file has a parity-shaped 250-word RNG and inclusive sorted helper. Keep damage-fire calls on the shared sim/scenario RNG stream. |
| `src/rules/art_data.rs::damage_fire_offsets` | Parses present offsets into a vector. Needs care for native sentinel/occupied-stop semantics when moving to native slots: do not scan/fill past a first missing/sentinel slot. |
| `src/sim/components.rs::DamageFireOverlays` | Still app-side overlays; native stores `AnimClass*` at `BuildingClass+0x5C8..+0x5E4`. |

## 6. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `CanBeOccupied=` is the actual `+0x157B` byte and nonzero selects `ConditionRed`; zero selects `ConditionYellow`. | Thread parsed `object.can_be_occupied` into the damage-fire threshold decision; do not add a new damage-fire selector field. | `src/app_building_anim.rs`, `src/rules/object_type.rs`, damage-fire tests | `GACNST` at exactly 50% health spawns fires; stock `CAGAS01`/another `CanBeOccupied=yes` building at 50% does not spawn until 25%. | `damage_fire_threshold_uses_can_be_occupied_selector` | High: current yellow fallback spawns garrisonable-building fires too early and consumes RNG too early. |
| Initial type RNG is one `RandomRanged(0,count-1)` before slot scan; each successful constructed slot with positive frame count draws `RandomRanged(0,frame_count-1)` after construction/zAdjust, then type index increments/wraps. | Preserve the exact order on the shared scenario RNG when replacing overlays with native anim slots. Avoid per-slot fire-type RNG or deterministic slot modulo. | `src/app_building_anim.rs`, future generic `AnimClass` runtime, `src/sim/rng.rs` | With three fire types and two valid offsets, a known RNG state consumes exactly three ranged calls in order: initial type, slot 0 frame, slot 1 frame; selected slot 1 type is initial+1 modulo count. | `damage_fire_rng_consumes_initial_type_then_slot_frames` | High: one extra/missing draw desyncs later gameplay-visible randomness. |
| Slot creation returns on first sentinel offset or first occupied fire slot; it does not fill gaps or later empty slots. | Model native 8 slots and contiguous scan/early-return semantics, not vector "all known offsets" fill behavior once true native slots exist. | `src/rules/art_data.rs`, `src/app_building_anim.rs`, future `BuildingClass` slot storage | A building with offset 0 valid, offset 1 missing, offset 2 present in data creates only slot 0; a building with slot 0 already occupied creates no additional later slots. | `damage_fire_slot_scan_stops_at_first_sentinel_or_occupied_slot` | Medium/high: mods or malformed data will drift; occupied-slot partial refill changes lifecycle and RNG consumption. |
| True-to-false damage-fire transition calls vtable `+0xF8` on all 8 non-null slots and clears them; it does not destroy only visible/vector entries. | Future native runtime should uninit exact slots and clear all eight pointers on repair above selected threshold and destruction cleanup. | future generic `AnimClass` pool, `src/app_building_anim.rs`, `src/sim/components.rs` | Repair a garrisonable building from 25% to just above 25% clears all native fire slots; repair from 50% to 49% remains active when `CanBeOccupied=yes`. | `damage_fire_repair_clear_uses_selected_threshold_and_all_slots` | Medium: lifecycle leaks or premature removal change sounds/draw/order. |

## 7. Negative Facts / Do Not Do

- Do not create a separate `DamageFireConditionRed`/`UseRedDamageFire` INI field. Evidence: parser writes `CanBeOccupied=` into `+0x157B`, and update reads the same byte. Active in YR: Yes.
- Do not treat docs tying `+0x157B` to garrison as contradictory by itself. The byte is `CanBeOccupied`; the surprising fact is that damage-fire threshold selection reuses it. Active in YR: Yes.
- Do not always use `ConditionYellow`. `CanBeOccupied=yes` buildings use `ConditionRed`. Active in YR: Conditional on INI; stock civilian garrison buildings trigger it.
- Do not choose a random `DamageFireTypes` entry for each slot. Native chooses one random starting index, then increments/wraps on successful slot construction. Active in YR: Yes.
- Do not scan past the first missing/sentinel `DamageFireOffsetN` or first occupied native fire slot. Active in YR: Yes.
- Do not consume any damage-fire RNG when `DamageFireTypes` count is zero, when bounds are equal, or for a non-positive frame count. Active in YR: Conditional on data/frame count.
- Do not implement this as a render-path lazy overlay spawn. The owner is `BuildingClass::Update`; native objects are real `AnimClass` slots. Active in YR: Yes.

## 8. Remaining Uncertainty

- No live debugger capture was taken for a concrete pre-spawn `Scenario+0x218` state, so this report verifies deterministic RNG order/bounds/state owner but does not list literal numeric fire-type/start-frame outputs for a particular match seed.
- Exact native behavior if `AnimClass__Constructor` returns null is verified from decompile only at the control-flow level: no slot write, no frame RNG, no type-index increment, loop advances to next slot. Allocation-failure is not expected in normal stock play but is active helper behavior.

## 9. Stale Docs / Replacement Wording

- `docs/research/BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`: replace "`Type+0x157B` (bool) - damage-fire threshold selector / field label unresolved; older docs disagree on the INI label" with "`Type+0x157B` (bool) - parsed from `CanBeOccupied=` by `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; `BuildingClass::Update` also reuses the same byte as the damage-fire threshold selector: zero uses `ConditionYellow`, nonzero uses `ConditionRed`."
- `docs/research/ANIMCLASS_BUILDING_OBJECT_DAMAGE_RUNTIME_SPAWNS_GHIDRA_REPORT.md`: replace "`+0x157B` | `BuildingTypeClass` | damage-fire threshold selector | ... | Yes, label uncertain" with "`+0x157B` | `BuildingTypeClass` | `CanBeOccupied=` byte, reused as damage-fire threshold selector | parser `0x004600CB..0x004600DF`; update selector `0x0043FC39..0x0043FC82` | Yes; zero = `ConditionYellow`, nonzero = `ConditionRed`."
- `docs/research/DAMAGE_FIRE_ANIMS_GHIDRA.md`: after the threshold discussion, add "Fresh selector audit confirms `BuildingType+0x157B` is the parsed `CanBeOccupied=` byte, not an independent damage-fire field. This means stock garrisonable `CanBeOccupied=yes` buildings use `ConditionRed` for persistent damage-fire activation."

## 10. Sources

- Ghidra read-only decompile: `BuildingClass::Update @ 0x0043FB20`; assembly context `0x0043FC39..0x0043FC82`.
- Ghidra read-only decompile: `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`; assembly contexts `0x0043C0DB..0x0043C100`, `0x0043C105..0x0043C140`, `0x0043C1B4..0x0043C25B`.
- Ghidra read-only decompile: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; parser assembly context `0x004600CB..0x004600DF`.
- Ghidra read-only assembly context: `BuildingTypeClass` constructor default write `0x0045E07D`.
- Ghidra read-only decompile/disassembly range: `Random::RandomRanged @ 0x0065C7E0..0x0065C88A`.
- INI evidence: `ini/rulesmd.ini:519`, `752-753`, `11622-11652`, `13002`, `14108`, `19322`; `ini/artmd.ini:1599-1621`, `16018-16034`.
- Rust evidence: `src/app_building_anim.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`, `src/sim/rng.rs`, `src/sim/components.rs`.

## Status

COMPLETE for the requested selector/default/RNG/slot-write contract.
