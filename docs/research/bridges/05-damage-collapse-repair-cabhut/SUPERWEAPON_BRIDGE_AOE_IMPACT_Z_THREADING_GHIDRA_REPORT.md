# Superweapon Bridge AoE Impact-Z Threading - Ghidra Research Report

**Address(es):** `0x00489280` (`Apply_area_damage`), `0x0053A300`, `0x0053B080`, `0x006CC390`, `0x004690B0`, `0x00423AC0`, `0x006E2390`, `0x006E0490`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR superweapon and superweapon-adjacent trigger callers that can route object splash through `Apply_area_damage`, limited to impact-Z construction and ground-vs-bridge object-list selection.  
**Non-Scope:** direct-fire splash, death AoE, bridge tile damage state machines, normal warhead detonation math, DiskLaser unit weapon internals, and full Chronosphere movement semantics.  
**Confidence:** HIGH for Lightning Storm, Psychic Dominator, Genetic Converter default, Ion trigger action, and `Apply_area_damage` layer selector; MEDIUM for Nuclear Missile final anim-damage Z because the bullet terminal path was not fully drained here.  
**Active in YR:** Conditional. Lightning Storm, Psychic Dominator, Genetic Converter, and Nuclear Missile are standard YR superweapons. Ion/Psychic Dominator trigger actions are live trigger actions in YR but are not standard sidebar superweapon launch paths.

## 1. Overview

`Apply_area_damage` selects exactly one object layer from the impact cell: ground list (`CellClass+0xE4`) or bridge/deck list (`CellClass+0xE8`). Standard YR superweapon callers are not all equal: Lightning Storm, Psychic Dominator, and default Genetic Converter construct a concrete impact Z high enough to select the bridge layer when targeted on structural bridge cells. The original Rust-status note in this report is now stale: current Lightning/Genetic Rust code threads `bridge_adjusted_impact_z` through `AoELayerContext`.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x140 & 0x100` | structural bridge cell, required before `Apply_area_damage` can select deck list | `0x00489562-0x0048958D`; `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` | Yes |
| `CellClass+0x140 & 0x400` | bridge-end/ramp-style flag used by Ion trigger Z construction, not by `Apply_area_damage` selector itself | `FUN_006E2390 @ 0x006E2390` | Conditional, trigger action only |
| `CellClass+0xE4` | ground object linked-list head | `0x004896CF`; prior bridge AoE report | Yes |
| `CellClass+0xE8` | bridge/deck object linked-list head | `0x004896C7`; prior bridge AoE report | Yes |
| `CellClass+0x11B` | signed terrain level byte used by Lightning Z construction | `LightningStorm__GroundStrike @ 0x0053A300` | Yes |
| `DAT_0089E864` | bridge-height offset halved by `Apply_area_damage` selector | `0x0048957A-0x00489584`; prior bridge AoE report | Yes |
| `DAT_00A9FA84` | Lightning bridge-height addend, same semantic role as bridge-height offset | `0x0053A300` | Yes |
| `DAT_00B0C07C` | SuperClass launch bridge-height addend for standard SW launch cases | `SuperClass__Launch @ 0x006CC390` | Yes |
| `DAT_00B0E6D4` | trigger-action bridge-height addend for Ion/PD area helper paths | `0x006E2390`, `TriggerAction__Execute @ 0x006DD8B0` | Conditional |
| `Rules+0x17C8` | Genetic Converter `MutateExplosion` branch; retail YR default is enabled | `SuperClass__Launch @ 0x006CD8xx`; `ini/rulesmd.ini:146` | Yes |

## 3. Core Logic

### Shared layer selector

Verified prior work remains correct and is not re-investigated here except as caller context:

```text
if impact_cell has structural bridge flag 0x100
and impact_z > CellClass::GetGroundHeight(impact_coord) + BridgeHeight / 2:
    scan deck list (CellClass+0xE8)
else:
    scan ground list (CellClass+0xE4)
