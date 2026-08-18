# Garrison Fire Index / Kill Credit / Veterancy -- Ghidra Research Report

**Address(es):** `0x004526F0`, `0x00458DD0`, `0x006FDD50`, `0x00702D40`, `0x007091D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** garrison shooter selection, round-robin index update, occupant weapon/veterancy use, kill-credit target, invalid-index handling visible in the scoped functions, and empty-garrison fire gating.  
**Non-Scope:** occupant death/removal, sell/destruction ejection, bunker `+0x2E4`, target choice beyond the already-settled garrison scan range, and fire-port render cadence except the fire-index-to-event handoff.  
**Confidence:** High for the scoped binary behavior; Medium for Rust deltas where the current Rust surface has no kill-credit/XP system to compare.  
**Active in YR:** Yes. Standard YR data sets `Occupier`, `OccupyWeapon`, `EliteOccupyWeapon`, `CanBeOccupied`, and `CanOccupyFire` in `ini/rulesmd.ini`; the scoped functions are live virtual calls reached by normal building fire, weapon selection, and destruction accounting.

## Working Notes Required Before Investigation

Target question: Confirm garrison firing shooter selection and consequences: current-fire-index semantics, occupant weapon/veterancy source, kill-credit/experience target, invalid occupant handling, and empty-garrison fire gate.

Non-goals: Do not restudy fire-port render positioning, entry gates, occupant death/removal, ejection, or bunker lifecycle except where directly needed for this slice.

Evidence needed to mark COMPLETE: decompile plus assembly context for `GetWeapon`, `IsOccupied`, `Fire_At`, `RegisterDestruction`, and `FUN_007091D0`; INI/Rust scan; stale-doc wording; implementation handoff with test-name proposals.

Stop conditions: all scoped questions resolved or explicitly deferred; no Ghidra mutations; write only this report plus shared claims file.

## 1. Overview

Garrison fire is building-owned fire whose weapon identity comes from the current occupant. The verified round-robin field in this path is `BuildingClass+0x69C`, which indexes the `DynamicVectorClass<InfantryClass*>` occupant item buffer at `+0x688`; the older `+0x664` "current firing occupant" claim is stale for this behavior.

The important consequence is ordering: `GetWeapon` uses `+0x69C` to choose the occupant before firing, `Fire_At` advances `+0x69C` after successful projectile/effect launch, and `RegisterDestruction` later credits the occupant currently addressed by live `+0x69C`. The binary does not show a captured "firing occupant id" in this scoped accounting path.

## 2. Class Layout / Key Offsets

| Offset | Owner | Purpose | Evidence | Active in YR |
|---|---|---|---|---|
| `+0x520` | `BuildingClass` | `BuildingTypeClass*` | `IsOccupied` assembly `0x00458DD0..0x00458DEC`; `FUN_007091D0` `0x00709212..0x0070921E` | Yes |
| `+0x688` | `BuildingClass` | occupant item buffer pointer | `GetWeapon` assembly `0x00452752..0x00452758`; `RegisterDestruction` `0x00702FC5..0x00702FD1` | Yes |
| `+0x694` | `BuildingClass` | occupant count | `GetOccupantCount` decompile `0x004581F0`; `GetWeapon` assembly `0x00452748..0x00452750` | Yes |
| `+0x69C` | `BuildingClass` | current garrison fire index | `GetWeapon` assembly `0x00452742..0x00452758`; `Fire_At` assembly `0x006FF065..0x006FF085`; `RegisterDestruction` `0x00702FC5..0x00702FD1` | Yes |
| `+0x157B` | `BuildingTypeClass` | `CanBeOccupied` | `IsOccupied` `0x00458DD6`; fire gate `0x00709212..0x00709220` | Yes |
| `+0x157C` | `BuildingTypeClass` | `CanOccupyFire` | `IsOccupied` `0x00458DE0..0x00458DEC` | Yes |
| `+0x6C0` | `InfantryClass` | `InfantryTypeClass*` | `GetWeapon` `0x00452768`; `RegisterDestruction` `0x00702FDB` | Yes |
| `+0x150` | `InfantryClass` | occupant veterancy object | `GetWeapon` `0x0045275B..0x00452761`; `RegisterDestruction` `0x00702FEA..0x00702FF0` | Yes |
| `+0xE04` / `+0xE20` | `InfantryTypeClass` | `OccupyWeapon` / `EliteOccupyWeapon` | `GetWeapon` decompile `0x004526F0`; assembly `0x00452770..0x00452792` | Yes |

## 3. Core Logic

### Shooter / Weapon Selection

`BuildingClass::GetWeapon` first checks a separate fire-port/bunker-style array at `Building+0x5EC` with byte count `+0x702`; that path is not the ordinary `CanBeOccupied` occupant-vector path. For ordinary garrison occupants, it calls vtable `+0x400` (`IsOccupied`), then checks `occupant_count <= current_index`. If the building is not occupied or the index is out of range, it falls back to `TechnoClass::GetWeapon`. Evidence: decompile `0x004526F0`; assembly `0x00452734..0x00452750`. Active in YR: Yes, because `CanBeOccupied`/`CanOccupyFire` YR buildings use this vtable weapon query.

When in range, `GetWeapon` loads `InfantryClass* occupant = Items[current_index]`, checks `occupant+0x150` with `VeterancyClass::IsElite`, loads `occupant+0x6C0`, and returns `InfantryTypeClass+0xE04` for non-elite or `+0xE20` for elite. If the selected occupy pointer is null, it calls the occupant infantry's own `GetWeapon(0)`. Evidence: decompile `0x004526F0`; assembly `0x00452752..0x00452787`. Active in YR: Yes, `rulesmd.ini` defines `OccupyWeapon`/`EliteOccupyWeapon` for stock garrison infantry.

Tiny detail: elite fallback is `EliteOccupyWeapon` directly to occupant primary when `+0xE20 == 0`; the binary does not fall back elite `EliteOccupyWeapon -> OccupyWeapon -> Primary` in this function. Current Rust does fall back elite to `OccupyWeapon` before primary. Evidence: decompile `0x004526F0`; assembly `0x00452770..0x00452787`. Active in YR: Conditional on an elite garrison occupant whose `EliteOccupyWeapon` pointer is null.

### Round-Robin Index Update

`TechnoClass::Fire_At` advances the index only after the projectile/effect launch succeeds (`vtable+0x1F0` returned nonzero), then only if `IsOccupied()` is true, `this` is non-null, and `WhatAmI == 6` (building). It increments `Building+0x69C`, calls `GetOccupantCount` (`vtable+0x408`), then signed-divides by count and stores the remainder back to `+0x69C`. Evidence: decompile `0x006FDD50`; assembly `0x006FF031..0x006FF085`. Active in YR: Yes for garrisoned buildings that actually fire.

The modulo is not a pre-fire selection step. Selection has already happened through `GetWeapon`; the update occurs after successful launch, so the field means "next/current live garrison index" after the shot, not necessarily a stable "shooter id" attached to a projectile. Evidence: call order in decompile `0x006FDD50` plus assembly `0x006FF031..0x006FF085`. Active in YR: Yes.

### Kill Credit / Experience

`TechnoClass::RegisterDestruction` (`0x00702D40`) awards destruction credit through several owner-remap cases. In the garrison building case it checks attacker `IsOccupied()` and `WhatAmI == 6`, then reads the attacker's live `+0x69C`, loads `Items[current_index]` through `+0x688`, loads the occupant's type at `+0x6C0`, and calls that type/veterancy credit path with the victim owner/type-derived value. Evidence: decompile `0x00702D40`; assembly `0x00702F98..0x00702FF0`. Active in YR: Yes when the attacker object passed to destruction registration is an occupied firing building.

There is no local capture of the occupant that selected the weapon inside this scoped function. Because `Fire_At` advances `+0x69C` after launch, `RegisterDestruction` credits whichever occupant is addressed by live `+0x69C` when destruction registration executes. Evidence: absence of a captured occupant in `0x00702D40` plus live field reads `0x00702FC5..0x00702FD1`; post-launch advance `0x006FF065..0x006FF085`. Active in YR: Yes; exact projectile travel timing effects are deferred to projectile/damage runtime tracing.

### Empty / Invalid Handling

`IsOccupied` is exactly `CanBeOccupied && CanOccupyFire && GetOccupantCount() > 0`. Evidence: decompile `0x00458DD0`; assembly `0x00458DD0..0x00458DFE`. Active in YR: Yes.

The separate high-level fire gate `FUN_007091D0` is not the same predicate: it blocks a building of type `6` with `CanBeOccupied` and `GetOccupantCount()==0`, without reading `CanOccupyFire`. Evidence: decompile `0x007091D0`; assembly `0x00709206..0x00709230`. Active in YR: Yes. This means "empty garrisonable building cannot fire" is `CanBeOccupied + count==0`, while "occupied garrison fire modifiers/selection" use `CanBeOccupied + CanOccupyFire + count>0`.

`GetWeapon` handles out-of-range current index by falling back to the building's own weapon (`occupant_count <= current_index`). It does not visibly null-check `Items[current_index]` after the bounds check. Evidence: `0x00452742..0x0045275B`. Active in YR: Yes, but null occupant entries should not exist in a healthy DynamicVector occupant list.

`RegisterDestruction` does not repeat the `count <= current_index` bounds fallback before dereferencing `Items[current_index]`; it relies on `IsOccupied()` and index maintenance elsewhere. Evidence: `0x00702F98..0x00702FF0`. Active in YR: Yes for occupied building kill-credit accounting; occupant removal normalization is outside this slot.

## 4. INI Keys

| Key | Default / source | Binary effect in this slice | Active in YR |
|---|---|---|---|
| `CanBeOccupied` | object default false; many stock YR civilian buildings set yes in `rulesmd.ini` | `IsOccupied` and empty-fire gate read `BuildingTypeClass+0x157B` | Yes |
| `CanOccupyFire` | object default false; stock YR garrisonable combat buildings set yes in `rulesmd.ini` | `IsOccupied` requires `+0x157C`; empty-fire gate does not read it | Yes |
| `MaxNumberOccupants` | object default 0; stock YR values vary | not directly read in this slice after entry; count comes from `+0x694` | Yes, via entry/capacity out of scope |
| `Occupier` | infantry default false; stock infantry set yes/no | not read in this slice; entry gate out of scope | Yes |
| `OccupyWeapon` | infantry default null; stock `E1`/`GGI`/`INIT` define values | non-elite `GetWeapon` returns `InfantryTypeClass+0xE04`, null -> primary | Yes |
| `EliteOccupyWeapon` | infantry default null; stock garrison infantry define values | elite `GetWeapon` returns `InfantryTypeClass+0xE20`, null -> primary | Yes |
| `OccupantAnim` | weapon optional | Rust event/render surface uses it; binary visual path not reopened except as fire event metadata | Yes for weapons that define it |

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Building weapon query | current occupant weapon selected through `GetWeapon` | `0x004526F0`, `0x00452742..0x00452787` | Yes |
| Occupied predicate | `CanBeOccupied && CanOccupyFire && count>0` | `0x00458DD0..0x00458DFE` | Yes |
| Fire launch | successful launch advances `+0x69C = (+0x69C + 1) % count` | `0x006FF031..0x006FF085` | Yes |
| Fire ability gate | empty `CanBeOccupied` building cannot fire even if it has own weapons | `0x007091D0`, `0x00709206..0x00709230` | Yes |
| Destruction accounting | occupied building credit goes through live `Items[+0x69C]` occupant type/veterancy | `0x00702F98..0x00702FF0` | Yes |

## 6. Current Rust Implementation Status

Current Rust already has a garrison fire index in `src/sim/passenger.rs:41`, uses it to select occupants for scan/fire snapshots in `src/sim/combat/mod.rs:1228` and `src/sim/combat/mod.rs:1362`, emits garrison muzzle index at `src/sim/combat/mod.rs:1998`, and advances the index after fire at `src/sim/combat/mod.rs:2094`.

Current Rust selects garrison weapons in `src/sim/combat/combat_weapon.rs:280`. One mismatch is visible: for elite occupants, Rust currently tries `EliteOccupyWeapon`, then `OccupyWeapon`, then primary; binary `GetWeapon` tries `EliteOccupyWeapon`, then primary.

Current Rust has an empty-garrison fire gate matching `CanBeOccupied && empty cargo` in `src/sim/combat/combat_fire_gate.rs:111`, which matches `FUN_007091D0` better than the stricter `IsOccupied` predicate. No implemented RegisterDestruction-style kill-credit/XP surface was found in this scan; current fire events carry the building `attacker_id` and building veterancy at `src/sim/combat/mod.rs:1980..1985`, not a credited occupant id.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::GetWeapon` occupant-vector path | verified | decompile `0x004526F0`; assembly `0x00452742..0x00452787` | none for this slice |
| fire-port/bunker preliminary path in `GetWeapon` | touched-not-exhausted | decompile `0x004526F0`; assembly `0x004526FA..0x00452734` | bunker/fire-port ownership belongs to slot 5 |
| `BuildingClass::IsOccupied` | verified | decompile `0x00458DD0`; assembly `0x00458DD0..0x00458DFE` | none |
| `TechnoClass::Fire_At` round-robin advance | verified | decompile `0x006FDD50`; assembly `0x006FF031..0x006FF085` | projectile runtime timing not traced |
| `TechnoClass::RegisterDestruction` garrison credit branch | verified | decompile `0x00702D40`; assembly `0x00702F98..0x00702FF0` | exact delayed projectile impact scenarios require runtime trace |
| `FUN_007091D0` empty-garrison fire block | verified | decompile `0x007091D0`; assembly `0x00709206..0x00709230` | none |
| occupant removal normalizes `+0x69C` | deferred | slot-1 scope | needed only if occupant dies while index points past new count |
| fire-port render positioning | deferred | parent non-goal | not needed for shooter/credit contract |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Which field is the ordinary garrison current fire index? -> BuildingClass+0x69C, not +0x664, in GetWeapon/Fire_At/RegisterDestruction.` (evidence: `0x00452742..0x00452758`, `0x006FF065..0x006FF085`, `0x00702FC5..0x00702FD1`)
- `[RESOLVED] OQ-2 -- Is shooter selection pre-fire or post-fire? -> `GetWeapon` selects using live `+0x69C` before Fire_At advances it after successful launch.` (evidence: `0x004526F0`, `0x006FF031..0x006FF085`)
- `[RESOLVED] OQ-3 -- Which veterancy selects normal vs elite garrison weapon? -> occupant infantry's `+0x150`, not building veterancy.` (evidence: `0x0045275B..0x00452770`)
- `[RESOLVED] OQ-4 -- Does elite missing `EliteOccupyWeapon` fall back to `OccupyWeapon`? -> No in `GetWeapon`; it falls back to occupant primary.` (evidence: `0x00452770..0x00452787`)
- `[RESOLVED] OQ-5 -- Where does kill credit go for occupied building attacker? -> through live `Items[Building+0x69C]` occupant type/veterancy path.` (evidence: `0x00702F98..0x00702FF0`)
- `[RESOLVED] OQ-6 -- Is the kill-credit occupant captured at fire time? -> No captured occupant is visible in scoped `RegisterDestruction`; it rereads live building index.` (evidence: `0x00702FC5..0x00702FD1`)
- `[RESOLVED] OQ-7 -- Is empty fire gate exactly `CanBeOccupied && CanOccupyFire && count>0`? -> No. That is `IsOccupied`; separate fire gate blocks `CanBeOccupied && count==0` and does not read `CanOccupyFire`.` (evidence: `0x00458DD0..0x00458DFE`, `0x00709206..0x00709230`)
- `[RESOLVED] OQ-8 -- What happens when current index is out of bounds during weapon selection? -> `GetWeapon` falls back to building weapon if `count <= index`.` (evidence: `0x00452742..0x00452750`)
- `[RESOLVED] OQ-9 -- Does `RegisterDestruction` locally bounds-check current index? -> No local `count <= index` guard appears in the garrison branch.` (evidence: `0x00702F98..0x00702FF0`)
- `[RESOLVED] OQ-10 -- Does `Fire_At` avoid divide-by-zero on index modulo? -> Yes through prior `IsOccupied()` true and `GetOccupantCount()` call; `IsOccupied` requires count > 0.` (evidence: `0x00458DD0..0x00458DFE`, `0x006FF031..0x006FF085`)
- `[RESOLVED] OQ-11 -- Is this TS legacy only? -> No; YR INI uses `CanBeOccupied`, `CanOccupyFire`, `OccupyWeapon`, and `EliteOccupyWeapon`, and the functions are live building/fire/destruction virtual paths.` (evidence: `ini/rulesmd.ini:3720..3722`, `ini/rulesmd.ini:13002..13004`, `0x004526F0`, `0x006FDD50`)
- `[DEFERRED] OQ-12 -- Does occupant removal clamp or reset `+0x69C` when the vector shrinks?` (category: out-of-scope; reason: slot 1 owns occupant death/removal; next-step-if-pursued: trace PenetratesBunker removal and DynamicVector erase side effects)
- `[DEFERRED] OQ-13 -- For delayed projectile kills, how often does live `+0x69C` differ from the occupant that selected the weapon?` (category: needs-runtime-debugger; reason: static binary proves the accounting read, not in-match projectile timing frequency; next-step-if-pursued: runtime trace two-occupant garrison with slow projectile and logged `+0x69C`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary garrison current fire index is `+0x69C`; selected occupant is `Items[index]`; update is `(index + 1) % count` after successful launch | `0x00452742..0x00452758`; `0x006FF065..0x006FF085` | mostly implemented | `src/sim/passenger.rs:41`, `src/sim/combat/mod.rs:1228`, `src/sim/combat/mod.rs:2094` | Keep index as "current/next live index", advance only when a garrison shot actually launches | two occupants fire three successful shots and selected occupant order is 0,1,0 while final index is 1 | Do not use stale `+0x664` name/offset as the contract |
| Elite occupant with null `EliteOccupyWeapon` falls back to primary, not to `OccupyWeapon` | `0x00452770..0x00452787` | mismatch observed | `src/sim/combat/combat_weapon.rs:280` | For elite garrison selection, use `EliteOccupyWeapon` if present, otherwise occupant primary; do not insert `OccupyWeapon` in between | elite occupant with only `OccupyWeapon` and primary fires primary from garrison | Current Rust likely overuses normal occupy weapon for elite missing elite override |
| Destruction credit for occupied building attacker reads live `Items[+0x69C]`, not a captured shooter id | `0x00702F98..0x00702FF0`; Fire_At advance `0x006FF065..0x006FF085` | missing/unchecked, no XP surface found | future kill-credit/experience system; likely combat damage/death accounting plus `SimFireEvent` metadata | When adding XP/kill credit, route occupied-building kills through the live current occupant index at destruction registration time unless runtime tracing proves an earlier capture path | two-occupant garrison fires once, index advances, target destruction credits occupant at current live index | Do not credit the building or blindly credit the occupant that selected the weapon unless a later runtime trace contradicts this static accounting path |

### Acceptance Test Name Proposals

- `garrison_round_robin_advances_only_after_successful_fire`
- `elite_garrison_missing_elite_occupy_weapon_falls_back_to_primary`
- `garrison_register_destruction_credits_live_current_index_occupant`

### Negative Facts / Do Not Do

- Do not treat `BuildingClass+0x664` as the verified ordinary garrison current-fire index; this slice verifies `+0x69C` in all three load-bearing paths. Evidence: `0x00452742`, `0x006FF065`, `0x00702FC5`.
- Do not implement the empty fire gate as `CanBeOccupied && CanOccupyFire && count == 0`; `FUN_007091D0` only checks `CanBeOccupied && count == 0`, while `CanOccupyFire` belongs to `IsOccupied`. Evidence: `0x00709212..0x00709230` vs `0x00458DD0..0x00458DFE`.
- Do not make elite missing `EliteOccupyWeapon` fall back to normal `OccupyWeapon` in the binary `GetWeapon` path. Evidence: `0x00452770..0x00452787`.
- Do not credit garrison kills to the building object when implementing destruction accounting. Evidence: occupied-building branch loads occupant via `+0x688/+0x69C/+0x6C0` at `0x00702FC5..0x00702FEA`.
- Do not assume `RegisterDestruction` has its own index bounds fallback; it dereferences live `Items[index]` after `IsOccupied` and building-type checks. Evidence: `0x00702F98..0x00702FF0`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`: replace "Which occupant fires: The mechanism for selecting which occupant fires from the garrison (rotating through them) was not fully traced. `BuildingClass+0x664` may be the current firing index." with "`BuildingClass+0x69C` is the verified ordinary garrison current fire index for the occupant DynamicVector path. `BuildingClass::GetWeapon` reads `Items[+0x69C]` after an `IsOccupied` and bounds check, `TechnoClass::Fire_At` advances `+0x69C = (+0x69C + 1) % GetOccupantCount()` after successful launch, and `TechnoClass::RegisterDestruction` rereads live `Items[+0x69C]` for occupied-building kill credit."

## 10. Sources

- Ghidra decompile/assembly: `BuildingClass::GetWeapon` `0x004526F0`, assembly `0x00452742..0x00452787`.
- Ghidra decompile/assembly: `BuildingClass::IsOccupied` `0x00458DD0`, assembly `0x00458DD0..0x00458DFE`.
- Ghidra decompile: `BuildingClass::GetOccupantCount` `0x004581F0`.
- Ghidra decompile/assembly: `TechnoClass::Fire_At` `0x006FDD50`, assembly `0x006FF031..0x006FF085`.
- Ghidra decompile/assembly: `TechnoClass::RegisterDestruction` `0x00702D40`, assembly `0x00702F98..0x00702FF0`.
- Ghidra decompile/assembly: `FUN_007091D0`, assembly `0x00709206..0x00709230`.
- Prior docs scanned: `GARRISON_SYSTEM_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`.
- INI scanned: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/combat_weapon.rs`, `src/sim/combat/combat_fire_gate.rs`, `src/sim/world/mod.rs`.
