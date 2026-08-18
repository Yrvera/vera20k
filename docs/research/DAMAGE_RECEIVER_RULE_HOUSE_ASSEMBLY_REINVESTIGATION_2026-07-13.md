# Damage Receiver Rule, House, and Type Assembly Reinvestigation

Date: 2026-07-13  
Mode: `/re-investigate` coverage-map  
Status: **PARTIAL — G1 FAILED**  
Parity target: active Yuri's Revenge `gamemd.exe`

## Verdict

The native assembly chain is now pinned for the receiver's principal rule/type/house inputs: 18-byte Veteran/Elite ability arrays; Object/Techno type immunity bytes; readiness and ammo type fields; difficulty, country, house, category, per-instance, and rank multipliers; the global damage thresholds/list/sound; and the verified Warhead gates. The current Rust model owns only a subset. In particular, it lacks the full ability arrays, most receiver immunities, native House combat-modifier assembly, `VeteranCombat`/`VeteranArmor`, `BuildingDamageSound`, `Sparky`, and a general signed Techno ammo/readiness state.

This report is deliberately **PARTIAL**. The bounded evidence did not close (1) every concrete-wrapper field source, because the parallel wrapper report was not available at closure, (2) every writer/save/load/reset path for Techno `+0x160` firepower and `+0x2FC` ammo, (3) the constructor fallback for `BuildingDamageSound`, or (4) whether `ParentCountry=` ever copies combat fields outside the checked reader/consumer. Those are authority-critical for an exhaustive certification, so G1 cannot pass.

## Preflight and evidence discipline

- Preflight timestamp: `2026-07-13T13:53:46+02:00`.
- The research index was current/valid. A broad filtered query returned zero useful rows, so exact strings, exact addresses, direct research documents, stock INIs, and Rust symbol reads were used.
- The sole output path did not exist at preflight or at the closure checkpoint; no same-file conflict existed.
- Static read-only Ghidra was connected to active `/gamemd.exe` in project `testProsjekt-12.1.2-test`, image base `0x00400000`, 32-bit x86.
- No debugger, UI automation, process/screen manipulation, Ghidra mutation, Cargo command, code edit, or external mod source was used.
- Stock data means `ini/rules.ini` plus `ini/rulesmd.ini` overlay. The `*md` value replaces a base value at the same section/key; base is fallback. Art follows the corresponding `art.ini` then `artmd.ini` overlay, but none of the type/house scalar rows below comes from art.

### Fresh binary calls used in this pass

- `decompile_function` / `disassemble_function`: `0x005F7090`, `0x005F92D0`, `0x00477640`, `0x00710AF0`, `0x00715460`, `0x005113F0`, `0x00511850`, `0x004F6EC0`, `0x0050BD30`.
- `search_strings`, `get_bulk_xrefs`, `get_assembly_context`, `inspect_memory_content`, and `search_instructions` for the exact keys and stores listed below.
- Fresh rule/warhead read sites: `0x0066ACAD` (`BuildingDamageSound`), `0x0066D573` (`DamageFireTypes`), `0x0066B34B`/`0x0066B372` (`ConditionRed`/`ConditionYellow`), and `0x0075D57D` (`Sparky`).
- Fresh type parse sites: `0x007154A3`/`0x007154E8` (ability arrays), `0x0071220F` (`TypeImmune`), `0x00714FA7`/`0x00714FC8` (`ImmuneToPsionics`/`ImmuneToPsionicWeapons`), `0x00714D53`/`0x0071504C` (`ImmuneToRadiation`/`ImmuneToPoison`), `0x007148BF`/`0x007148E0` (readiness), and `0x00714755` plus the adjacent `Ammo` parse (ammo fields).

Existing research was used only where its cited binary audit was stronger than reopening a closed branch. Important anchors are `DAMAGE_RECEIVER_OBJECT_CALLBACKS_REINVESTIGATION_2026-07-13.md`, `DAMAGE_RECEIVER_TECHNO_GATES_REINVESTIGATION_2026-07-13.md`, `RULESCLASS_DIFFICULTY_SLOTS.md`, `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` as audited in `AUDIT_LOG.md`, `VETERANCY_SYSTEM_GHIDRA_REPORT.md`, and `CRATE_SYSTEM_GHIDRA_REPORT.md`.

## Target question, non-goals, COMPLETE bar, and stop conditions

**Target question:** For every rule/type/house/rank/runtime field consumed by the verified Object/Techno damage receiver and its concrete wrappers, what is the native field, parser source, constructor fallback, overlay/inheritance order, stock merged value, mutable runtime writer chain, and current Rust owner or missing surface?

