# TiberiumClass Growth/Spread Driver Timers and Gates - Ghidra Research Report

**Address(es):** `0x007221B0`, `0x00722C40`, `0x0055AFB0`, `0x007216C0`, `0x00721A50`, `0x00689E90`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** active standard-YR driver behavior for `TiberiumClass::SpreadDriver_AllTypes` and `TiberiumClass::GrowthDriver_AllTypes`: tick integration, scenario gates, timer start/interval fields, first-fire behavior, reload math, per-type iteration order, and INI/map fields needed for those claims.  
**Non-Scope:** growth/spread processor internals, queue heap/bitmap ownership, `CanGrowTiberium`/`CanSpreadTiberium` predicates, native queue save/load, map-load queue seeding, TIBTRE placement/damage behavior.  
**Confidence:** High for driver gates, timer field offsets, first-fire behavior, reload formulas, and tick order; Medium for the semantic purpose of timer middle words `+0x104/+0x120`, which are written by the driver but not proven consumed in this slice.  
**Active in YR:** Yes, conditional on `ScenarioClass+0x34A6` and the current per-type timer state. `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls both drivers every live logic tick.

## Working Notes

Target question: Verify exact active standard-YR driver behavior for growth/spread queue timers and gates.  
Non-goals: Do not re-investigate processor pop/batch semantics, queue duplicate behavior, queue save/load rebuild, map-load seeding, or TIBTRE placement/damage facts except where needed for driver call contracts.  
Evidence needed to mark COMPLETE: decompile and assembly for both drivers; live caller proof from the logic tick; constructor timer writes; `TiberiumClass::ReadINI` offsets for `Spread`/`Growth`; map `[Basic] TiberiumGrowthEnabled` writer; INI stock values; Rust touchpoint scan.  
Stop conditions: Stop after the driver timer/gate model is implementation-ready and list adjacent processor/save-load details as out of scope.

## 1. Overview

The standard YR live logic tick calls all growth drivers before any spread driver. Each driver first checks the map-level byte `ScenarioClass+0x34A6` (`[Basic] TiberiumGrowthEnabled`). If that byte is zero, neither driver iterates any `TiberiumClass`.

The drivers are not map scanners. They iterate `g_TiberiumClass_Array` in ascending index order and use per-type CDTimer-like fields. Spread reloads its interval directly from the type's `Spread=` integer. Growth reloads from `ftol(Growth * multiplier)`, where the multiplier is `0.3` when SpecialFlags bit `0x40` is set and `1.0` when that bit is clear.

## 2. Key Offsets

| Owner | Offset | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `ScenarioClass` | first dword bit `0x40` | SpecialFlags `TiberiumGrows`; growth interval multiplier selector, not the driver on/off gate. | Conditional; stock/default paths set this bit. | `0x00722CA4..0x00722CB7`; `SPECIAL_FLAGS_SYSTEM.md`; `rulesmd.ini:44` |
| `ScenarioClass` | `+0x34A6` | `[Basic] TiberiumGrowthEnabled`; hard gate for both all-type drivers. | Conditional; stock maps/default templates use enabled. | `0x007221B8..0x007221C1`, `0x00722C48..0x00722C51`, `0x0068A589` |
| `TiberiumClass` | `+0x9C` | `Spread=` interval read from INI and reloaded by spread driver. | Yes. | `0x00721A50`, `0x0072221C..0x00722227` |
| `TiberiumClass` | `+0xA8` | `Growth=` base interval read from INI and multiplied by growth driver. | Yes. | `0x00721A50`, `0x00722CC6..0x00722CCE` |
| `TiberiumClass` | `+0x100` | spread timer last/start frame. | Yes. | constructor `0x0072176A..0x00721776`; driver `0x007221DD` |
| `TiberiumClass` | `+0x104` | spread timer middle word, written on reload. Semantic consumer not proven here. | Yes as state write; consumer deferred. | `0x00722224` |
| `TiberiumClass` | `+0x108` | spread timer interval. | Yes. | constructor `0x00721776`; driver `0x007221E3`, `0x00722227` |
| `TiberiumClass` | `+0x11C` | growth timer last/start frame. | Yes. | constructor `0x00721794..0x007217A0`; driver `0x00722C76`, `0x00722CDE` |
| `TiberiumClass` | `+0x120` | growth timer middle word, written on reload. Semantic consumer not proven here. | Yes as state write; consumer deferred. | `0x00722CE0` |
| `TiberiumClass` | `+0x124` | growth timer interval. | Yes. | constructor `0x0072179A`; driver `0x00722C7C`, `0x00722CE3` |

## 3. Core Logic

### 3.1 Live tick integration

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` reaches the ore queue drivers at the fixed live-tick point:

