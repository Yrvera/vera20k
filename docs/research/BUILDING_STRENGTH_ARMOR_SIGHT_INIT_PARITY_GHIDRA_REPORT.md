# Building Strength / Armor / Sight Init Parity - Ghidra Research Report

**Address(es):** `0x005F9490` (`ObjectTypeClass::ReadINI`), `0x0071429A` (`TechnoTypeClass::ReadINI` `Sight=` site), `0x0044FB20` / `0x0044FD8A` callers into `BuildingClass::ReadFromINI`, `0x004C9C70` (`FactoryClass::StartProduction`), `0x0045E880` (`BuildingTypeClass` create-instance vtable slot +0x8C), `0x0043B740` (`BuildingClass::Constructor`), `0x00442C40` (`BuildingClass::Init_Managers`), `0x005F5C60` (`ObjectClass::GetHealthRatio`), `0x00440580` (`BuildingClass::Unlimbo`), `0x00449A50` (`BuildingClass::Mission_Construction`).
**Investigation Mode:** exhaustive-slice for the named fields and ordinary undamaged production-object current-HP initialization.
**Claimed Scope:** `Strength=`, `Armor=`, and `Sight=` type-field parsing/defaults; building scenario-load HP scaling; building placement reveal radius source; Rust read-only comparison for these fields.
**Non-Scope:** combat damage formulas, target selection, shroud spiral/LOS algorithms, production animation behavior, full save/load state restoration.
**Confidence:** High for type-field parsing, scenario-load HP mapping, and production-created initial HP.
**Active in YR:** Yes for all verified parsing and placement/load paths; conditional only for the TS fog-of-war snapshot branch, which is off by default.

## 1. Overview

The binary stores `Armor=` and `Strength=` as universal `ObjectTypeClass` fields and `Sight=` as a universal `TechnoTypeClass` field inherited by `BuildingTypeClass`. Buildings use those type fields as max HP / armor index / reveal radius. Scenario-map buildings scale their map health token by `Type+0xA0`; ordinary factory-created buildings are initialized to full `Type+0xA0` by `BuildingClass::Init_Managers` before `BuildingClass::Constructor` returns.

Current Rust parses all three keys into `ObjectType`, uses `strength` as both `Health.current` and `Health.max` for production spawns, uses `strength` as map max-health for scenario entities, uses `armor` by type lookup in AoE damage, and uses `sight` as `GameEntity.vision_range`. This is a MATCH for the covered field storage, map-spawn max/radius/armor mapping, and ordinary factory-created building initial HP.

## 2. Class Layout / Key Offsets

| Field | Binary offset | Owner | Evidence | Active in YR |
|---|---:|---|---|---|
| `Armor=` | `ObjectTypeClass+0x9C` (`param_1[0x27]`) | all object types, including buildings | `ObjectTypeClass::ReadINI` `0x005F94BC..0x005F94CD`; armor matrix doc confirms 11-name table | Yes; building types inherit this parser |
| `Strength=` / max HP | `ObjectTypeClass+0xA0` (`param_1[0x28]`) | all object types, including buildings | `ObjectTypeClass::ReadINI` `0x005F94D3..0x005F94ED`; `ObjectClass::GetHealthRatio` `0x005F5C60` divides current HP by `Type+0xA0` | Yes; building scenario loader and health ratio consumers read it |
| `Sight=` | `TechnoTypeClass+0x5E8` (`param_1[0x17A]`) | all techno types, including buildings | `TechnoTypeClass::ReadINI` xref `0x0071429A` stores result to `[EBP+0x5E8]`; ctor default `param_1[0x17A]=0` | Yes; `BuildingClass::Unlimbo`/base techno reveal uses it |
| Current HP | `ObjectClass+0x6C` | runtime object | `ObjectClass::GetHealthRatio` `0x005F5C60`; `BuildingClass::ReadFromINI` writes scenario health; `BuildingClass::Init_Managers @ 0x00442C40` copies `Type+0xA0` for constructed buildings | Yes |
| Estimated/visual HP | `ObjectClass+0x70` | runtime object | `BuildingClass::ReadFromINI` writes `field_0x70 = Health`; MCV deploy doc also writes both | Yes |

## 3. Core Logic

### Type INI parsing