```

Active in YR: Yes. Evidence: `Apply_area_damage @ 0x00489280`, especially selector region `0x00489562-0x0048958D`.

### Lightning Storm

`LightningStorm__Process @ 0x0053A6C0` calls `LightningStorm__GroundStrike @ 0x0053A300` after storm bolt anim progress reaches its strike point. `GroundStrike` constructs cell-center X/Y from the target cell and sets:

```text
impact_z = cell.Level * level_height + (cell.Flags & 0x100 ? bridge_height : 0)
```

The same local coordinate is used for explosion animation setup, the `FUN_0048A620` visual helper, and the `Apply_area_damage` call site at `0x0053A5D0`. For a structural bridge impact cell, the impact Z is one full bridge-height above the ground reference, so the strict `> ground + bridge_height / 2` selector chooses the bridge/deck object list. Non-bridge cells use ground.

Active in YR: Yes. Evidence: `LightningStorm__GroundStrike @ 0x0053A300`, caller `LightningStorm__Process @ 0x0053A81B`, `ini/rulesmd.ini:130-137`.

Tiny details:

- The bridge addend is gated only by `Flags & 0x100`; Lightning does not add height for `0x400` cells at this site.
- The duplicate-last-strike guard compares X, Y, and Z against `DAT_00A9FA30/34/38` before damage; the Z compared is the bridge-adjusted Z.
- The decompiler displays `Apply_area_damage(0, damage, 1, warhead)` at this fastcall site, but `Apply_area_damage` dereferences its coord argument. Treat the displayed first argument as a decompiler ABI artifact; the material verified behavior is the immediately preceding constructed local impact coordinate and the live non-crashing damage path.

### Psychic Dominator

`PsychicDominator__Process @ 0x0053AF40` calls `PsychicDominator__MindControlArea @ 0x0053B080` when first-animation progress reaches `Rules+0x304` (`DominatorFireAtPercentage`). `MindControlArea` gets the target cell from `DAT_00A9FA48`, calls the cell vtable `+0x48` center-coordinate method, copies X/Y/Z into a local coordinate, spawns `Rules+0x300` anim, then calls `Apply_area_damage` with `Rules+0x2F8` warhead and `DAT_00A9FACC` damage.

Active in YR: Yes. Evidence: `PsychicDominator__Process @ 0x0053AF40`, `PsychicDominator__MindControlArea @ 0x0053B080`, `ini/rulesmd.ini:536-542`, `ini/rulesmd.ini:30982-30992`.

Bridge layer implication: the impact Z is the cell center Z returned by `CellClass` vtable `+0x48`. The same method is used throughout bridge-aware standard SW launch cases before explicit `+ bridge_height` adds, so the implementation handoff should not assume all center-coordinate Z values are ground-only without a separate `GetCenterCoords` audit. For object splash parity, Rust should preserve a concrete target-cell impact Z and not fall back to all-layer damage.

### Genetic Converter

`SuperClass__Launch @ 0x006CC390`, case `9`, handles `Type=GeneticConverter`. It gets target-cell center coords, then if the target cell has `Flags & 0x100`, adds `DAT_00B0C07C` to the Z. It then branches on `Rules+0x17C8`.

Retail YR `ini/rulesmd.ini:146` sets `MutateExplosion=yes`, so the default standard YR path is the `Apply_area_damage` branch at `0x006CD90C`, not the manual 3x3 branch. On a structural bridge target, the default path therefore supplies a bridge-adjusted impact Z and selects the deck list. If `MutateExplosion=no`, the alternate manual 3x3 path does not use the impact-Z selector; it chooses each scanned cell's `+0xE4` or `+0xE8` list directly based on that scanned cell's `Flags & 0x100`.

Active in YR: Yes for default `MutateExplosion=yes`; Conditional for manual path when `MutateExplosion=no`. Evidence: `SuperClass__Launch @ 0x006CC390` case `9`; `ini/rulesmd.ini:146`; `ini/rulesmd.ini:590`; `ini/rulesmd.ini:27246`.

Tiny details:

- The launch anim Z is `impact_z + 5`; this visual offset should not be used as object-damage impact Z.
- The manual path's layer choice is per scanned cell, not the `Apply_area_damage` single-impact-cell selector.
- Prior report wording that treated manual mode as the default is stale.

### Nuclear Missile / MultiMissile

`SuperClass__Launch @ 0x006CC390`, case `0`, launches the nuke carrier weapon/bullet. The launch case computes target ground height and adds bridge height only for the bullet's constructed start/target context. The object splash ultimately flows through `WarheadTypeClass__Detonate @ 0x004690B0`, `NukeGroundZero__ApplyDamage @ 0x004251F0`, and/or damage anim processing in `AnimClass__AI @ 0x00423AC0`. A 2026-05-22 verify-doc audit corrected the earlier zero-damage claim: `NukeGroundZero__ApplyDamage` passes `Rules+0x1530` (`AtomDamage`, retail 1000) and `Rules+0xF8C` (`NukeWarhead`) into `Apply_area_damage`, so it does not exit through the `param_2 == 0 || param_4 == 0` gate in the normal retail path.

Active in YR: Yes. Evidence: `SuperClass__Launch @ 0x006CC390` case `0`; `WarheadTypeClass__Detonate @ 0x004690B0`; `NukeGroundZero__ApplyDamage @ 0x004251F0`; `AnimClass__AI @ 0x00423AC0`; `ini/rulesmd.ini:584`.

Bridge layer implication: no verified standard nuke call in this slice adds bridge height specifically for the final damaging `Apply_area_damage` object splash. The best-supported handoff is to avoid inventing a nuke bridge-deck shortcut until the bullet terminal/anim position path is separately drained.

### Trigger Action Ion / PD Area Helpers

`TriggerAction__Execute @ 0x006DD8B0` has live action cases that call `FUN_006E2390` (case `0x2A`) and `FUN_006E0490` (case `0x3F`). These are not standard sidebar superweapon launches, but they are live YR trigger actions.

`FUN_006E2390` constructs:

```text
impact_z = CellClass::GetGroundHeight(center)
if target cell flags have 0x100 or 0x400:
    impact_z += DAT_00B0E6D4