1. `CALL 0x00722C40` at `0x0055B4D7` -> `TiberiumClass::GrowthDriver_AllTypes`.
2. `CALL 0x007221B0` at `0x0055B4DC` -> `TiberiumClass::SpreadDriver_AllTypes`.
3. `BombClass::UpdateAll` and later object updates follow.

Active in YR: Yes. This function is the standard live logic vector update; the calls are unconditional once execution reaches the normal tail block.

### 3.2 Shared driver gate and iteration order

Both drivers start by reading `g_ScenarioClass_Instance + 0x34A6`.

Spread assembly:

- `0x007221B0`: load scenario pointer.
- `0x007221B8`: read byte `[EAX + 0x34A6]`.
- `0x007221BF..0x007221C1`: `TEST CL,CL`; `JZ 0x00722236`.

Growth assembly:

- `0x00722C40`: load scenario pointer.
- `0x00722C48`: read byte `[EAX + 0x34A6]`.
- `0x00722C4F..0x00722C51`: `TEST CL,CL`; `JZ 0x00722CF7`.

If the byte is zero, the driver exits before reading `g_TiberiumClass_Array_Count`. If nonzero, both drivers iterate `ESI = 0..g_TiberiumClass_Array_Count-1` and fetch the current class pointer from `g_TiberiumClass_Array + ESI*4`.

Tiny detail: neither driver checks per-type percentage before calling the processor. Percentage gates live inside the processors, not these drivers. This matters for stock `Cruentus`: the driver can call the processor at timer maturity, and the processor exits because percentage is `0`.

### 3.3 Spread driver timer test and reload

`TiberiumClass::SpreadDriver_AllTypes @ 0x007221B0` reads:

- `last = *(this + 0x100)` at `0x007221DD`.
- `interval = *(this + 0x108)` at `0x007221E3`.

The branch shape is:

```text
if last == -1:
    fire only if interval == 0
else:
    elapsed = g_CurrentFrameCounter - last
    if elapsed < interval:
        skip this type
    else:
        fire
```

On fire:

1. `CALL 0x00722440` (`SpreadProcessor`) at `0x00722200`.
2. Reload class pointer from the array.
3. Read `current = g_CurrentFrameCounter`.
4. Read raw type interval `*(this + 0x9C)` (`Spread=`) at `0x0072221C`.
5. Write `+0x100 = current`, `+0x104 = middle-word temp`, `+0x108 = Spread`.

No `Math__ftol`, multiplier, `[General] GrowthRate`, `SpreadPercentage`, or SpecialFlags bit read appears in the spread driver itself. The processor/predicate side owns percentage and spread-germination gates.

### 3.4 Growth driver timer test and reload

`TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40` reads:

- `last = *(this + 0x11C)` at `0x00722C76`.
- `interval = *(this + 0x124)` at `0x00722C7C`.

The branch shape is identical to spread:

```text
if last == -1:
    fire only if interval == 0
else:
    elapsed = g_CurrentFrameCounter - last
    if elapsed < interval:
        skip this type
    else:
        fire
```

On fire:

1. `CALL 0x00722F00` (`GrowthProcessor`) at `0x00722C99`.
2. Load `ScenarioClass` first dword and test byte bit `0x40` at `0x00722CA4`.
3. If bit `0x40` is set, `FLD double ptr [0x007E5138]` at `0x00722CA9`; `0x007E5138` is verified elsewhere as double `0.3`.
4. If bit `0x40` is clear, `FLD double ptr [0x007E1718]` at `0x00722CB1`; `0x007E1718` is verified as double `1.0`.
5. Load `Growth=` with `FILD dword ptr [this + 0xA8]` at `0x00722CC6`.
6. `FMUL ST1` at `0x00722CCC`.
7. `CALL Math__ftol @ 0x007C5F00` at `0x00722CCE`.
8. Write `+0x11C = g_CurrentFrameCounter`, `+0x120 = middle-word temp`, `+0x124 = ftol(Growth * multiplier)`.

For stock `Riparius Growth=2200`, this means the post-fire growth interval is `ftol(2200 * 0.3) = 660` when bit `0x40` is on, and `2200` when bit `0x40` is off. There is no read of `[General] GrowthRate` in this driver.