- `ObjectTypeClass::ReadINI` first delegates common object parsing. It reads `Armor=` through the armor-name lookup helper with current `+0x9C` as the default. Missing `Armor=` leaves constructor/default value unchanged. Evidence: `0x005F94BC` pushes `"Armor"`, `0x005F94C8` calls the armor parser, `0x005F94CD` stores `EAX` to `[this+0x9C]`. Active in YR: Yes.
- Immediately after `Armor=`, `ObjectTypeClass::ReadINI` reads `Strength=` with current `+0xA0` as the default. Missing `Strength=` leaves constructor/default value unchanged. Evidence: `0x005F94D3` reads `[this+0xA0]`, `0x005F94DA` pushes `"Strength"`, `0x005F94E2` calls `ReadInt`, `0x005F94ED` stores to `[this+0xA0]`. Active in YR: Yes.
- `ObjectTypeClass::Constructor` zero-initializes `+0x9C` and `+0xA0`, so missing `Armor=` defaults to armor index `0` (`none` per verified armor table) and missing `Strength=` defaults to `0`. Evidence: constructor writes `param_1[0x27]=0` and `param_1[0x28]=0`. Active in YR: Yes.
- `TechnoTypeClass::ReadINI` reads `Sight=` with current `+0x5E8` as default and stores back to `+0x5E8`. Evidence: `0x00714293` loads `[EBP+0x5E8]`, `0x0071429A` pushes `"Sight"`, `0x007142A2` calls `ReadInt`, `0x007142A7` stores result to `[EBP+0x5E8]`. Active in YR: Yes.
- `TechnoTypeClass::Constructor` defaults `+0x5E8` to `0`. Evidence: ctor write `param_1[0x17A]=0`. Active in YR: Yes.

### Runtime use

- Health ratio and downstream HP-relative behavior use current HP divided by `Type+0xA0`. Evidence: `ObjectClass::GetHealthRatio` at `0x005F5C60` returns `this->Health / this->GetType()->+0xA0`. Active in YR: Yes.
- Scenario `[Structures]` load creates a `BuildingClass`, then computes current HP from the scenario health token and the building type's `+0xA0`; health tokens above `0xFF` are clamped to `0x100`, and the result is capped to the type max. Evidence: `BuildingClass::ReadFromINI` decompile: `if (local_d4 > 0xff) local_d4 = 0x100; ... this->Health = Math::ftol(...); if (Type+0xA0 - 3 < Health) this->Health = Type+0xA0; field_0x70 = Health`. Active in YR: Yes; this is standard scenario load.
- Factory/production buildings are created by `FactoryClass::StartProduction @ 0x004C9C70`, which calls `type->vtable+0x8C`, stores the result at `Factory+0x58`, and does not itself write HP. The building target `0x0045E880` allocates `0x720` and calls `BuildingClass::Constructor @ 0x0043B740`. The inherited `ObjectClass::Constructor @ 0x005F3900` first initializes `+0x6C/+0x70` to `0xFF`; after assigning `Type` at `+0x520`, `BuildingClass::Constructor` calls `BuildingClass::Init_Managers @ 0x00442C40`, whose instructions at `0x00442C7B/0x00442C7E` copy `Type+0xA0` into both fields. `FactoryClass::StartProduction` thus receives a full-HP building. The ordinary construction mission `0x00449A50`, `GrandOpening`, Unlimbo, and construction completion do not replace that value. Active in YR: Yes. Evidence: active `gamemd.exe` Ghidra `batch_decompile(0x0043B740,0x00442C40,0x005F3900,0x00449A50)` plus function-scoped `search_instructions` for `+0x6C/+0x70`, 2026-07-25.
- Placement reveal uses `Sight=` (`Type+0x5E8`) as the radius. The prior high-confidence Unlimbo report says normal placement reveals through `TechnoClass::Unlimbo`/object reveal and wall extension calls `RevealAroundCell` twice with `Type+0x5E8`; I spot-checked the `Sight=` parser and Unlimbo report path. Active in YR: Yes for ordinary reveal; conditional for the fogged snapshot branch gated by `SpecialFlags.FogOfWar`, default off in standard YR.
- Armor field use is universal: combat selection/damage docs identify target armor as `target.Type+0x9C`; the verses matrix doc confirms `Armor=` writes `+0x9C` and the armor-name table order is `none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2`. Active in YR: Yes; this report does not re-cover formulas.

## 4. INI Keys

