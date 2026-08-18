# Garrison Fire Index / Weapon Fallback / Credit -- Reswarm Verification

**Address(es):** `0x004526F0`, `0x00458DD0`, `0x006FDD50`, `0x00702D40`, `0x007091D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** ordinary `CanBeOccupied` garrison fire index selection/advance, occupant weapon fallback, empty fire gate, and kill-credit/veterancy implications against current Rust surfaces.  
**Non-Scope:** render timing/positioning except `OccupantAnim` event metadata, entry gates, sell/ejection, tank bunker lifecycle, global projectile runtime frequency.  
**Confidence:** High for static binary behavior and current Rust fallback status; Medium for future XP timing implications that require a runtime projectile trace.  
**Active in YR:** Yes. Standard YR INI defines `Occupier`, `OccupyWeapon`, `EliteOccupyWeapon`, `CanBeOccupied`, and `CanOccupyFire`; the verified paths are live virtual fire/weapon/destruction paths.

## Working Notes Required Before Investigation

Target question: Re-check `BuildingClass+0x69C` garrison fire index, occupant weapon fallback, post-shot advance, and kill-credit/veterancy implications against current Rust.  
Non-goals: Do not restudy render timing, entry gates, ejection, tank bunkers, or write Rust.  
Evidence needed to mark COMPLETE: decompile plus assembly for weapon selection, index advance, credit lookup, empty gate, INI liveness, and current Rust scan with handoff/test names.  
Stop conditions: all scoped material questions resolved or explicitly deferred; no Ghidra mutations; write only this report plus shared claims file.

## 1. Overview

Ordinary civilian garrison fire uses the building as the firing object, but selects its weapon from the infantry occupant at the current `BuildingClass+0x69C` index in the occupant vector. A successful `TechnoClass::Fire_At` launch advances `+0x69C` after launch; destruction credit later rereads live `+0x69C` rather than using a captured shooter id in the scoped static path.

Current dirty Rust already incorporates the previously missing elite fallback fix: elite garrison occupants with no `EliteOccupyWeapon` fall directly to primary, not to normal `OccupyWeapon`.

## 2. Class Layout / Key Offsets

| Offset | Owner | Verified purpose | Evidence | Active in YR |
|---|---|---|---|---|
| `+0x520` | `BuildingClass` | `BuildingTypeClass*` | `IsOccupied` assembly `0x00458DD0..0x00458DEC`; empty fire gate `0x00709212..0x00709220` | Yes |
| `+0x688` | `BuildingClass` | occupant item buffer pointer | `GetWeapon` `0x00452752..0x00452758`; `RecordKill` `0x00702FC5..0x00702FD1` | Yes |
| `+0x694` | `BuildingClass` | occupant count | `GetWeapon` `0x00452748..0x00452750`; `IsOccupied` calls vtable `+0x408` | Yes |
| `+0x69C` | `BuildingClass` | ordinary garrison current fire index | `GetWeapon` `0x00452742`; `Fire_At` `0x006FF065..0x006FF085`; `RecordKill` `0x00702FC5..0x00702FD1` | Yes |
| `+0x157B` | `BuildingTypeClass` | `CanBeOccupied` | `IsOccupied` `0x00458DD6`; fire gate `0x00709212..0x00709220` | Yes |
| `+0x157C` | `BuildingTypeClass` | `CanOccupyFire` | `IsOccupied` `0x00458DE0..0x00458DE8` | Yes |
| `+0x6C0` | `InfantryClass` | `InfantryTypeClass*` | `GetWeapon` `0x00452768`; `RecordKill` `0x00702FDB` | Yes |
| `+0x150` | `InfantryClass` | occupant veterancy object | `GetWeapon` `0x0045275B..0x00452761`; `RecordKill` `0x00702FEA..0x00702FF0` | Yes |
| `+0xE04` / `+0xE20` | `InfantryTypeClass` | `OccupyWeapon` / `EliteOccupyWeapon` | `GetWeapon` `0x00452770..0x00452792` | Yes |

## 3. Core Logic

### Weapon Selection

`BuildingClass::GetWeapon` first checks a separate `+0x5EC` / `+0x702` path that is not the ordinary occupant-vector path. For ordinary garrison occupants it calls `IsOccupied` via vtable `+0x400`, reads `+0x69C`, checks `count <= index`, and falls back to `TechnoClass::GetWeapon` if not occupied or out of range. Active in YR: Yes. Evidence: decompile `0x004526F0`; assembly `0x00452734..0x00452758`.

When in range, it loads `Items[index]` from `+0x688`, checks the occupant's veterancy at `InfantryClass+0x150`, reads the occupant type at `+0x6C0`, and selects `InfantryTypeClass+0xE04` for non-elite or `+0xE20` for elite. If the selected occupy weapon pointer is null, it calls the occupant infantry vtable `+0x3F8` with slot `0`, meaning primary fallback. Active in YR: Yes; conditional for null fallback on data/mods that omit occupy weapon fields. Evidence: decompile `0x004526F0`; assembly `0x0045275B..0x00452792`.

Elite fallback is direct `EliteOccupyWeapon -> Primary`; it is not `EliteOccupyWeapon -> OccupyWeapon -> Primary`. Active in YR: Conditional on an elite occupant type with null `EliteOccupyWeapon`. Evidence: `0x00452770..0x00452787`.

### Index Advance

`TechnoClassFireAtSpawnsBullet` advances `BuildingClass+0x69C` only after the bullet/effect virtual `+0x1F0` succeeds. It then checks `IsOccupied`, non-null `this`, and `WhatAmI == 6`, increments the index, calls occupant count via vtable `+0x408`, uses signed `IDIV`, and stores the remainder back to `+0x69C`. Active in YR: Yes. Evidence: decompile `0x006FDD50`; assembly `0x006FF031..0x006FF085`.

This means `+0x69C` is a live current/next index, not a captured shooter id attached to a projectile. Active in YR: Yes. Evidence: pre-fire selection at `0x00452742..0x00452758`; post-launch advance at `0x006FF065..0x006FF085`.

### Kill Credit / Veterancy Implication

`TechnoClass__RecordKill` (`0x00702D40`) handles occupied-building credit by checking attacker `IsOccupied()` and `WhatAmI == 6`, then rereads live `Items[Building+0x69C]`, loads occupant type `+0x6C0`, and calls the occupant type/veterancy credit path using the victim-derived value. It then calls the veterancy/credit helper with `LEA ECX,[occupant+0x150]`. Active in YR: Yes when an occupied building is the attacker object in destruction accounting. Evidence: decompile `0x00702D40`; assembly `0x00702F98..0x00702FF0`.

The scoped static path does not capture the occupant that selected the weapon. For delayed projectile kills, static evidence therefore points to crediting the occupant addressed by live `+0x69C` at record-kill time, not necessarily the pre-advance shooter. Active in YR: Yes for the accounting read; exact frequency of shooter/current-index divergence is deferred to runtime trace. Evidence: live read `0x00702FC5..0x00702FD1` plus post-launch advance `0x006FF065..0x006FF085`.

### Empty / Invalid Handling

`BuildingClass::IsOccupied` is exactly `CanBeOccupied && CanOccupyFire && GetOccupantCount() > 0`. Active in YR: Yes. Evidence: decompile `0x00458DD0`; assembly `0x00458DD0..0x00458DFE`.

The high-level fire gate at `0x007091D0` is different: for buildings (`WhatAmI == 6`) it blocks `CanBeOccupied && GetOccupantCount() == 0`, without reading `CanOccupyFire`. Active in YR: Yes; caller evidence includes `FUN_00709290` and `FootClass__Mission_AreaGuard`. Evidence: decompile `0x007091D0`; assembly `0x00709206..0x00709230`; caller list from Ghidra.

`GetWeapon` has an out-of-range fallback (`count <= index` -> building weapon). `RecordKill` does not repeat that local bounds guard before dereferencing `Items[index]`; it relies on occupied state and index maintenance. Active in YR: Yes. Evidence: `GetWeapon` `0x00452742..0x00452750`; `RecordKill` `0x00702F98..0x00702FF0`.

## 4. INI Keys

| Key | Source/default evidence | Effect in this slice | Active in YR |
|---|---|---|---|
| `CanBeOccupied` | many stock `rulesmd.ini` buildings, e.g. lines `13002`, `14108`, `14222` | `BuildingTypeClass+0x157B` gates `IsOccupied` and empty fire block | Yes |
| `CanOccupyFire` | many stock `rulesmd.ini` buildings, e.g. lines `13004`, `14111`, `14225` | `BuildingTypeClass+0x157C` gates `IsOccupied`; not read by empty fire block | Yes |
| `Occupier` | stock infantry, e.g. `E1` line `3720`, `INIT` line `4877` | entry/liveness only, not read in this slice | Yes |
| `OccupyWeapon` | stock `E1` line `3721`, `INIT` line `4875`, civilian `M1Carbine` line `4333` | non-elite occupy weapon pointer at `InfantryTypeClass+0xE04` | Yes |
| `EliteOccupyWeapon` | stock `E1` line `3722`, `INIT` line `4876`, civilian `M1Carbine` line `4334` | elite occupy weapon pointer at `InfantryTypeClass+0xE20` | Yes |
| `OccupantAnim` | weapon data; render/event metadata only here | Rust event metadata uses selected weapon's `OccupantAnim` | Conditional on weapon |

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Building weapon query | ordinary garrison uses `Items[+0x69C]` occupant weapon | `0x004526F0`, `0x00452742..0x00452792` | Yes |
| Successful fire | post-launch advance `(index + 1) % count` | `0x006FF031..0x006FF085` | Yes |
| Destruction/veterancy accounting | rereads live `Items[+0x69C]` and occupant veterancy | `0x00702F98..0x00702FF0` | Yes |
| Empty garrison gate | blocks `CanBeOccupied && count == 0` | `0x007091D0`, `0x00709206..0x00709230` | Yes |

## 6. Current Rust Implementation Status

- `src/sim/passenger.rs:45` stores `PassengerCargo.garrison_fire_index` and documents the verified `BuildingClass+0x69C` meaning. Current Rust delta: none observed for storage naming.
- `src/sim/combat/mod.rs:1263` and `src/sim/combat/mod.rs:1397` use `garrison_fire_index % count` to select the occupant for scan/snapshot. Current Rust delta: Rust normalizes with modulo before selection; binary `GetWeapon` falls back to building weapon if `count <= index`. In healthy maintained state this is equivalent; if stale out-of-range state can occur, this is DRIFT.
- `src/sim/combat/mod.rs:2051` emits `garrison_muzzle_index` from the captured pre-advance fire index and `src/sim/combat/mod.rs:2052` emits the selected weapon's `occupant_anim`. Current Rust delta: no issue in this scope.
- `src/sim/combat/mod.rs:2146` advances the index after a fired garrison shot as `(idx + 1) % count`. Current Rust delta: none observed for successful-fire advance ordering in the current combat tick structure.
- `src/sim/combat/combat_weapon.rs:280` now selects elite `EliteOccupyWeapon` else primary; test `elite_garrison_missing_elite_occupy_weapon_falls_back_to_primary` exists at `src/sim/combat/combat_weapon.rs:588`. Current Rust delta: older mismatch fixed.
- `src/sim/combat/combat_fire_gate.rs:111` blocks empty `CanBeOccupied` structures regardless of `CanOccupyFire`, matching `0x007091D0`. Current Rust delta: none observed for this gate.
- No implemented `RecordKill`/XP surface was found in the scan. `src/sim/combat/mod.rs:2161` records `last_attacker_id` as the building attacker id, and death handling has no visible occupant XP credit path. Current Rust delta: future feature missing/unchecked.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::GetWeapon` ordinary occupant-vector path | verified | decompile `0x004526F0`; assembly `0x00452742..0x00452792` | none |
| preliminary `+0x5EC/+0x702` path in `GetWeapon` | touched-not-exhausted | decompile `0x004526F0` | tank bunker/fire-port ownership out of scope |
| `BuildingClass::IsOccupied` | verified | decompile/assembly `0x00458DD0..0x00458DFE` | none |
| `TechnoClassFireAtSpawnsBullet` index advance | verified | decompile `0x006FDD50`; assembly `0x006FF031..0x006FF085` | none for static ordering |
| `TechnoClass__RecordKill` occupied-building branch | verified | decompile `0x00702D40`; assembly `0x00702F98..0x00702FF0` | runtime delayed-projectile divergence frequency |
| empty garrison fire gate `0x007091D0` | verified | decompile/assembly `0x00709206..0x00709230`; callers listed | none |
| occupant-vector shrink normalization | deferred | prior report says normal area damage does not remove occupants | exact direct-removal clamp/reset belongs to occupant lifecycle |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Which field is ordinary current fire index? -> `BuildingClass+0x69C`.` (evidence: `0x00452742`, `0x006FF065`, `0x00702FC5`)
- `[RESOLVED] OQ-2 -- Is weapon selection pre-advance? -> Yes, `GetWeapon` reads live index before `Fire_At` post-launch advance.` (evidence: `0x00452742..0x00452758`, `0x006FF065..0x006FF085`)
- `[RESOLVED] OQ-3 -- What is elite null fallback? -> primary via occupant vtable `+0x3F8`, not normal `OccupyWeapon`.` (evidence: `0x00452770..0x00452787`)
- `[RESOLVED] OQ-4 -- Which veterancy controls occupy weapon? -> occupant veterancy `InfantryClass+0x150`.` (evidence: `0x0045275B..0x00452770`)
- `[RESOLVED] OQ-5 -- Does `RecordKill` use live index? -> Yes, rereads `Items[+0x69C]`.` (evidence: `0x00702FC5..0x00702FD1`)
- `[RESOLVED] OQ-6 -- Is empty fire gate identical to `IsOccupied`? -> No; empty gate omits `CanOccupyFire`.` (evidence: `0x00458DD0..0x00458DFE`, `0x00709212..0x00709230`)
- `[RESOLVED] OQ-7 -- Is current dirty Rust still mismatching elite fallback? -> No, current code and test use direct primary fallback.` (evidence: `src/sim/combat/combat_weapon.rs:290`, `src/sim/combat/combat_weapon.rs:588`)
- `[DEFERRED] OQ-8 -- Can direct occupant-vector shrink create out-of-range `+0x69C` in active YR?` (category: requires-different-system-context; reason: removal lifecycle is outside slot; next-step-if-pursued: trace every occupant removal/clear caller for index writes)
- `[DEFERRED] OQ-9 -- How often does delayed projectile impact credit a different occupant than the weapon selector?` (category: needs-runtime-debugger; reason: static path proves live read, not scenario frequency; next-step-if-pursued: two-occupant slow projectile runtime trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x69C` selects `Items[index]`; successful launch advances `(index + 1) % count` after launch | `0x00452742..0x00452758`; `0x006FF065..0x006FF085` | mostly matched; modulo pre-normalization may mask stale out-of-range fallback | `src/sim/passenger.rs`, `src/sim/combat/mod.rs` | Preserve live current/next index semantics; advance only after a shot launches | two occupants fire three successful shots in order `0,1,0`; missed/suppressed shot does not advance | Do not use stale `+0x664` |
| Elite null occupy weapon falls directly to primary | `0x00452770..0x00452787` | fixed in current dirty Rust | `src/sim/combat/combat_weapon.rs` | Keep `EliteOccupyWeapon` else primary; normal `OccupyWeapon` only for non-elite path | elite occupant with primary + normal `OccupyWeapon` but no `EliteOccupyWeapon` fires primary | Do not reintroduce elite->normal occupy fallback |
| Occupied-building kill credit reads live `Items[+0x69C]` at record-kill time | `0x00702F98..0x00702FF0`; advance `0x006FF065..0x006FF085` | missing/unchecked; no XP surface found | future kill-credit/veterancy implementation near combat death accounting | XP credit should use live current occupant index unless a later runtime trace proves a different call path | two-occupant garrison fires once, index advances, target destruction credits the live current-index occupant | Do not credit the building or captured pre-advance shooter by assumption |
| Empty `CanBeOccupied` buildings cannot fire even if `CanOccupyFire` is false/irrelevant | `0x007091D0`, `0x00709212..0x00709230` | matched | `src/sim/combat/combat_fire_gate.rs` | Keep empty fire block keyed on `CanBeOccupied && count==0` | empty garrisonable building with own primary is blocked from firing | Do not replace with `IsOccupied` predicate |

