# Tank Bunker Combat Surface - Ghidra Research Report

Date: 2026-05-23

Target question: For a unit installed in a stock `NATBNK` tank bunker through the reciprocal `BuildingClass+0x2E4` / unit `TechnoClass+0x2E4` link, what combat behavior does gamemd.exe apply for firing source/owner, weapon selection, range, ROF, outgoing damage, interaction with open-topped and civilian garrison modifiers, and incoming target/damage routing?

Non-goals: Tank-bunker entry/exit lifecycle except the already-verified reciprocal `+0x2E4` state needed to identify combat branches; pathfinding row-helper behavior; bunker wall animations/sounds; civilian `CanBeOccupied` lifecycle; IFV/Gunner rendering; broad combat system parity outside the branches below.

Evidence needed to mark COMPLETE: Decompile plus disassembly/assembly context for `RulesClass` key reads, `TechnoClass::GetROF`, `TechnoClass::InRange`, `TechnoClass::Fire_At`, and `TechnoClass::ReceiveDamage`; caller/xref proof that these are live combat surfaces; INI/default evidence for stock YR values and `PenetratesBunker`; Rust scan; implementation handoff with test-name proposals.

Stop conditions: Stop when the combat branches gated by unit/building `+0x2E4` are verified and separable from garrison/open-topped branches, or record uncertainty about target acquisition/damage forwarding; do not re-study lifecycle beyond the reciprocal link.