| Key | Binary read site | Default if missing | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `Strength=` | `ObjectTypeClass::ReadINI` `0x005F94DA` | constructor `0` | Max HP/type strength at `+0xA0`; scenario building HP scales/caps against it | Yes |
| `Armor=` | `ObjectTypeClass::ReadINI` `0x005F94BC` | constructor `0` = `none` | Armor index at `+0x9C`; target armor lookup for verses | Yes |
| `Sight=` | `TechnoTypeClass::ReadINI` `0x0071429A` | constructor `0` | Reveal/vision radius at `+0x5E8` | Yes |

Retail YR building sections in `ini/rulesmd.ini` commonly set all three; examples include `[GACNST] Strength=1000 Armor=concrete Sight=8`, `[GAPOWR] Strength=750 Armor=wood Sight=4`, and many civilian/tech buildings. Base RA2 `rules.ini` is fallback; `rulesmd.ini` overrides for YR.

## 5. Integration Points

| Path | Verified behavior | Active in YR |
|---|---|---|
| Type bootstrap | `BuildingTypeClass::ReadINI` inherits `TechnoTypeClass::ReadINI`, which inherits `ObjectTypeClass::ReadINI`; the three fields are not building-only special cases | Yes |
| Scenario/map `[Structures]` load | Constructs building, converts map health percentage/token to current HP using `Type+0xA0`, caps to max, then unlimbos | Yes |
| Sidebar/factory production | `FactoryClass::StartProduction` creates object via `type->vtable+0x8C`; constructor finalization copies `Type.Strength` to current and visual HP; `HouseClass::Place_Production` later unlimbos that full-HP object | Yes |
| Placement reveal | `BuildingClass::Unlimbo` / `TechnoClass::Unlimbo` uses `Type+0x5E8` for reveal radius | Yes |
| TS fog snapshot | `SpecialFlags.FogOfWar` branch creates fogged building snapshots after checking foundation cells | Conditional; default off in YR |

## 6. Current Rust Implementation Status