**Non-goals:** callback ordering; full concrete death/state machines; damage-kernel re-derivation; Rust implementation; live debugging; render composition beyond rule/list inputs; external mod behavior.

**Evidence required for COMPLETE:** every row needs a native read, native constructor/parser or verified non-INI source, stock merged value, full runtime writer and save/load/reset chain when mutable, concrete-wrapper read coverage, and a Rust owner/handoff. No authority-critical `UNKNOWN` may remain.

**Stop conditions:** stop on the approved receiver boundary; do not expand callback/state machines; stop opening new evidence branches at the parent closure checkpoint; report unresolved rows instead of inferring them.

## 1. Native construction and overlay order

The verified order for these fields is:

1. The appropriate base and concrete type constructors establish byte/dword/float/double defaults.
2. Stock base rules are loaded, then YR `rulesmd.ini` overlays the same section/key. A missing key preserves the value passed as the INI reader's fallback.
3. `ObjectTypeClass::ReadINI @ 0x005F92D0` fills base object fields; `TechnoTypeClass::ReadINI` fills Techno fields; concrete Aircraft/Infantry/Unit/Building readers follow for their fields.
4. A **present** `VeteranAbilities` or `EliteAbilities` value does not append to an old array: parser `0x00477640` zeroes an 18-byte local array, sets recognized indices, then copies all 18 bytes to the destination. A missing key copies the prior/default 18 bytes.
5. House-country scalars are constructed neutral, read from each country section, then `HouseClass::SetDifficulty @ 0x004F6EC0` assembles House doubles. Per-category armor floats remain on `HouseTypeClass` and are read live by `0x0050BD30`.
6. Runtime crate, promotion, firing/reload, and save/load paths may then change per-instance values.

`HouseTypeClass::ReadINI @ 0x00511850` stores `ParentCountry=` at `+0x98` and resolves a parent index, but the checked combat reader and `SetDifficulty` use the current country object's own fields. No combat-field copy from a parent was proven in this bounded pass. Stock countries omit all relevant country combat keys and remain neutral, so this uncertainty does not change the listed stock result; it remains open for modded parent-country behavior.

## 2. Type/default/source/stock/Rust matrix

Evidence grades: **B** = fresh binary this pass; **A** = previously binary-audited document; **I** = stock merged INI; **R** = current Rust read.