**Address(es):** `0x0066BBB0`, `0x006FCFA0`, `0x006F7220`, `0x006FDD50`, `0x00701900`, `0x004526F0`, `0x0075D3A0`, `0x00458E50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Combat math and damage-routing behavior for objects with an active bunker `+0x2E4` link, plus negative separation from civilian garrison and open-topped transport branches.
**Non-Scope:** Lifecycle, row helpers, projectile pathing in every projectile type, and UI cursor dispatch beyond damage-routing implications.
**Confidence:** High for modifiers and ReceiveDamage routing; Medium for global target retargeting because only enough projectile/selection context was touched to support the handoff.
**Active in YR:** Conditional. Active when a standard YR `Bunker=yes` building such as checked stock `[NATBNK]` has an installed unit and the reciprocal links are nonzero.

## 1. Overview

Tank-bunker combat is not a building-garrison weapon path. The firing object remains the installed unit; the unit's own weapon lookup, owner, ROF, range, and damage pipeline run with extra branches gated by `unit+0x2E4 != 0` and `WhatAmI() != Building`.

Civilian garrison uses `BuildingClass::IsOccupied()` and the occupant vector at `+0x684`; tank bunker uses the single reciprocal link at `+0x2E4`. Open-topped transport uses `TechnoClass+0x82`. The binary evaluates these as independent branches, so bunker and open-topped bonuses can stack if both flags are present, while normal civilian garrison is naturally excluded from bunker application by the `WhatAmI() != Building` guard.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| Unit `TechnoClass` | `+0x2E4` (`param_1[0xB9]`) | Nonzero containing bunker pointer; combat gate for bunkered unit modifiers and damage shielding | Install writer `0x0045930F`; combat reads in `0x006FCFA0`, `0x006F7220`, `0x006FDD50`, `0x00701900` | Conditional |
| `BuildingClass` | `+0x2E4` | Installed unit pointer on the bunker shell | Install writer `0x00459301`; ReceiveDamage compares same-cell building lookup to unit's `+0x2E4` at `0x00701BD9` | Conditional |
| `RulesClass` | `+0xF4C` | `BunkerDamageMultiplier` float | parser store `0x0066C6F9`; Fire_At multiply `0x006FE42A` | Yes, when bunkered unit fires |
| `RulesClass` | `+0xF50` | `BunkerROFMultiplier` float | parser store `0x0066C71D`; GetROF divide `0x006FD1E4` | Yes, when nonzero and bunkered unit fires |
| `RulesClass` | `+0xF54` | `BunkerWeaponRangeBonus` int cells | parser store `0x0066C73D`; InRange add `0x006F72BD..0x006F72C6` | Yes, when bunkered unit checks range |
| `TechnoClass` | `+0x82` | In-open-topped-transport flag | InRange add `Rules+0xF5C` at `0x006F72C8..0x006F72E1`; Fire_At multiply `Rules+0xF58` at `0x006FE43B..0x006FE455` | Conditional |
| `WarheadTypeClass` | `+0x146` | `PenetratesBunker` | parser string `0x00847E08` -> `WarheadTypeClass__ReadINI_Body`; ReceiveDamage reads at `0x00701BBE` | Conditional per warhead |

## 3. Core Logic

### Rules Parser and Stock Defaults

`RulesClass__ReadCombatDamage @ 0x0066BBB0` reads the combat damage group in this exact sequence: `OccupyDamageMultiplier`, `OccupyROFMultiplier`, `OccupyWeaponRange`, then the bunker triple, then the open-topped values. Disassembly context confirms stores to `Rules+0xF4C`, `+0xF50`, `+0xF54`, `+0xF58`, and `+0xF5C` at `0x0066C6F9`, `0x0066C71D`, `0x0066C73D`, `0x0066C761`, and `0x0066C781`.

Stock YR `rulesmd.ini` sets `[CombatDamage]` `BunkerDamageMultiplier=1.3`, `BunkerROFMultiplier=1.3`, `BunkerWeaponRangeBonus=2`, `OpenToppedDamageMultiplier=1.2`, and `OpenToppedRangeBonus=2`. Active in YR: Yes; these are standard YR rules and live parser keys.

### Firing Source, Owner, and Weapon Selection

`TechnoClass::Fire_At @ 0x006FDD50` begins by calling `this->vtable+0x3F8` to resolve the weapon (`0x006FDD64..0x006FDD69`). For a tank-bunkered unit the `this` object is the unit, not the building, because the bunker branches later test `this+0x2E4` with `WhatAmI()!=6`.

The bullet path allocates/initializes a bullet with the firing `this` object and calls `BulletClass__SetOwner`; no alternate bunker-building source substitution was observed in the verified branch. Active in YR: Conditional, whenever the installed unit fires.

Civilian garrison is the contrasting case: `BuildingClass::GetWeapon @ 0x004526F0` first checks fire-port slots (`+0x5EC/+0x702`) and then `IsOccupied()` and the occupant vector (`+0x688/+0x69C`) to return `OccupyWeapon` / `EliteOccupyWeapon`. A tank-bunkered vehicle does not enter this branch because it is not a firing building. Active in YR: Yes for civilian garrison; No for the tank-bunkered unit's weapon selection.

### Outgoing Damage Modifier

In `TechnoClass::Fire_At`, outgoing damage modifiers run in order:

1. Veteran/elite firepower branch.
2. Civilian garrison branch: if `this->vtable+0x400` `IsOccupied()` is true, multiply by `Rules+0xF40`.
3. Tank bunker branch: if `this+0x2E4 != 0` and `WhatAmI()!=6`, multiply by `Rules+0xF4C`.
4. Open-topped branch: if `this+0x82 != 0`, multiply by `Rules+0xF58`.

Disassembly proof: garrison multiply at `0x006FE3E7..0x006FE400`; bunker gate/multiply at `0x006FE40B..0x006FE430`; open-topped gate/multiply at `0x006FE43B..0x006FE455`. Active in YR: Conditional for each flag. Bunker and open-topped stack because they are sequential independent tests. Garrison and bunker do not stack in standard play because garrison true means the firing object is a building (`WhatAmI()==6`), which fails the bunker guard.

### ROF Modifier

`TechnoClass::GetROF @ 0x006FCFA0` applies civilian garrison first, then bunker:

- If `IsOccupied()` is true, ROF divides by occupant count and then, if `Rules+0xF44 > 0.0`, divides by `OccupyROFMultiplier`. Assembly: `0x006FD150..0x006FD1A6`.
- If `this+0x2E4 != 0`, `WhatAmI()!=6`, and `Rules+0xF50 != 0.0`, ROF divides by `BunkerROFMultiplier`. Assembly: `0x006FD1B1..0x006FD1EA`.

Active in YR: Conditional for bunkered non-buildings. A stock multiplier of `1.3` shortens the interval because YR ROF is a delay/tick count.

No `OpenToppedROFMultiplier` branch exists in this function. Active in YR: No.

### Range Modifier

`TechnoClass::InRange @ 0x006F7220` starts from weapon range (`weapon+0xB4`) and returns true immediately for the `-0x200` sentinel. It then applies:

- Garrison: if `IsOccupied()`, replace range with `(halfFoundation + OccupyWeaponRange) * 256`; assembly `0x006F727A..0x006F729F`.
- Bunker: if `this+0x2E4 != 0` and `WhatAmI()!=6`, add `BunkerWeaponRangeBonus * 256`; assembly `0x006F72A2..0x006F72C6`.
- Open-topped: if `this+0x82 != 0`, add `OpenToppedRangeBonus * 256`; assembly `0x006F72C8..0x006F72E1`.

Active in YR: Conditional. Bunker/open-topped range bonuses stack additively if both instance flags are set. Civilian garrison's replacement branch can run first, but the later bunker guard excludes ordinary occupied buildings.

### Incoming Targetability and Damage Routing

`TechnoClass::ReceiveDamage @ 0x00701900` has a dedicated `this+0x2E4` block before ordinary warhead immunity and `ObjectClass::ReceiveDamage`.

For a building shell with nonzero `+0x2E4`, `WhatAmI()==6`, and `PenetratesBunker=yes`, it sets incoming damage to zero and returns without damaging the shell. Evidence: decompile plus assembly context `0x00701BB6..0x00701BC6` reads `Warhead+0x146` and branches out. Active in YR: Conditional for penetrator warheads against an occupied bunker shell.

For a non-building unit with nonzero `+0x2E4` and `PenetratesBunker=no`, it calls the unit's cell/building lookup path, compares the same-cell building to `this+0x2E4`, and if they match, zeroes incoming damage and returns. Evidence: assembly `0x00701BC8..0x00701BE7` calls `vtable+0x1BC`, `Look_up_building_in_cell`, compares to `[ESI+0x2E4]`, then writes `*damage=0`. Active in YR: Conditional for contained units hit by non-penetrating warheads.

If a non-building bunkered unit is damaged with `PenetratesBunker=yes`, the non-building branch falls through to normal damage. Active in YR: Conditional for penetrator warheads. This is the Rust-facing routing rule: non-penetrators are absorbed by the bunker shell; penetrators may damage the contained unit and do not damage the shell.

`WarheadTypeClass::ReadINI_Body` is the only string anchor for `PenetratesBunker` (`0x00847E08`), so putting `PenetratesBunker=yes` on a weapon section is not evidence that the weapon penetrates unless the referenced warhead also sets it. Active in YR: Yes for warhead parsing; conditional per stock warhead.

## 4. INI Keys

| Section | Key | Stock YR value | Effect | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `[CombatDamage]` | `BunkerDamageMultiplier` | `1.3` | Multiplies outgoing damage from non-building firer with `+0x2E4` | `rulesmd.ini:843`; `0x0066C6F9`; `0x006FE42A` | Yes, conditional on bunkered unit fire |
| `[CombatDamage]` | `BunkerROFMultiplier` | `1.3` | Divides ROF delay for non-building firer with `+0x2E4`, if nonzero | `rulesmd.ini:844`; `0x0066C71D`; `0x006FD1E4` | Yes, conditional |
| `[CombatDamage]` | `BunkerWeaponRangeBonus` | `2` | Adds cells * 256 to weapon range for non-building firer with `+0x2E4` | `rulesmd.ini:845`; `0x0066C73D`; `0x006F72BD..0x006F72C6` | Yes, conditional |
| `[CombatDamage]` | `OpenToppedDamageMultiplier` | `1.2` | Multiplies outgoing damage when `+0x82` is true | `rulesmd.ini:868`; `0x0066C761`; `0x006FE44F` | Yes, conditional |
| `[CombatDamage]` | `OpenToppedRangeBonus` | `2` | Adds cells * 256 to range when `+0x82` is true | `rulesmd.ini:867`; `0x0066C781`; `0x006F72D8..0x006F72E1` | Yes, conditional |
| Warhead section | `PenetratesBunker` | default false; stock true on several warheads such as `[Super]`, `[DiskWH]`, `[ORCAAP]` | Gates bunker-shell vs contained-unit damage routing | string `0x00847E08`; `0x0075D3A0`; `0x00701BBE` | Conditional |

## 5. Integration Points

`GetROF` is a vtable-backed live combat method; xrefs include vtable data entries. `InRange` is called by `FootClass__Greatest_Threat_Scan` and `TechnoClass__CanFireAt`. `Fire_At` is called/overridden by Aircraft, Infantry, and unit fire paths and has vtable entries. `ReceiveDamage` is called by `FootClass__ReceiveDamage` and `BuildingClass__ReceiveDamage`.

The tank-bunker combat gate relies only on the reciprocal lifecycle setting `unit+0x2E4` and `building+0x2E4`. No `SpecialFlags` or TS-only gate was found in these combat branches. Active in YR: Conditional through stock `NATBNK` lifecycle.

## 6. Current Rust Implementation Status

Rust already parses the combat keys into `GarrisonRules` (`src/rules/ruleset.rs`) and stores a building-side `GameEntity::bunker_occupant` (`src/sim/game_entity.rs`). It does not yet have a verified unit-side bunker back-reference, and the combat tick currently applies `OccupyDamageMultiplier`, `OccupyROFMultiplier`, and `OccupyWeaponRange` only for civilian garrison snapshots.

`src/sim/combat/mod.rs` does not apply `bunker_damage_multiplier`, `bunker_rof_multiplier`, `bunker_weapon_range_bonus`, `open_topped_damage_multiplier`, or `open_topped_range_bonus` to the direct combat path. `src/rules/warhead_type.rs` does not parse `PenetratesBunker`, and current damage application has no bunker shell/contained-unit routing branch.

Open-topped has partial data and weapon-override handling, but the current boarding code assigns `WeaponOverride::OpenTransport` to the transport entity rather than establishing the binary's per-passenger `+0x82` firing flag. That is a separate open-topped handoff risk; for this report, the important bunker fact is that the binary's `+0x82` branch stacks after the bunker branch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RulesClass__ReadCombatDamage` bunker/open-topped keys | verified | `0x0066BBB0`; assembly `0x0066C68F..0x0066C781`; `rulesmd.ini:843..869` | none |
| Bunkered firing source/weapon lookup | verified | `0x006FDD64..0x006FDD69`; `0x006FE40B..0x006FE430`; lifecycle writer `0x0045930F` | exact AI tick scheduling while limbo is lifecycle/non-scope |
| Bunker outgoing damage multiply | verified | decompile `0x006FDD50`; assembly `0x006FE40B..0x006FE430` | none |
| Bunker ROF divisor | verified | decompile `0x006FCFA0`; assembly `0x006FD1B1..0x006FD1EA` | none |
| Bunker range bonus | verified | decompile `0x006F7220`; assembly `0x006F72A2..0x006F72C6` | none |
| Open-topped stacking after bunker | verified | range `0x006F72C8..0x006F72E1`; damage `0x006FE43B..0x006FE455` | no open-topped ROF branch exists |
| Garrison mutual exclusion from bunker modifiers | verified | `BuildingClass::GetWeapon @ 0x004526F0`; bunker branches require `WhatAmI()!=6` | none |
| `PenetratesBunker` parse | verified | string `0x00847E08`; `WarheadTypeClass__ReadINI_Body`; parser report | none |
| Incoming damage routing | verified | `0x00701900`; assembly `0x00701BB6..0x00701BE7` | broad projectile retargeting by every projectile family not fully closed |
| Current Rust status | verified by scan | `src/rules/ruleset.rs`, `src/sim/game_entity.rs`, `src/sim/combat/mod.rs`, `src/rules/warhead_type.rs` | implement deltas below |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Where are bunker combat rules parsed? -> RulesClass+0xF4C/+0xF50/+0xF54.` (evidence: `0x0066C6F9`, `0x0066C71D`, `0x0066C73D`)
- `[RESOLVED] OQ-2 - Is the firer the bunker building or the contained unit? -> The contained unit remains the firing object; bunker branches test unit+0x2E4 and non-building type.` (evidence: `0x006FDD64..0x006FDD69`, `0x006FE40B..0x006FE430`)
- `[RESOLVED] OQ-3 - Does bunker use OccupyWeapon? -> No; OccupyWeapon is selected through BuildingClass::GetWeapon for occupied buildings, not non-building bunker occupants.` (evidence: `0x004526F0`, `0x006FE41C`)
- `[RESOLVED] OQ-4 - How is bunker outgoing damage applied? -> multiply by Rules+0xF4C after garrison branch, before open-topped branch.` (evidence: `0x006FE40B..0x006FE455`)
- `[RESOLVED] OQ-5 - How is bunker ROF applied? -> divide delay by Rules+0xF50 if nonzero.` (evidence: `0x006FD1B1..0x006FD1EA`)
- `[RESOLVED] OQ-6 - How is bunker range applied? -> add Rules+0xF54*256 after any garrison replacement branch.` (evidence: `0x006F72A2..0x006F72C6`)
- `[RESOLVED] OQ-7 - Do bunker and open-topped stack? -> Yes for damage and range because branches are independent and sequential; no open-topped ROF branch exists.` (evidence: `0x006FE40B..0x006FE455`, `0x006F72A2..0x006F72E1`)
- `[RESOLVED] OQ-8 - Is garrison mutually exclusive? -> For standard garrison, yes; the firing object is a building and fails the bunker `WhatAmI()!=6` guard.` (evidence: `0x004526F0`, `0x006FE41C`, `0x006FD1C2`, `0x006F72B3`)
- `[RESOLVED] OQ-9 - How does `PenetratesBunker` route damage? -> shell damage is zeroed for penetrators; contained-unit damage is zeroed for non-penetrators when same-cell bunker matches unit+0x2E4.` (evidence: `0x00701BBE`, `0x00701BD9..0x00701BE7`)
- `[RESOLVED] OQ-10 - Does Rust parse/apply this? -> parses bunker/open-topped combat keys, but does not apply them or parse PenetratesBunker.` (evidence: Rust scan listed in Section 6)
- `[DEFERRED] OQ-11 - Do all projectile and UI command paths retarget the bunker shell to the contained unit before damage?` (category: bounded-cost-too-high; reason: not needed to implement the verified `ReceiveDamage` routing rule, and full closure spans projectile family/UI command investigations; next-step-if-pursued: trace `WarheadTypeClass::Detonate`, `BulletClass` target resolution, and object action dispatch for a `PenetratesBunker=yes` direct-fire scenario)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bunkered unit is the firing source and uses its own normal/elite weapon; bunker bonuses key off unit-side `+0x2E4`, not `PassengerRole` or `CanBeOccupied`. | `0x006FDD64..0x006FDD69`; `0x006FE40B..0x006FE430`; install writer `0x0045930F` | missing unit-side bunker back-reference and combat snapshot flag | `src/sim/game_entity.rs`, bunker lifecycle, `src/sim/combat/mod.rs`, `src/sim/combat/combat_weapon.rs` | Add a bunker-contained firing state separate from passenger/garrison cargo; resolve the installed unit's own weapon and owner, not the building's garrison weapon. | A Grizzly installed in `NATBNK` fires `105mm`/elite weapon from its owner, not an `OccupyWeapon` or bunker-building weapon. Proposed test: `bunkered_unit_uses_own_weapon_and_owner_not_garrison_weapon` | Do not implement tank bunker as `PassengerRole::Inside` civilian garrison or as `BuildingClass::GetWeapon` occupant fire. |
| Bunker damage, ROF, and range modifiers are independent branches: damage *= `BunkerDamageMultiplier`, ROF /= `BunkerROFMultiplier` if nonzero, range += `BunkerWeaponRangeBonus*256`; open-topped damage/range stack after bunker; no open-topped ROF branch. | `0x006FE40B..0x006FE455`; `0x006FD1B1..0x006FD1EA`; `0x006F72A2..0x006F72E1`; parser `0x0066C6F9..0x0066C781` | parsed but not applied for bunker/open-topped runtime combat | `src/sim/combat/mod.rs`, `src/sim/combat/in_range.rs`, cooldown code, open-topped state | Apply modifiers in binary order and rounding/truncation equivalent to `ftol`; do not apply an open-topped ROF multiplier. | Bunkered tank with base damage 100 and stock rules deals 130 before Verses; base range 5 becomes 7 cells; base ROF 30 becomes `ftol(30/1.3)`. Proposed test: `bunkered_unit_applies_damage_rof_and_range_bonus_with_open_topped_stack` | High risk of double-applying garrison rules or inventing open-topped ROF. |
| `PenetratesBunker` is a WarheadType flag controlling shell vs contained-unit damage: penetrating warheads do not damage the shell; non-penetrating warheads do not damage the contained unit while linked to the same-cell bunker. | parser string `0x00847E08`; `ReceiveDamage @ 0x00701900`; assembly `0x00701BB6..0x00701BE7` | `WarheadType` lacks `penetrates_bunker`; damage pipeline lacks bunker routing | `src/rules/warhead_type.rs`, `src/sim/combat/mod.rs`, `combat_aoe`, direct damage events | Parse the warhead flag and route direct/AoE damage through bunker shell/occupant rules before ordinary damage application. | Non-penetrating AP hit against occupied `NATBNK` damages shell only; `ORCAAP`/`DiskWH` with `PenetratesBunker=yes` damages contained unit and leaves shell HP unchanged. Proposed test: `penetrates_bunker_routes_damage_to_occupant_nonpenetrator_to_shell` | Do not apply this to civilian `CanBeOccupied` garrison occupants or remove garrison pips. |

### Stale Docs / Follow-up Docs

- `docs/research/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace the `BunkerDamageMultiplier applied in damage accumulation` confidence line with: "`BunkerDamageMultiplier` is HIGH confidence: `TechnoClass::Fire_At @ 0x006FDD50` tests `this+0x2E4`, excludes `WhatAmI()==6`, and multiplies outgoing damage by `RulesClass+0xF4C` at `0x006FE42A`; this branch runs after garrison `Rules+0xF40` and before open-topped `Rules+0xF58`."
- `docs/research/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace "Bunker and OpenTopped can stack (additive), but Garrison is mutually exclusive with both (it overwrote the base first)" with: "Bunker and OpenTopped stack for damage and range because their branches are independent and sequential. Civilian garrison is mutually exclusive with bunker in standard play because its firing object is a building and the bunker branch requires `WhatAmI()!=6`; the garrison range replacement itself is not the exclusion mechanism."
- `docs/research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`: Replace any wording implying a live `OpenToppedROFMultiplier` or ROF branch with: "No open-topped ROF multiplier branch was found in `TechnoClass::GetROF @ 0x006FCFA0`; open-topped contributes damage (`Rules+0xF58`) and range (`Rules+0xF5C`) in the verified combat surface."
- `docs/research/WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`: Replace "`PenetratesBunker`: Damage passes through to units garrisoned in buildings" with: "`PenetratesBunker` gates tank-bunker `+0x2E4` damage routing in `TechnoClass::ReceiveDamage`: penetrators zero shell damage; non-penetrators zero contained-unit damage when the same-cell bunker matches the unit's `+0x2E4`. It is not evidence for civilian `CanBeOccupied` garrison occupant damage."

## Negative Facts / Do Not Do

- Do not apply `BunkerDamageMultiplier`, `BunkerROFMultiplier`, or `BunkerWeaponRangeBonus` to a `CanBeOccupied` building just because it has occupants. Active in YR: No for civilian garrison; evidence `WhatAmI()!=6` guards at `0x006FE41C`, `0x006FD1C2`, `0x006F72B3`.
- Do not use `OccupyWeapon` / `EliteOccupyWeapon` for tank-bunkered vehicles. Active in YR: No; evidence `BuildingClass::GetWeapon @ 0x004526F0` is the garrison building path, while bunker modifiers apply to non-building unit `this`.
- Do not make `BunkerWeaponRangeBonus` replace weapon range. Active in YR: No; evidence `ADD EDI, ECX` after loading `Rules+0xF54` at `0x006F72BD..0x006F72C6`.
- Do not invent `OpenToppedROFMultiplier`. Active in YR: No; evidence `GetROF @ 0x006FCFA0` has garrison and bunker ROF branches only in the checked modifier cluster.
- Do not implement `PenetratesBunker=yes` as "damage all infantry in any occupied building" or as garrison pip removal. Active in YR: No for this surface; evidence `ReceiveDamage` uses `this+0x2E4` routing, not the `BuildingClass+0x684` garrison vector.

## Remaining Uncertainty

The exact target-retargeting path for every projectile/UI command class was not exhaustively closed. The verified `ReceiveDamage` branches are sufficient for Rust's immediate shell-vs-contained-unit damage routing, but a later trace should follow `WarheadTypeClass::Detonate`, `BulletClass` target resolution, and object action dispatch for one stock `PenetratesBunker=yes` direct-fire scenario if projectile target identity must be pixel-identical.

## Sources

- Ghidra decompile/read-only: `RulesClass__ReadCombatDamage @ 0x0066BBB0`; `TechnoClass::GetROF @ 0x006FCFA0`; `TechnoClass::InRange @ 0x006F7220`; `TechnoClass::Fire_At @ 0x006FDD50`; `TechnoClass::ReceiveDamage @ 0x00701900`; `BuildingClass::GetWeapon @ 0x004526F0`; `AircraftClass::What_Action @ 0x00417CC0` touched; `BulletClass`/`WarheadTypeClass` detonation surfaces touched for routing context.
- Ghidra assembly/disassembly contexts: `0x0066C68F..0x0066C781`, `0x006FD150..0x006FD1EA`, `0x006F727A..0x006F72E1`, `0x006FE3E7..0x006FE455`, `0x00701BB6..0x00701BE7`.
- Ghidra xrefs: `InRange` callers `FootClass__Greatest_Threat_Scan`, `TechnoClass__CanFireAt`; `Fire_At` callers/overrides `AircraftClass__Fire_At`, `InfantryClass__Fire_At_Override`, unit fire path; `ReceiveDamage` callers `FootClass__ReceiveDamage`, `BuildingClass__ReceiveDamage`.
- INI checked: `ini/rulesmd.ini:843..869`, `ini/rulesmd.ini` stock `PenetratesBunker=yes` warhead lines, and `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md` stock `NATBNK` activation evidence.
- Prior docs checked: `BUNKER_SYSTEM_GHIDRA_REPORT.md`; `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md`; `GARRISON_SYSTEM_GHIDRA_REPORT.md`; `GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`; `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`; `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`.
- Rust scan checked: `src/rules/ruleset.rs`; `src/rules/warhead_type.rs`; `src/sim/game_entity.rs`; `src/sim/combat/mod.rs`; `src/sim/combat/combat_weapon.rs`; `src/sim/passenger.rs`.