| Rust area | Status vs verified binary | Evidence |
|---|---|---|
| `ObjectType` parsing `Strength=` | MATCH for key/default storage | `src/rules/object_type.rs:808` uses `get_i32("Strength").unwrap_or(0)` |
| `ObjectType` parsing `Armor=` | MATCH for key/default string | `src/rules/object_type.rs:809` uses `get("Armor").unwrap_or("none")` |
| `ObjectType` parsing `Sight=` | MATCH for key/default storage | `src/rules/object_type.rs:824` uses `get_i32("Sight").unwrap_or(0)` |
| Map entity health max | MATCH for using `Strength=` as max; UNKNOWN for exact map-token edge cases beyond 0..256 | `src/sim/world/world_spawn.rs:73-90` scales `map_ent.health` by `obj.strength` |
| Production spawn current/max health | MATCH for ordinary nonzero-strength stock buildings | `src/sim/world/world_spawn.rs` sets current=max=`obj.strength.max(1)`; native `BuildingClass::Init_Managers @ 0x00442C40` copies `Type+0xA0` to both HP fields before constructor return |
| Runtime vision radius | MATCH for using `Sight=` as entity vision radius with Rust cap | `src/sim/world/world_spawn.rs:106-109`, `:319`; `GameEntity::new` stores `vision_range` at `game_entity.rs:323-327` |
| Armor lookup for damage | MATCH for type-field lookup in Rust's implemented AoE path | `src/sim/combat/combat_aoe.rs:159-165` and `:264-270` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectTypeClass::ReadINI` `Armor=` | verified | `0x005F94BC..0x005F94CD` | none |
| `ObjectTypeClass::ReadINI` `Strength=` | verified | `0x005F94D3..0x005F94ED` | none |
| `ObjectTypeClass::Constructor` defaults | verified | decompile writes `+0x9C=0`, `+0xA0=0` | none |
| `TechnoTypeClass::ReadINI` `Sight=` | verified | `0x0071429A..0x007142A7` | none |
| `TechnoTypeClass::Constructor` `Sight` default | verified | decompile write `param_1[0x17A]=0` | none |
| Scenario building HP conversion | verified | `BuildingClass::ReadFromINI` decompile | exact malformed/missing scenario health token behavior not pursued |
| `FactoryClass::StartProduction` create path | verified | `0x004C9C70` decompile | none for initial HP |
| `BuildingTypeClass` create-instance slot | verified | vtable `0x007E4570+0x8C -> 0x0045E880`; decompile calls `0x0043B740` | none |
| `BuildingClass::Constructor` and type finalization | verified for initial HP | `0x0043B740` assigns `Type` then calls `0x00442C40`; MOVs at `0x00442C7B/0x00442C7E` copy `Type+0xA0` to `+0x6C/+0x70` | damage/repair/scripted mutation after construction is separate scope |
| `ObjectClass::GetHealthRatio` | verified | `0x005F5C60` | none |
| Placement reveal radius | verified via existing report + parser spot-check | `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` lines 348-362; parser xref `0x0071429A` | no shroud algorithm re-trace by design |
| Rust comparison | verified read-only | listed file:line evidence | no edits made |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Where is Strength read? -> ObjectTypeClass::ReadINI stores ReadInt("Strength", old +0xA0) to +0xA0` (evidence: `0x005F94D3..0x005F94ED`)
- `[RESOLVED] OQ-02 - Where is Armor read? -> ObjectTypeClass::ReadINI stores armor-name lookup result to +0x9C` (evidence: `0x005F94BC..0x005F94CD`; armor matrix doc)
- `[RESOLVED] OQ-03 - Where is Sight read? -> TechnoTypeClass::ReadINI stores ReadInt("Sight", old +0x5E8) to +0x5E8` (evidence: `0x0071429A..0x007142A7`)
- `[RESOLVED] OQ-04 - Missing-key defaults? -> constructor defaults are Armor index 0, Strength 0, Sight 0` (evidence: `ObjectTypeClass::Constructor`; `TechnoTypeClass::Constructor`)
- `[RESOLVED] OQ-05 - Does scenario building HP depend on Strength? -> yes, scenario health token is converted/capped using Type+0xA0` (evidence: `BuildingClass::ReadFromINI`)
- `[RESOLVED] OQ-06 - Does health-ratio code use Strength as max? -> yes, `GetHealthRatio` divides current HP by `GetType()+0xA0`` (evidence: `0x005F5C60`)
- `[RESOLVED] OQ-07 - Does placement reveal use Sight? -> yes, building placement reveal radius is Type+0x5E8` (evidence: `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` lines 348-362)
- `[RESOLVED] OQ-08 - Is FogOfWar branch standard YR? -> no, conditional on `SpecialFlags.FogOfWar`; default off` (evidence: `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` lines 364-379)
- `[RESOLVED] OQ-09 - Does current Rust parse all three keys? -> yes` (evidence: `object_type.rs:808-824`)
- `[RESOLVED] OQ-10 - Does current Rust map sight to vision? -> yes` (evidence: `world_spawn.rs:106-109`, `:319`; `vision/mod.rs:498-499`)
- `[RESOLVED] OQ-11 - Does current Rust use armor in damage lookup? -> yes in implemented AoE damage` (evidence: `combat_aoe.rs:159-165`, `:264-270`)
- `[RESOLVED] OQ-12 - What exact current HP does an ordinary undamaged factory-created BuildingClass have immediately after `CreateInstance` and through Mission_Construction? -> Full `Type.Strength`; `BuildingClass::Init_Managers` overwrites inherited `0xFF` defaults at `+0x6C/+0x70` before constructor return, and the traced construction mission does not write either field.` Evidence: active `gamemd.exe` `0x0043B740`, `0x00442C40` (`0x00442C7B/0x00442C7E`), `0x005F3900`, `0x00449A50`.

## Sources

- Ghidra decompilation / assembly context: `ObjectTypeClass::ReadINI`, `ObjectTypeClass::Constructor`, `TechnoTypeClass::ReadINI`, `TechnoTypeClass::Constructor`, `BuildingClass::ReadFromINI`, `FactoryClass::StartProduction @ 0x004C9C70`, `BuildingTypeClass` create-instance slot `0x0045E880`, `BuildingClass::Constructor @ 0x0043B740`, `BuildingClass::Init_Managers @ 0x00442C40`, `ObjectClass::Constructor @ 0x005F3900`, `ObjectClass::GetHealthRatio @ 0x005F5C60`, `BuildingClass::Mission_Construction @ 0x00449A50`.
- Existing reports referenced: `docs/research/BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`, `docs/research/combat/systems/verses_armor_matrix.md`, `docs/research/FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`, `docs/research/BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust files checked read-only: `src/rules/object_type.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/combat/combat_aoe.rs`, `src/sim/vision/mod.rs`.