```

Active in YR: Conditional. Evidence: `TriggerAction__Execute @ 0x006DD8B0` case `0x2A`; `FUN_006E2390 @ 0x006E2390`.

`FUN_006E0490` calls `Apply_area_damage` five times around a trigger coordinate with the warhead at `Rules+0xFA8` and decompiler-displayed zero damage; this matches an area visual/bridge-cell helper, not a standard YR sidebar superweapon. It should not be used to infer default superweapon AoE behavior.

Active in YR: Conditional trigger action only. Evidence: `TriggerAction__Execute @ 0x006DD8B0` case `0x3F`; `FUN_006E0490 @ 0x006E0490`.

## 4. INI Keys

| Key | Section | Retail YR value | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `LightningDamage` | `[General]` | `250` | damage argument for Lightning Storm AoE | Yes, `ini/rulesmd.ini:131` |
| `LightningWarhead` | `[General]` | `IonWH` | warhead pointer used by Lightning Storm AoE | Yes, `ini/rulesmd.ini:133` |
| `LightningStormDuration/HitDelay/ScatterDelay/CellSpread/Separation` | `[General]` | retail defaults at lines `130-137` | schedule/targeting, not layer selector | Yes |
| `DominatorWarhead` | `[General]` | `DominatorWH` | warhead pointer for PD damage phase | Yes, `ini/rulesmd.ini:537` |
| `DominatorDamage` | `[General]` | `1000` | damage argument for PD damage phase | Yes, `ini/rulesmd.ini:538` |
| `DominatorFireAtPercentage` | `[General]` | `20` | process gate before `MindControlArea` fires | Yes, `ini/rulesmd.ini:542` |
| `MutateExplosion` | `[General]` | `yes` | selects default Genetic `Apply_area_damage` path | Yes, `ini/rulesmd.ini:146` |
| `MutateExplosionWarhead` | `[SpecialWeapons]` | `MutateExplosion` | warhead used by default Genetic AoE path | Yes, `ini/rulesmd.ini:590` |
| `NukeWarhead` | `[SpecialWeapons]` | `Nuke` | nuke carrier warhead; object splash path remains anim/bullet-driven | Yes, `ini/rulesmd.ini:584` |
| `IonCannonWarhead` | `[CombatDamage]` | `IonCannonWH` | bridge-tile special case, not standard sidebar SW layer selector | Conditional, `ini/rulesmd.ini:874` |

## 5. Integration Points

| Caller | Path | Layer behavior | Active in YR |
|---|---|---|---|
| `0x0053A300` | Lightning Storm ground strike | concrete impact Z; bridge deck for structural bridge target | Yes |
| `0x0053B080` | Psychic Dominator damage/capture phase | concrete center-coordinate Z; should not use all-layer fallback | Yes |
| `0x006CD90C` | Genetic default `MutateExplosion=yes` | concrete impact Z; bridge deck for structural bridge target | Yes |
| `0x006CC390` case `9` manual branch | Genetic `MutateExplosion=no` | manual per-cell `+0xE4/+0xE8`, no impact-Z selector | Conditional |
| `0x004690B0` / `0x00423AC0` | Nuclear Missile detonation/anim damage | damaging final Z not fully drained; no verified bridge-height deck shortcut | Yes, MEDIUM |
| `0x006E2390` | trigger Ion strike helper | concrete ground height plus `0x100|0x400` bridge addend | Conditional trigger action |
| `0x006E0490` | trigger multi-point helper | not standard sidebar SW; do not infer standard SW behavior | Conditional trigger action |

## 6. Current Rust Implementation Status

Rust already has a bridge-layer-aware AoE primitive, but implemented SW launchers are not threading it yet:

- `src/sim/combat/combat_aoe.rs:35` defines `AoELayerContext { occupancy, terrain, impact_z }`.
- `src/sim/combat/combat_aoe.rs:57` switches to single selected layer only when occupancy and terrain are present; otherwise it falls back to all entities.
- `src/sim/combat/combat_aoe.rs:187` selects bridge layer with a strict `impact_z > level + bridge_height / 2` test.
- `src/sim/superweapon/lightning_storm.rs:241-255` now computes `bridge_adjusted_impact_z` and passes occupancy, terrain, and `impact_z` through `AoELayerContext`; the earlier `AoELayerContext::default()` status is stale.
- `src/sim/superweapon/genetic_converter.rs:88-102` now computes `bridge_adjusted_impact_z` and passes occupancy, terrain, and `impact_z` through `AoELayerContext`; the earlier `AoELayerContext::default()` status is stale.
- Psychic Dominator and MultiMissile standard launch damage are not implemented in the scanned Rust launch dispatch (`src/sim/world/world_commands.rs:1177-1232` warns for unimplemented kinds except Lightning/Genetic and non-damage SWs).

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Apply_area_damage` object layer selector | verified | `0x00489280`, prior bridge AoE report | none for this slice |
| Lightning Storm `GroundStrike` impact Z | verified | `0x0053A300`, caller `0x0053A81B` | none for bridge object-layer handoff |
| Psychic Dominator `MindControlArea` impact Z source | verified | `0x0053B080`, caller `0x0053AFBA` | exact `GetCenterCoords` internals deferred |
| Genetic Converter case `9`, `MutateExplosion=yes` | verified | `0x006CC390`, call `0x006CD90C`, `ini/rulesmd.ini:146` | none |
| Genetic Converter manual `MutateExplosion=no` | verified for layer choice shape | `0x006CC390` case `9` manual branch | full non-default gameplay parity out of scope |
| Nuclear Missile damaging final Z | touched-not-exhausted | `0x006CC390`, `0x004690B0`, `0x00423AC0`, `0x004251F0` | separate bullet terminal / anim spawn audit |
| Ion trigger helper `FUN_006E2390` | verified | `0x006E2390`, `TriggerAction__Execute` case `0x2A` | not standard sidebar SW |
| PD/Ion trigger helper `FUN_006E0490` | touched-not-exhausted | `0x006E0490`, `TriggerAction__Execute` case `0x3F` | out-of-scope trigger visual/helper semantics |
| DiskLaser callers | deferred | xrefs to `0x00663030`, `0x004A76AF` | non-standard SW/unit weapon path, outside scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Which standard YR superweapon paths call Apply_area_damage for object splash? -> Lightning Storm, Psychic Dominator, Genetic Converter default, and Nuclear/anim path are relevant; IronCurtain, Chronosphere, ChronoWarp, ParaDrop, AmerParaDrop, SpyPlane, ForceShield, and PsychicReveal are not object splash damage paths in the launch switch.` (evidence: `SuperClass__Launch @ 0x006CC390`; `src/rules/superweapon_type.rs:24-48`)
- `[RESOLVED] OQ-2 - Does Lightning Storm use default/all-layer behavior in gamemd? -> No; it constructs concrete bridge-adjusted impact Z and routes through the normal selector.` (evidence: `0x0053A300`, `0x0053A5D0`)
- `[RESOLVED] OQ-3 - Does Psychic Dominator use a concrete Z? -> Yes; it copies target cell center-coordinate Z into the damage coordinate before calling `Apply_area_damage`.` (evidence: `0x0053B080`, `0x0053B16B`)
- `[RESOLVED] OQ-4 - Which Genetic branch is default in stock YR? -> `MutateExplosion=yes`, so default is `Apply_area_damage`, not manual 3x3.` (evidence: `0x006CD90C`; `ini/rulesmd.ini:146`)
- `[RESOLVED] OQ-5 - Does Genetic manual mode use impact-Z selection? -> No; it directly picks `CellClass+0xE4` or `+0xE8` per scanned cell from that cell's bridge flag.` (evidence: `SuperClass__Launch @ 0x006CC390` case `9`)
- `[CORRECTED 2026-05-22] OQ-6 - Does `NukeGroundZero__ApplyDamage` deliver object splash? -> It does not zero out in the normal retail path: assembly at `0x00425222..0x00425237` passes `Rules+0xF8C` (`NukeWarhead`) and `Rules+0x1530` (`AtomDamage`, retail 1000) into `Apply_area_damage`. Exact nuke final impact-Z/layer behavior still remains bounded to the bullet terminal / anim spawn audit.`
- `[RESOLVED] OQ-7 - Is Ion Cannon a standard YR sidebar superweapon? -> No in standard YR rules; its helper is reachable as a trigger action.` (evidence: `TriggerAction__Execute @ 0x006DD8B0` case `0x2A`; `rulesmd.ini` standard SW list lacks IonCannon)
- `[DEFERRED] OQ-8 - Exact `CellClass::GetCenterCoords` bridge-Z internals.` (category: `requires-different-system-context`; reason: this slice only needed caller layer threading and found center-Z use; next-step-if-pursued: audit vtable `+0x48` implementation and all callers)
- `[DEFERRED] OQ-9 - Nuclear bullet terminal Z at final damaging animation tick.` (category: `requires-different-system-context`; reason: requires bullet/anim spawn trace outside SW caller layer handoff; next-step-if-pursued: drain `NukeMaker__SpawnDownwardNuke`, BulletClass terminal detonation, and RING1 anim construction)
- `[DEFERRED] OQ-10 - DiskLaser object splash layer behavior.` (category: `out-of-scope`; reason: caller is a unit/weapon system, not standard YR superweapon launch bridge AoE)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Lightning Storm impact on structural bridge uses impact Z = level height + bridge height and selects deck occupants only | `0x0053A300`; `0x00489562-0x0048958D`; `src/sim/superweapon/lightning_storm.rs:241-255` | implemented after original report; verify with targeted tests | `src/sim/superweapon/lightning_storm.rs`, `src/sim/combat/combat_aoe.rs` caller context | preserve occupancy, terrain, and bridge-adjusted impact Z in `AoELayerContext` for each bolt | two infantry/tanks in same bridge XY, one deck and one under-bridge; lightning bolt on bridge cell damages deck only | Do not regress to `AoELayerContext::default()`; proposed test `test_lightning_storm_bridge_aoe_uses_impact_z_layer` |
| Genetic Converter default stock YR `MutateExplosion=yes` uses `Apply_area_damage` with bridge-adjusted impact Z; manual branch is non-default and per-cell layer-based | `0x006CD90C`; `ini/rulesmd.ini:146`; `src/sim/superweapon/genetic_converter.rs:88-102` | implemented after original report; verify with targeted tests | `src/sim/superweapon/genetic_converter.rs`, `src/rules/ruleset.rs` | preserve concrete bridge impact Z in AoE for default mutate explosion; manual `MutateExplosion=no` should not be modeled as the default selector | target structural bridge cell with infantry above and below; default Genetic Converter mutates deck infantry only | Do not implement manual 3x3 as stock default; proposed test `test_genetic_converter_default_bridge_aoe_uses_impact_z_layer` |
| Psychic Dominator damage phase uses target cell center-coordinate Z and should not damage both layers through fallback when implemented | `0x0053B080`; `0x0053AF40`; `ini/rulesmd.ini:537-542` | unimplemented / unchecked | future `src/sim/superweapon/psychic_dominator.rs`, launch dispatch in `src/sim/world/world_commands.rs` | when implementing PD damage, supply concrete target-cell impact Z and selected-layer occupancy context to AoE damage | target PD on structural bridge with candidates on deck and ground; damage/mind-control phase affects selected layer, not both | Do not start from all-entity AoE fallback; proposed test `test_psychic_dominator_bridge_damage_uses_target_center_z_layer` |