### 3.5 Constructor first-fire setup

`TiberiumClass::Constructor @ 0x007216C0` initializes the timer start/interval fields before the type is inserted into `g_TiberiumClass_Array`.

Assembly:

- `0x0072176A`: load `g_CurrentFrameCounter`.
- `0x00721770`: write `+0x100 = current`.
- `0x00721776`: write `+0x108 = 0`.
- `0x00721794`: load `g_CurrentFrameCounter`.
- `0x0072179A`: write `+0x124 = 0`.
- `0x007217A0`: write `+0x11C = current`.

Because both drivers fire when `elapsed >= interval`, an interval of zero means the first eligible driver pass fires immediately, even if `elapsed == 0`. The first fire then reloads the real interval as described above.

Tiny edge detail: if a timer `last` field is exactly `-1`, the driver does not compute elapsed. In that branch, a nonzero interval skips the type and zero interval fires. This is not the same as treating `-1` as "long ago."

### 3.6 INI/map readers that feed the fields

`TiberiumClass::ReadINI @ 0x00721A50` reads:

- `Spread` into `+0x9C`.
- `SpreadPercentage` into `+0xA0`.
- `Growth` into `+0xA8`.
- `GrowthPercentage` into `+0xB0`.

`ScenarioClass::Read_INI_Basic @ 0x00689E90` reads `[Basic] TiberiumGrowthEnabled` into `ScenarioClass+0x34A6` at `0x0068A589`.

Stock INI evidence:

- `ini/rulesmd.ini:43-45`: `[General] GrowthRate=5`, `TiberiumGrows=yes`, `TiberiumSpreads=yes`.
- `ini/rulesmd.ini:30388-30396`: `[Riparius] Growth=2200`, `GrowthPercentage=.06`, `Spread=2200`, `SpreadPercentage=.06`.
- `ini/rulesmd.ini:30400-30407`: `[Cruentus] Growth=10000`, `GrowthPercentage=0`, `Spread=10000`, `SpreadPercentage=0`.

The driver functions use per-type `Growth`/`Spread`, not `[General] GrowthRate`.

## 4. Current Rust Implementation Status

Current Rust still models ore growth as a scan/reservoir system:

- `src/sim/ore_growth.rs:62` defines `OreGrowthConfig { grows, spreads, growth_rate_seconds }`.
- `src/sim/ore_growth.rs:82-91` derives `grows` from `[General] TiberiumGrows`, `[Basic] TiberiumGrowthEnabled`, and map special flags; derives `spreads` without `[Basic] TiberiumGrowthEnabled`; derives cadence from `[General] GrowthRate`.
- `src/sim/ore_growth.rs:145` stores `OreGrowthState` as scan cursor/candidates plus partial queue-shaped vectors, not per-type timers.
- `src/sim/ore_growth.rs:309` `tick_ore_growth` scans cells according to `growth_rate_seconds`.
- `src/sim/world/mod.rs:1645` calls `tick_ore_growth`; terrain spawners run afterward.
- `src/sim/world/world_hash.rs:179` hashes current `ore_growth_state`, but the hashed model is still not the native per-type timer/queue model.