| Native field | Constructor fallback and reader semantics | Stock merged result | Current Rust owner / gap |
|---|---|---|---|
| `TechnoType+0x29C..0x2AD` `VeteranAbilities[18]` | All 18 bytes zero in `0x00710AF0`; `0x00477640` replaces all 18 bytes when key is present. **B** | 70 explicit live rows. All 70 set index 1 `STRONGER`; 69 set index 2 `FIREPOWER`; omitted types retain all-zero. **I** | `src/rules/object_type.rs:1049` extracts only `FEARLESS`; full array and indices 0..17 are **MISSING**. **R** |
| `TechnoType+0x2AE..0x2BF` `EliteAbilities[18]` | Same zero/default and whole-array replacement. The correct Elite base is `+0x2AE`, not stale `+0xAB8`. **B** | 69 explicit live rows. All 69 set `STRONGER`; 68 set `FIREPOWER`; `TELE` is the cold exception. **I** | `src/rules/object_type.rs:1050` extracts only `FEARLESS`; full array **MISSING**. **R** |
| ability index 1 `STRONGER` | Byte 1 in each array; receiver selects Veteran or Elite byte by rank. **B** | See counts above. Rules scalar is `VeteranArmor=1.5` (`rulesmd.ini:19`). **I** | Ability byte and `VeteranArmor` scalar **MISSING**. Damage shadow accepts a preassembled value only. **R** |
| ability index 2 `FIREPOWER` | Byte 2 in each array; fire path selects rank byte. **B/A** | See counts above. Rules scalar is `VeteranCombat=1.1` (`rulesmd.ini:16`). **I** | Ability byte and `VeteranCombat` scalar **MISSING**. **R** |
| `ObjectType+0x233` `Immune` byte | `false` at `0x005F7090`; `ReadBool("Immune", current)` at `0x005F92D0`. **B** | 19 explicit: 18 `yes`; `CAARMR=no`; all omissions `false`. **I** | Standard `ObjectType` has no field/parser. **MISSING**. **R** |
| `ObjectType+0x232` `Insignificant` byte | `false`; `ReadBool("Insignificant", current)`. **B** | 396 explicit: 387 `yes`, 9 `no`; omissions `false`. **I** | Only `TerrainObjectType` has a separate default-true field (`terrain_object_type.rs:42-86`); standard receiver type field **MISSING**. **R** |
| `TechnoType+0xC8C` `TypeImmune` byte | Parent TechnoType ctor `false`; current-value `ReadBool`. **B** | `DLPH=yes`, `YURI=yes`; all omissions `false`. **I** | Type schema **MISSING**; value-type shadow input exists at `sim/combat/damage/mod.rs:97` but is not sourced live. **R** |
| `TechnoType+0xD35` `ImmuneToPsionics` byte | Aircraft/Infantry/Unit ctor `false`; Building ctor `true`; current-value override. **B** | 109 explicit: 37 `yes`, 72 `no`; omissions keep the class-specific fallback. **I** | **MISSING**. **R** |
| `TechnoType+0xD36` `ImmuneToPsionicWeapons` byte | Aircraft/Infantry/Unit `false`; Building `true`; current-value override. **B** | `BORIS=no`, `YURIPR=yes`; omissions keep class fallback. **I** | **MISSING**. **R** |
| `TechnoType+0xD37` `ImmuneToRadiation` byte | Parent TechnoType ctor `false`; current-value override. **B** | 42 explicit: 10 `yes`, 32 `no`; omissions `false`. **I** | Parsed at `object_type.rs:1097`, copied to runtime `GameEntity::immune_to_radiation`; this is the one receiver immunity with a live owner. **R** |
| `TechnoType+0xD3B` `ImmuneToPoison` byte | Aircraft/Infantry/Unit/Building ctor `false`; current-value override. **B** | `VIRUS=yes`; all omissions `false`. **I** | Type field **MISSING**; damage shadow has an unsourced `poison_immune` input. **R** |
| `TechnoType+0x6B1` `DamageReducesReadiness` byte | `false`; `ReadBool(current)`. **B** | No live stock key; `false` for all stock types. **I** | **MISSING**. **R** |
| `TechnoType+0x6B4` `ReadinessReductionMultiplier` float32 | `0.0f`; parsed through `ReadDouble` then stored float32. **B** | No live stock key; `0.0`. **I** | **MISSING**. **R** |
| `TechnoType+0x680` `InitialAmmo` int32 | `-1`; `ReadInt(current)`. **B** | No live stock assignment; `-1`. **I** | **MISSING**. **R** |
| `TechnoType+0x684` `Ammo` int32 | `-1`; `ReadInt(current)`. **B** | 13 explicit: eight `1`, three `5`, two `100`; all other types `-1`. **I** | Parsed at `object_type.rs:1034`, but modeled as aircraft ammo only. General Techno max-ammo ownership is **MISSING**. **R** |
| `Techno+0x2FC` current ammo/readiness int32 | Non-INI runtime field; verified initialization is `InitialAmmo` when not `-1`, else `Ammo`. Receiver performs signed int32 readiness math and lower-clamps only. **A/B** | Normally `-1` for unlimited types; finite stock types use the row above. | `GameEntity` has aircraft-only optional ammo (`game_entity.rs:368-370`); no general signed Techno readiness field. **MISSING/DRIFT**. **R** |
| ability index 9 `SELF_HEAL` | Independent Veteran/Elite ability byte. **B** | Authored in many Elite arrays. | Full ability byte **MISSING**. |
| ability index 14 `C4` | Independent Veteran/Elite ability byte; not the Infantry type C4 flag. **B** | Authored according to ability lists, not a combined self-heal flag. | Full ability byte **MISSING**. |
| `InfantryType+0xEC2` `C4` byte | Infantry type flag; constructor false, INI `C4=` override in prior verified C4 research. **A** | Stock C4 infantry include GHOST/TANY; other omissions false. | Parsed at `object_type.rs:1173`; live C4 intent exists. **R** |
| `BuildingType+0x1577` `CanC4` byte | Building ctor writes `true` at `0x0045E063`; `CanC4=` may override. **A** | `AMMOCRAT`, `CAMISC01`, `CAMISC02`, `CAMISC06` are `no`; other buildings default true. | Parsed with category-specific default at `object_type.rs:1174`; live field exists. **R** |
| `SelfHealC4` | No exact binary string, INI key, or native field was found. It is not a verified combined mechanism. **B/I** | No stock key. | Correct action is **do not add it**. Model `SELF_HEAL` ability, ability-index `C4`, Infantry `C4=`, and Building `CanC4=` separately. |