Acceptance test-name proposals:
- `garrison_round_robin_advances_only_after_successful_fire`
- `elite_garrison_missing_elite_occupy_weapon_falls_back_to_primary`
- `garrison_record_kill_credits_live_current_index_occupant`
- `empty_canbeoccupied_building_fire_gate_ignores_canoccupyfire`
- `garrison_stale_out_of_range_index_falls_back_to_building_weapon`

### Negative Facts / Do Not Do

- Do not treat `BuildingClass+0x664` as ordinary garrison current-fire index. Evidence: `+0x69C` reads/writes at `0x00452742`, `0x006FF065`, `0x00702FC5`. Active in YR: Yes.
- Do not route elite null `EliteOccupyWeapon` through normal `OccupyWeapon`. Evidence: `0x00452770..0x00452787` calls primary fallback directly. Active in YR: Conditional.
- Do not implement the empty fire gate as full `IsOccupied`; `CanOccupyFire` is not read by `0x007091D0`. Evidence: `0x00709212..0x00709230` vs `0x00458DD0..0x00458DFE`. Active in YR: Yes.
- Do not credit garrison kills to the building object once XP exists. Evidence: occupied-building branch loads occupant type/veterancy at `0x00702FC5..0x00702FF0`. Active in YR: Yes.
- Do not assume `RecordKill` locally bounds-checks `+0x69C`; it dereferences `Items[index]` after `IsOccupied`/building checks. Evidence: `0x00702F98..0x00702FF0`. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_FIRE_INDEX_KILL_CREDIT_VETERANCY_GHIDRA_REPORT.md`: replace "Current Rust does fall back elite to `OccupyWeapon` before primary" and "One mismatch is visible: for elite occupants, Rust currently tries `EliteOccupyWeapon`, then `OccupyWeapon`, then primary" with "Current dirty Rust now matches the binary elite fallback: `select_garrison_weapon` uses `EliteOccupyWeapon` for elite occupants when present and otherwise falls directly to the occupant's primary weapon; test `elite_garrison_missing_elite_occupy_weapon_falls_back_to_primary` covers this."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`: replace "`BuildingClass+0x664` may be the current firing index" with "`BuildingClass+0x69C` is the verified ordinary garrison current fire index for the occupant DynamicVector path; `GetWeapon`, `Fire_At`, and `RecordKill` all read/write `+0x69C` for this behavior."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`: replace the `+0x664` garrison fire-index rows/claims with "`+0x69C` is the verified ordinary garrison fire index; `+0x664` must not be used as the occupant round-robin field."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/FIRE_AT_PIPELINE_GHIDRA_REPORT.md`: replace "clear garrison fire index (`this+0x664`)" and "round-robins through `this+0x664` garrison slots" with "`TechnoClass::Fire_At` advances ordinary garrison index `this+0x69C` after successful launch; any `+0x664` reset claim is not the ordinary occupant-vector round-robin contract."

## Sources

- Ghidra decompile/assembly: `BuildingClass__GetWeapon` `0x004526F0`, especially `0x00452742..0x00452792`.
- Ghidra decompile/assembly: `BuildingClass__IsOccupied` `0x00458DD0`, especially `0x00458DD0..0x00458DFE`.
- Ghidra decompile/assembly: `TechnoClassFireAtSpawnsBullet` `0x006FDD50`, especially `0x006FF031..0x006FF085`.
- Ghidra decompile/assembly: `TechnoClass__RecordKill` `0x00702D40`, especially `0x00702F98..0x00702FF0`.
- Ghidra decompile/assembly/callers: `FUN_007091D0`, especially `0x00709206..0x00709230`; callers `FUN_00709290`, `FootClass__Mission_AreaGuard`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/combat_weapon.rs`, `src/sim/combat/combat_fire_gate.rs`.
- Prior docs scanned: `docs/research/GARRISON_FIRE_INDEX_KILL_CREDIT_VETERANCY_GHIDRA_REPORT.md`, `docs/research/GARRISON_SYSTEM_MODEL_SYNTHESIS.md`, `docs/research/GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`.