### Negative Facts / Do Not Do

- Do not treat `Wall=yes` or bridge tile damage as the object layer selector; object layer is decided before tile damage. Evidence: `Apply_area_damage @ 0x00489280`; `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`.
- Do not damage both `CellClass+0xE4` and `CellClass+0xE8` for one SW splash detonation. Evidence: selected list at `0x004896C7/0x004896CF`.
- Do not use the Genetic manual 3x3 branch as stock YR default; `MutateExplosion=yes` selects the `Apply_area_damage` branch. Evidence: `ini/rulesmd.ini:146`; `0x006CD90C`.
- Do not apply Lightning's bridge height rule to `0x400` cells; Lightning checks `0x100` only. Evidence: `0x0053A300`.
- Do not treat Ion trigger action behavior as a standard YR sidebar superweapon requirement. Evidence: `TriggerAction__Execute @ 0x006DD8B0` case `0x2A`; no standard `IonCannon` superweapon in `rulesmd.ini` `[SuperWeaponTypes]`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/SUPERWEAPON_IMPACT_Z_BRIDGE_AOE_GHIDRA_REPORT.md` Section 7.2 should replace "When `Rules+0x17c8 == 0` (which is the default ...)" with: "Retail YR `rulesmd.ini` sets `MutateExplosion=yes`, so the standard default path is the `Apply_area_damage` branch. The manual 3x3 `CellClass+0xE4/+0xE8` branch is conditional on `MutateExplosion=no` and should not be treated as stock default behavior."
- The same report's Section 8 should replace the broad "Coord-0 Pattern - Shared Global Impact Coord" conclusion with: "Several fastcall call sites are decompiler-ambiguous and display a zero first argument even though `Apply_area_damage` dereferences its coord argument. Use the caller's immediately constructed local coordinate and live call site as evidence for impact-Z; do not infer a global impact-coordinate system from the displayed zero argument alone."

## Sources

- Ghidra decompiled/read:
  - `0x00489280` `Apply_area_damage`
  - `0x0053A300` `LightningStorm__GroundStrike`
  - `0x0053A6C0` `LightningStorm__Process`
  - `0x0053B080` `PsychicDominator__MindControlArea`
  - `0x0053AF40` `PsychicDominator__Process`
  - `0x006CC390` `SuperClass__Launch`
  - `0x004690B0` `WarheadTypeClass__Detonate`
  - `0x00423AC0` `AnimClass__AI`
  - `0x004251F0` `NukeGroundZero__ApplyDamage`
  - `0x006DD8B0` `TriggerAction__Execute`
  - `0x006E2390` trigger Ion helper
  - `0x006E0490` trigger multi-point helper
  - `0x00663030`, `0x006620F0`, `0x006622C0` touched only to exclude DiskLaser from this scoped standard SW handoff
- Docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/SUPERWEAPON_IMPACT_Z_BRIDGE_AOE_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Rust scanned:
  - `src/sim/combat/combat_aoe.rs`
  - `src/sim/superweapon/lightning_storm.rs`
  - `src/sim/superweapon/genetic_converter.rs`
  - `src/sim/world/world_commands.rs`
  - `src/rules/superweapon_type.rs`