### Receiver use and readiness formula

The verified Techno receiver reads the rows above in this order-sensitive region:

- readiness side effect first: with `DamageReducesReadiness`, current ammo `Techno+0x2FC`, max ammo `Type+0x684`, Strength `Type+0xA0`, and multiplier `Type+0x6B4`, it computes the x87 expression documented in the Techno receiver report, converts once with `ftol`, and clamps only the lower bound to zero; negative damage can increase ammo above max;
- schedules/restarts reload through `0x006FB080` using `EmptyReload +0x69C`, `PipWrap +0x3E4`, `Reload +0x698`, and `ReloadIncrement +0x6A0`;
- then evaluates Radiation/`ImmuneToRadiation`, PsychicDamage/`ImmuneToPsionicWeapons`, Poison/`ImmuneToPoison`, alliance, and Psychedelic/`ImmuneToPsionics` gates;
- later applies `TypeImmune` and the Object receiver's `Immune`/`CanC4` rules.

The side effect can therefore occur even when a later immunity zeros the hit. A Rust implementation must not collapse these fields into one `immune` predicate evaluated before readiness.

## 3. Difficulty, country, house, category, instance, and rank assembly

### Native fields and stock values

| Source | Native field/default | Stock merged value | Native consumer/assembly | Rust owner / gap |
|---|---|---|---|---|
| Difficulty FirePower | `Rules+0x1538 + difficulty*0x50`, double; Difficulty reader fallback `1.0`. **A** | `Firepower` is omitted in Easy/Normal/Difficult, so all are `1.0`. **I** | `SetDifficulty` stores House `+0x188`; SP uses difficulty only, MP multiplies country `+0xC8`. **B** | No DifficultyClass combat schema or House assembly. **MISSING**. |
| Difficulty Armor | same block `+0x18`, double, fallback `1.0`. **A** | Easy `1.2`, Normal `1.0`, Difficult `0.8` (`rulesmd.ini:3449,3461,3474`). **I** | House `+0x1A0`; SP difficulty only, MP multiplies country `+0xE0`. **B** | **MISSING**. |
| Country global Firepower | `HouseType+0xC8` double, ctor `1.0`; `Firepower=` with current fallback. **B** | All stock country sections omit it: `1.0`. **I** | Applied once by MP `SetDifficulty`; ignored in SP. | `CountryRules` contains only `multiplay_passive` and `income_ppm` (`ruleset.rs:41-79`). **MISSING**. |
| Country global Armor | `HouseType+0xE0` double, ctor `1.0`; `Armor=`. **B** | All stock countries omit: `1.0`. **I** | Applied once by MP `SetDifficulty`; ignored in SP. | **MISSING**. |
| `ArmorInfantryMult` | `HouseType+0x100` float32, ctor `1.0f`. **B** | No stock live key: `1.0`. **I** | Live `GetArmorMultForType`, WhatAmI `0x10`. **B** | **MISSING**. |
| `ArmorUnitsMult` | `+0x104` float32, `1.0f`. **B** | `1.0`. | WhatAmI `0x28`. | **MISSING**. |
| `ArmorAircraftMult` | `+0x108` float32, `1.0f`. **B** | `1.0`. | WhatAmI `3`. | **MISSING**. |
| `ArmorBuildingsMult` | `+0x10C` float32, `1.0f`. **B** | `1.0`. | WhatAmI `7`, unless defense category below. | **MISSING**. |
| `ArmorDefensesMult` | `+0x110` float32, `1.0f`. **B** | `1.0`. | Building whose type `+0xE08 == 5`. | **MISSING**. |
| House Firepower | `House+0x188` double, ctor neutral; written by `SetDifficulty`. **B/A** | SP and MP stock: `1.0`, because difficulty and country are neutral. | Attacker fire path multiplies base damage by House `+0x188`, Techno `+0x160`, then rank/ability effects at their verified points. | Damage shadow accepts `attacker_country_firepower`, but no live House owner/assembly (`damage/mod.rs:45-49`). |
| House Armor | `House+0x1A0` double; written by `SetDifficulty`. **B** | Easy `1.2`, Normal `1.0`, Difficult `0.8` in both SP and stock MP. | Receiver divides by `House armor × category float × Techno+0x158`, with native grouping/conversions from the Techno report. | No live owner/assembly. **MISSING**. |
| Techno per-instance armor | `Techno+0x158` double, ctor `1.0`. **A** | `1.0`, or armor crate multiplier `1.5`. | Receiver divisor before VeteranArmor. | No live field/writer. **MISSING**. |
| Techno per-instance firepower | `Techno+0x160` double, ctor `1.0`. **A** | `1.0`, or firepower crate multiplier `2.0`. | Attacker fire path. | No live field/writer. **MISSING**. |
| Rank | `Techno+0x150` float32, ctor `0.0`; thresholds Veteran `>=1`, Elite `>=2`. **A** | Spawn/rules/scenario/crate dependent. | Selects Veteran/Elite array; `STRONGER` and `FIREPOWER` then gate their Rules scalar. | `GameEntity` has a `u16` veterancy model, not the native float/write contract. **DRIFT/UNCHECKED**. |