The Rust state has no equivalent of per-type `+0x100/+0x108` and `+0x11C/+0x124` timers or the growth reload multiplier branch.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Live logic tick ordering | verified | `0x0055B4D7`, `0x0055B4DC` | none |
| Spread driver gate | verified | `0x007221B8..0x007221C1` | none |
| Growth driver gate | verified | `0x00722C48..0x00722C51` | none |
| Spread timer test | verified | `0x007221DD..0x00722200` | none |
| Growth timer test | verified | `0x00722C76..0x00722C99` | none |
| Spread interval reload | verified | `0x0072221C..0x00722227` | none |
| Growth interval reload multiplier | verified | `0x00722CA4..0x00722CCE`; constants docs | none for driver formula |
| Constructor first-fire timer init | verified | `0x0072176A..0x007217A0` | middle timer words not semantically named |
| `TiberiumClass::ReadINI` offsets | verified | `0x00721A50` | none |
| `[Basic] TiberiumGrowthEnabled` writer | verified | `0x0068A589` | exact default initializer outside this slice; stock/template liveness documented in prior docs |
| Processor internals | deferred | sibling reports | out of scope |
| Native save/load timer rehydration | deferred | sibling slot/report | out of scope |
| Current Rust timer model | verified-source-scan | `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `world_hash.rs` | future implementation |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this an exhaustive slice? -> yes, bounded to driver timers/gates.` (evidence: user slot scope)
- `[RESOLVED] OQ-02 - Are the drivers active in standard YR? -> yes, called from the live logic tick at `0x0055B4D7/0x0055B4DC`.` (evidence: `0x0055AFB0`)
- `[RESOLVED] OQ-03 - Which driver runs first? -> growth runs before spread.` (evidence: `0x0055B4D7`, `0x0055B4DC`)
- `[RESOLVED] OQ-04 - What hard-gates both drivers? -> `ScenarioClass+0x34A6 != 0`.` (evidence: `0x007221B8`, `0x00722C48`)
- `[RESOLVED] OQ-05 - Does `TiberiumGrows` bit `0x40` hard-gate growth? -> no, in the growth driver it selects multiplier `0.3` versus `1.0` after processor fire.` (evidence: `0x00722CA4..0x00722CB7`)
- `[RESOLVED] OQ-06 - What fields does spread use? -> `+0x100` last/start and `+0x108` interval.` (evidence: `0x007221DD`, `0x007221E3`)
- `[RESOLVED] OQ-07 - What fields does growth use? -> `+0x11C` last/start and `+0x124` interval.` (evidence: `0x00722C76`, `0x00722C7C`)
- `[RESOLVED] OQ-08 - What happens when interval is zero? -> the type fires on the next eligible driver pass because elapsed is not less than zero.` (evidence: `0x007221F6..0x00722200`, `0x00722C8F..0x00722C99`)
- `[RESOLVED] OQ-09 - What happens when last is `-1`? -> the driver fires only if interval is zero; nonzero interval skips.` (evidence: `0x007221E9..0x00722200`, `0x00722C82..0x00722C99`)
- `[RESOLVED] OQ-10 - Does spread reload through `Math__ftol`? -> no, it writes raw `+0x9C`.` (evidence: `0x0072221C..0x00722227`)
- `[RESOLVED] OQ-11 - Does growth reload through `Math__ftol`? -> yes, `ftol(Growth * (bit0x40 ? 0.3 : 1.0))`.` (evidence: `0x00722CA4..0x00722CCE`)
- `[RESOLVED] OQ-12 - Where are `Growth` and `Spread` read from INI? -> `TiberiumClass::ReadINI @ 0x00721A50`.` (evidence: `0x00721A50`)
- `[RESOLVED] OQ-13 - Where is `[Basic] TiberiumGrowthEnabled` read? -> `ScenarioClass::Read_INI_Basic @ 0x0068A589`.` (evidence: `0x00689E90`)
- `[RESOLVED] OQ-14 - Does current Rust have native per-type timers? -> no, it uses `growth_rate_seconds` scan cadence.` (evidence: `src/sim/ore_growth.rs`)
- `[DEFERRED] OQ-15 - What consumes timer middle words `+0x104/+0x120`?` (category: requires-different-system-context; reason: drivers write them but timer helper/save-load slots are separate scope; next-step-if-pursued: save/load timer slot investigation)
- `[DEFERRED] OQ-16 - Exact default initializer for `ScenarioClass+0x34A6` before map read.` (category: requires-different-system-context; reason: this slice proves the reader and stock/template liveness; next-step-if-pursued: ScenarioClass constructor/default-map template pass)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Live tick runs all growth drivers before any spread driver. | `0x0055B4D7`, `0x0055B4DC` | current `tick_ore_growth` is one scan function; no native all-growth-then-all-spread driver split. | `src/sim/world/mod.rs`, `src/sim/ore_growth.rs` | Add driver-level ordering: iterate all types for growth, then all types for spread. | Both Riparius growth and spread timers mature on the same frame; every growth processor runs before any spread processor. Test: `ore_queue_drivers_run_all_growth_before_any_spread`. | Do not combine growth/spread into one per-cell scan loop. |
| Both drivers are hard-gated by map `[Basic] TiberiumGrowthEnabled` byte `+0x34A6`. | `0x007221B8`, `0x00722C48`, `0x0068A589` | Rust applies this only to `grows`, not to `spreads`. | `src/sim/ore_growth.rs`, `src/map/basic.rs`, app-init config bridge | If `TiberiumGrowthEnabled=no`, suppress both native growth and spread drivers. | Map with `[Basic] TiberiumGrowthEnabled=no` and `[SpecialFlags] TiberiumSpreads=yes` performs no spread processor calls. Test: `tiberium_growth_enabled_false_suppresses_growth_and_spread_drivers`. | Do not treat the key as growth-only because of its name. |
| Spread interval reload is raw per-type `Spread=` from `TiberiumClass+0x9C`. | `0x0072221C..0x00722227`, `0x00721A50` | Rust uses `[General] GrowthRate` scan cadence. | `rules::tiberium` parsing, `src/sim/ore_growth.rs` | Store per-type spread timer last/interval and reload interval from type data. | Changing `[Riparius] Spread=10` causes the second spread fire after 10 binary frames, independent of `[General] GrowthRate`. Test: `spread_driver_reloads_raw_tiberiumclass_spread_interval`. | Do not use `[General] GrowthRate` for spread cadence. |
| Growth interval reload is `ftol(Growth * (SpecialFlags bit 0x40 ? 0.3 : 1.0))`. | `0x00722CA4..0x00722CCE`; constants `0x007E5138=0.3`, `0x007E1718=1.0` | missing; no per-type growth timer or multiplier branch. | `src/sim/ore_growth.rs`, special-flags bridge, world hash | Store per-type growth timer and apply the exact multiplier branch after each processor fire. | Stock `Growth=2200`, bit `0x40` on reloads interval `660`; same map with bit off reloads `2200`. Test: `growth_driver_reloads_growth_interval_with_tiberiumgrows_multiplier`. | Do not hardcode `Growth=2200` or always multiply by `0.3`. |
| Constructor sets timer last fields to current frame and intervals to zero, so first eligible driver pass fires immediately. | `0x0072176A..0x007217A0`, driver comparisons | missing; Rust scanner waits according to scan cadence. | map init / `OreGrowthState` constructor | Initialize per-type timers so first driver pass fires immediately, then reloads real intervals. | On first live tick after map init, eligible nonempty queues process before waiting `Spread=2200`/`Growth=660`. Test: `ore_queue_initial_zero_interval_fires_on_first_driver_pass`. | Do not delay first growth/spread by one full interval. |