Fresh `decompile_function(0x004F6EC0)` settles an older naming confusion: the output order is House `+0x188 Firepower`, `+0x190 Groundspeed`, `+0x198 Airspeed`, `+0x1A0 Armor`, `+0x1A8 ROF`, `+0x1B0 Cost`, `+0x1B8 BuildTime`. Country doubles are applied only when `g_GameMode != 0`; category armor floats are separate and live.

### Exact stock armor and firepower folds

For the verified damage-receiver slice, stock neutral country/category/instance values reduce the common case but do not remove the mechanism:

```text
HouseFirepower(SP) = Difficulty.FirePower
HouseFirepower(MP) = Difficulty.FirePower * Country.Firepower

HouseArmor(SP) = Difficulty.Armor
HouseArmor(MP) = Difficulty.Armor * Country.Armor

Receiver pre-veteran armor divisor
  = HouseArmor * CountryCategoryArmor(type) * TechnoArmorInstance

Attacker firepower inputs
  = HouseFirepower * TechnoFirepowerInstance
    * (VeteranCombat only when rank-selected FIREPOWER byte is set)

Receiver veteran armor stage
  = separate divide/conversion by VeteranArmor
    only when rank-selected STRONGER byte is set
```

Do not algebraically merge the native armor stages without proving the same conversion points. The Techno receiver report verified a first divide/`ftol` for house/category/instance armor and a separate veteran-armor divide/`ftol`.

## 4. Runtime writer, initialization, save/load, and reset inventory

| Mutable value | Verified initialization and writers | Save/load/reset/removal | Closure |
|---|---|---|---|
| House `+0x188` Firepower, `+0x1A0` Armor | House ctor neutral; `SetDifficulty @ 0x004F6EC0` writes both at house creation/difficulty changes. Existing caller audit includes scenario house creation, takeover, and difficulty event paths. | Normal House serialization was not reopened; reapplication/reset coverage was bounded to known `SetDifficulty` callers. | **PARTIAL** |
| Techno `+0x158` armor double | Techno ctor `1.0`; armor crate path around `0x00482D56..0x00482EC0` changes only a neutral `1.0` target and applies stock Powerups Armor `1.5`. | Generic raw Techno save/load preserves it; no transient status writer found in the bounded sweep. Destruction removes the object rather than resets this field. | **COMPLETE for bounded receiver arithmetic** |
| Techno `+0x160` firepower double | Techno ctor `1.0`; crate table identifies the active firepower crate writer and stock Powerups multiplier `2.0`. | Exact writer condition, exhaustive writer set, raw serialization, and any reset/removal semantics were not re-closed in this task. | **PARTIAL — G1 blocker** |
| Techno `+0x150` veterancy float | ctor `0.0`; `AddExperience @ 0x0074FF50`; `SetRookie @ 0x00750080` writes `0.0`; `SetVeteran @ 0x00750090` writes `1.0` or `0.0` by flag; `SetElite @ 0x007500B0` writes `2.0` or `0.0`. Verified callers include country veteran lists, spy future bonuses, team/script rank, scenario placement, creation/undeploy paths, and the crate promotion. | Generic raw save/load preserves the float. Setter false-demotion semantics are real; do not assume setters only promote. | **COMPLETE for bounded rank input** |
| Techno `+0x2FC` ammo/readiness int32 | InitFromType selects `InitialAmmo` unless `-1`, otherwise `Ammo`; verified writers include readiness damage, firing/decrement paths, and reload increments/timer coordination. | Exhaustive class-specific writer set, save/load representation, spawn/reset/removal behavior, and the collision of class-specific `+0x2FC` meanings were not closed. | **PARTIAL — G1 blocker** |

## 5. Global Rules, Warhead inputs, and presentation-source rows

| Input | Native source/default/stock | Receiver/wrapper role | Current Rust owner / gap |
|---|---|---|---|
| `ConditionYellow` | Rules `+0x1700` double; reader uses current double; stock `[AudioVisual] 50%` (`rulesmd.ini:753`). Existing binary docs and constructor/read evidence give default `0.5`. | Damage-state/postlude threshold and damage-fire selection. | `ruleset.rs:285-293,1079-1084` stores f32 plus x1000 integer. Native is double; exact equivalence is unproven, so **DRIFT/UNCHECKED**. |
| `ConditionRed` | Rules `+0x1708` double; stock `25%` (`rulesmd.ini:752`), default `0.25`. | Red-state threshold. | Same f32/x1000 mechanism mismatch. |
| `BuildingDamageSound` | AudioVisual key parsed at `0x0066ACAD` to a VOC index at Rules `+0x714`, preserving the old handle when missing/lookup fails; stock `BuildingDamaged` (`rulesmd.ini:701`). Constructor fallback was not closed. | Concrete Building damage presentation. | No Rules field or audio handoff found. **MISSING**. |
| `DamageFireTypes` | Native read site `0x0066D573` reads the General-section key into the Rules dynamic list; stock YR `[General] FIRE01,FIRE02,FIRE03` (`rulesmd.ini:519`). The base `[AudioVisual]` spelling is not the section consumed at this site. | Building damage-fire animation selection/list count. | `ruleset.rs:380,1281-1292,1498-1505` owns names and resolves art rates. Exact native list/RNG consumer order was outside this task. |
| Warhead `Sparky +0x14A` | Warhead ctor `false`; `ReadBool(current)` at `0x0075D57D`; 28 explicit stock rows are all `no`, omissions remain false. **A/B/I** | Concrete Building foundation spark/debris branch. | `WarheadType` has no `sparky`. **MISSING**. |
| `PenetratesBunker +0x146` | Warhead ctor false; INI override. **A** | Linked-bunker forwarding gate in Techno receiver. | **MISSING**. |
| `Poison +0x156` | ctor false; INI override. **A** | Pairs with target `ImmuneToPoison`. | Parsed at `warhead_type.rs:187`; target immunity/source still missing. |
| `Psychedelic +0x16D` | ctor false; INI override. **A** | Pairs with `ImmuneToPsionics`, then building-specific branch. | Parsed at `warhead_type.rs:184`; target immunity missing. |
| `Radiation +0x177` | ctor false; INI override. **A** | Pairs with `ImmuneToRadiation`. | Parsed at `warhead_type.rs:190`, though its source comment uses a stale offset. |
| `PsychicDamage +0x178` | ctor false; INI override. **A** | Pairs with `ImmuneToPsionicWeapons`. | **MISSING**. |
| `AffectsAllies +0x179` | **only receiver-relevant Warhead boolean here whose ctor default is true**; INI override. **A** | When false, allied target gate may zero the hit. | Damage shadow has an input (`damage/mod.rs:112`) but `WarheadType` parser/storage is **MISSING**. |
| Foot wrapper Warhead `Sonic +0x14B` | ctor false; INI override. The wrapper read is verified, but the full concrete source matrix was not delivered at closure. | Concrete Foot consequence gate. | Parsed/storage ownership not found in current `WarheadType`. **PARTIAL**. |

`WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` was audited clean against constructor `0x0075CEC0` and reader `0x0075D3A0`: all relevant Warhead booleans default false except `AffectsAllies=true`. This supersedes stale offset comments in current Rust and older splash prose.

## 6. Current Rust handoff chain

The missing ownership is not confined to one function. The implementation chain must be closed in this order:

1. **Rules/type schemas:** add native-width fields or exact semantic equivalents for both 18-byte arrays, object/type immunities, readiness fields, `InitialAmmo`, country/difficulty/category combat modifiers, veteran scalars, `BuildingDamageSound`, `Sparky`, and missing Warhead gates.
2. **INI merge:** preserve base then md overlay and current-value fallback; present ability lists replace all 18 bytes rather than append.
3. **House assembly:** create the three DifficultyClass blocks and build House `Firepower`/`Armor` with the SP-vs-MP country rule; retain live per-category floats.
4. **Entity initialization:** create signed general Techno current ammo from `InitialAmmo`/`Ammo`; initialize per-instance armor/firepower doubles and float veterancy with native writer semantics.
5. **Runtime writers/serialization:** implement crate multipliers, rank setters/promotions, firing/reload/readiness writers, and exact save/load/reset behavior before treating receiver inputs as stable.
6. **Damage request/receiver:** source live values into the receiver in verified order, preserving readiness-before-immunity, class-specific psionic defaults, two armor conversion stages, and Object `Immune`/`CanC4` behavior.
7. **Presentation:** route `ConditionYellow`/`ConditionRed`, `BuildingDamageSound`, `DamageFireTypes`, and `Sparky` to deterministic events without moving gameplay logic into render/audio.