## Negative Facts / Do Not Do

- Do not use `[General] GrowthRate` as the driver cadence for this YR queue path. The drivers read per-type `Spread` and `Growth`.
- Do not treat SpecialFlags bit `0x40` as a hard growth-driver gate in `0x00722C40`; it selects the growth interval multiplier after the processor call.
- Do not let spread run when `[Basic] TiberiumGrowthEnabled=no`; the spread driver checks the same `+0x34A6` byte as growth.
- Do not treat `last == -1` as "timer expired"; with nonzero interval the driver skips that type.
- Do not implement timer priority as queue-entry wake-up time; queue priority behavior belongs to processor/heap reports, not these driver timers.

## Remaining Uncertainty

- Exact semantic consumer of timer middle words `+0x104` and `+0x120` remains deferred to save/load/timer-subobject research. The driver-visible last/interval behavior is verified.
- Exact default initializer for `ScenarioClass+0x34A6` before map `[Basic]` read remains deferred. The map reader and stock/template liveness are verified by `0x00689E90` and prior docs.

## Stale Docs / Follow-up Docs

- Replace wording that says growth driver simply uses per-type `Growth` with: "Growth driver reloads `+0x124` as `ftol(TiberiumClass+0xA8 Growth * (ScenarioClass.SpecialFlags bit 0x40 ? 0.3 : 1.0))`; with stock `Growth=2200` and bit `0x40` on, the reload interval is `660`."
- Replace wording that says `[Basic] TiberiumGrowthEnabled` only gates growth with: "`ScenarioClass+0x34A6` gates both `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40` and `SpreadDriver_AllTypes @ 0x007221B0` before per-type iteration."
- Replace wording that implies `last == -1` is an automatic fire state with: "In both drivers, `last == -1` bypasses elapsed computation and fires only when the interval word is zero; a nonzero interval skips."

## Sources

- Ghidra decompile: `0x007221B0`, `0x00722C40`, `0x0055AFB0`, `0x007216C0`, `0x00721A50`, `0x00689E90`.
- Ghidra assembly context: `0x007221B0`, `0x00722C40`, `0x0055B4D7`, `0x0072176A`, `0x00721794`.
- Prior docs checked: `TIBERIUMCLASS_MAP_LOAD_QUEUE_SEEDING_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md`, `SPECIAL_FLAGS_SYSTEM.md`, `ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`, `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`, `ADDRESS_MAP.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/app_init.rs`, `src/map/basic.rs`, `src/rules/ruleset.rs`.