The existing `src/sim/combat/damage/mod.rs` is a value-type shadow with several useful input slots, but it is not a live authority: comments still contain stale offset identities, many fields are unsourced, and the production damage path does not own the native assembly chain.

### Minimum acceptance fixtures for a later implementation contract

- `TELE` Veteran and Elite arrays: `STRONGER` set, `FIREPOWER` absent in both, proving exact index selection and no inferred ability.
- One Building and one non-Building with no psionic keys: Building defaults both psionic immunities true; non-Building false.
- An explicit Building `ImmuneToPsionics=no` overrides its true constructor fallback.
- `DLPH`/`YURI` TypeImmune and `VIRUS` ImmuneToPoison reach their exact receiver gates.
- Negative damage with readiness enabled can increase current ammo above max; a later immunity still leaves the readiness side effect.
- Easy/Normal/Difficult House armor produces `1.2/1.0/0.8`; SP ignores a non-neutral country scalar, MP applies it.
- Infantry/Unit/Aircraft/Building/Defense category lookup selects `+0x100/+0x104/+0x108/+0x10C/+0x110` exactly.
- Armor crate, firepower crate, promotion, demotion, save/load, and object removal preserve/reset only the verified values.
- `AMMOCRAT` or another `CanC4=no` Building exercises the Object receiver's post-kernel minimum-one rule, including a negative/healing request.
- Building thresholds use native double semantics; `BuildingDamageSound` and `DamageFireTypes`/`Sparky` produce the verified presentation events.

## 7. Stale or unsafe wording to supersede

- `ObjectType+0x233` is **Immune**, while `+0x232` is **Insignificant**. Older Building damage prose that calls `+0x233` Insignificant is wrong.
- Elite abilities are `TechnoType+0x2AE..0x2BF`, not `+0xAB8..+0xAC9`.
- Building and non-Building psionic defaults are not uniform: Building defaults `ImmuneToPsionics=true` and `ImmuneToPsionicWeapons=true`; Aircraft/Infantry/Unit default false.
- Building `CanC4` is `+0x1577`, default true. `+0x16A9` is not CanC4.
- `InitialAmmo +0x680`, max `Ammo +0x684`, and runtime current ammo `Techno+0x2FC` are distinct.
- `SelfHealC4` is not a verified field. Do not collapse `SELF_HEAL`, ability-index `C4`, Infantry `C4=`, and Building `CanC4=`.
- `DamageFireTypes` is read from the General section at the verified native site; do not let the base file's same key under AudioVisual establish the native source section.
- Warhead `Radiation/Poison/PsychicDamage/AffectsAllies` offsets in current Rust comments are stale relative to the audited active binary layout.
- Do not confuse the virtual method slot `vtable+0x160` (active invulnerability query in the receiver report) with the unrelated data field `Techno+0x160` (per-instance firepower double).
- Older country-multiplier prose that relabels House `+0x190/+0x198/+0x1A0` is superseded by fresh `SetDifficulty`: Groundspeed, Airspeed, Armor respectively.

## 8. Coverage ledger and final questions

| ID | Question | Result |
|---|---|---|
| Q01 | Exact Veteran array width/base/default? | **RESOLVED**: 18 bytes at `+0x29C`, all zero. |
| Q02 | Exact Elite array width/base/default? | **RESOLVED**: 18 bytes at `+0x2AE`, all zero. |
| Q03 | Does a present ability key merge or replace? | **RESOLVED**: replaces all 18 bytes. |
| Q04 | STRONGER/FIREPOWER indices? | **RESOLVED**: 1 and 2. |
| Q05 | Object Immune vs Insignificant offsets/defaults? | **RESOLVED**: `+233/+232`, both false. |
| Q06 | TypeImmune source/default/stock? | **RESOLVED**. |
| Q07 | Building vs non-Building psionic defaults? | **RESOLVED** for all four concrete Techno type ctors. |
| Q08 | Radiation/Poison type immunities? | **RESOLVED**. |
| Q09 | Readiness fields, widths, defaults, stock? | **RESOLVED**. |
| Q10 | Initial/max/current ammo source? | **RESOLVED** for construction and receiver reads. |
| Q11 | Exhaustive ammo writers/save/load/reset? | **DEFERRED / G1 blocker**. |
| Q12 | Difficulty Firepower/Armor defaults and stock? | **RESOLVED**. |
| Q13 | Country Firepower/Armor/category fields/defaults? | **RESOLVED**. |
| Q14 | SP vs MP country application? | **RESOLVED**. |
| Q15 | Exact category dispatch? | **RESOLVED**. |
| Q16 | Techno armor instance writers/serialization? | **RESOLVED for bounded field**. |
| Q17 | Techno firepower writers/serialization/reset? | **DEFERRED / G1 blocker**. |
| Q18 | Veterancy writers/promotion/demotion/save-load? | **RESOLVED for bounded field**. |
| Q19 | Condition thresholds and stock? | **RESOLVED**; Rust width differs. |
| Q20 | BuildingDamageSound source/stock/default? | **PARTIAL**: source/stock resolved, ctor fallback open. |
| Q21 | DamageFireTypes native section/list and stock? | **RESOLVED for source/list**; concrete RNG consumer excluded. |
| Q22 | Sparky and receiver Warhead defaults? | **RESOLVED**. |
| Q23 | SelfHealC4 real? | **RESOLVED negative**: no such combined field/key. |
| Q24 | Every concrete-wrapper field source covered? | **DEFERRED / G1 blocker**: parallel wrapper report absent at closure. |
| Q25 | ParentCountry combat-field inheritance? | **UNCHECKED outside checked reader/consumer; G1 blocker for mod space**. |

### Adversarial checks

1. **Could `+0x233` still be Insignificant?** No. Fresh ctor and ReadINI stores distinguish `+0x232 Insignificant` and `+0x233 Immune`.
2. **Could Elite abilities really live at stale `+0xAB8`?** No. Fresh constructor and reader assembly write exactly `+0x2AE..+0x2BF`.
3. **Could all psionic immunities default false?** No. Four concrete ctor sweeps show Buildings write `D35=D36=1`; Aircraft/Infantry/Unit write zero.
4. **Could country combat multipliers apply in every game mode?** No. Fresh `SetDifficulty` branches on `g_GameMode`; country doubles occur only in the nonzero/MP branch.
5. **Could `SelfHealC4` be a hidden alias?** No exact binary/INI string exists, while four distinct verified mechanisms account for the likely conflation.
6. **Could present Veteran/Elite lists add to a previous list?** No. Parser zeroes the temporary 18 bytes before setting tokens.
7. **Could base `[AudioVisual] DamageFireTypes` define the native section?** No. The fresh native site reads the General-section handle; YR stock supplies the General key.

### Cold spot checks

- **Cold spot 1 — TELE:** merged stock arrays keep `STRONGER` but omit `FIREPOWER`, so an implementation that assumes every ranked type gets both bonuses will be observably wrong.
- **Cold spot 2 — CanC4=no healing:** Object receiver applies the Building/CanC4 minimum-one rule after armor normalization; for the four stock `CanC4=no` buildings it can turn zero or negative/healing input into `+1` damage.

### Zero-add pass

A final mandatory-key/doc/INI/Rust scan added no new named rule/type/house category beyond the rows above. A binary zero-add certification for every concrete wrapper could not be performed because the parallel wrapper field-source report was not present at closure. That failure is retained as Q24 and is why the result is not COMPLETE.

## 9. Do-not-do and remaining uncertainty

Do not:

- invent a `SelfHealC4` field;
- treat stock-neutral country/category values as proof the mechanism may be omitted;
- parse only FEARLESS from the ability arrays;
- use one global psionic default for every class;
- evaluate immunity before readiness;
- fold the two native armor conversion stages into one expression without exhaustive equivalence proof;
- model native double thresholds as f32/x1000 and call it parity without proof;
- wire raw Ghidra labels or stale offset comments into Rust;
- reuse current `+0x2FC` class-specific meanings without a class-aware ownership audit.

Authority-critical uncertainties remaining:

1. exhaustive concrete-wrapper field/source matrix;
2. exhaustive Techno `+0x160` firepower writer/save/load/reset/removal chain;
3. exhaustive class-specific `+0x2FC` ammo writer/save/load/reset/removal chain;
4. `BuildingDamageSound` constructor fallback when the key is absent;
5. any combat-field inheritance performed from `ParentCountry=` outside `HouseTypeClass::ReadINI` and `HouseClass::SetDifficulty`.

Until those five items are closed, this document is an implementation handoff and disparity source, **not a parity certificate**.
