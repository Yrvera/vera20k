# TIBTRE Building Exception Bytes 0xC9A / 0x1701 - Ghidra Research Report

**Address(es):** `0x004838E0` (`CellClass::CanPlaceTiberium`), `0x00712170` (`TechnoTypeClass::ReadINI`), `0x0045FE50` (`BuildingTypeClass::ReadINI_Water`), `0x00710AF0` (`TechnoTypeClass::Constructor`), `0x0045DD90` (`BuildingTypeClass::Constructor`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Identify the two building-type bytes used by the TIBTRE `CanPlaceTiberium` live-building gate: key names, readers, defaults, stock YR users, and Rust terrain-spawn validation implications.
**Non-Scope:** Full `CanPlaceTiberium` gate proof beyond the live-building branch, full map census of stock scenarios containing both TIBTRE and invisible lamp buildings, building draw/cloak behavior outside its role as evidence for field identity, save/load serialization.
**Confidence:** High
**Active in YR:** Yes, conditional on a live building object occupying the candidate cell during active gameplay.

## 0. Investigation Setup

Target question: What are `BuildingTypeClass+0xC9A` and `BuildingTypeClass+0x1701` in the `CanPlaceTiberium` live-building gate, where are they read and defaulted, which stock YR building types set them, and can TIBTRE placement pass on a live building cell?

Non-goals: Do not re-investigate TIBTRE timing, source tiberium type, non-building `CanPlaceTiberium` gates, terrain object lifecycle, `AllowTiberium`, or cell flag `0x500`. Do not modify Rust, INI, in-repo docs, or Ghidra state.

Evidence needed to mark COMPLETE:

- Direct `CanPlaceTiberium` decompile/assembly showing the two byte reads and pass/reject polarity.
- Direct parser evidence mapping `+0xC9A` to an INI key and default.
- Direct parser evidence mapping `+0x1701` to an INI key and default.
- Stock INI census for `Invisible=` and `InvisibleInGame=` among building types.
- Current Rust scan for relevant parsed fields and `terrain_spawn` validation gap.

Stop conditions:

- Stop after resolving the two offsets and Rust-facing handoff.
- Defer full map placement census if it would require scanning every scenario for TIBTRE/lamp adjacency.
- Defer visual/cloak semantics except as field-identity evidence.

## 1. Overview

The live-building gate in `CellClass::CanPlaceTiberium` does not reject every building. A live `BuildingClass` with health greater than zero rejects ore placement only when both `BuildingTypeClass+0xC9A` and `BuildingTypeClass+0x1701` are zero.

`+0xC9A` is inherited `TechnoTypeClass Invisible=`. `+0x1701` is `BuildingTypeClass InvisibleInGame=`. Standard YR has no stock `Invisible=yes` technotypes in `rules.ini` or `rulesmd.ini`, but it has sixteen stock building light objects with `InvisibleInGame=yes`; the building parser also forces `Invisible=1` and `RadarVisible=0` when `InvisibleInGame=yes` is read.

## 2. Class Layout / Key Offsets

| Class | Offset | Type | Key / meaning | Default | Evidence | Active in YR |
|---|---:|---|---|---:|---|---|
| `TechnoTypeClass` inherited by `BuildingTypeClass` | `+0xC9A` | byte bool | `Invisible=` | `0` | Constructor `0x007113D8`; parser `0x00714A97..0x00714AAB`; string `0x00843944` | Yes, code-live; stock data dormant except `InvisibleInGame` forces it for buildings |
| `TechnoTypeClass` inherited by `BuildingTypeClass` | `+0xC9B` | byte bool | `RadarVisible=` | `0` | Parser context `0x00714AB1..`; `InvisibleInGame` force-clear at `0x00460E10` | Yes |
| `BuildingTypeClass` | `+0x1701` | byte bool | `InvisibleInGame=` | `0` | Constructor `0x0045E206`; parser `0x00460DEB..0x00460E01`; string `0x0081A8CC` | Yes |
| `BuildingClass` | `+0x520` | pointer | `BuildingTypeClass*` used by the gate | n/a | `CanPlaceTiberium` `0x00483942` loads type pointer after RTTI/health checks | Yes |
| `BuildingClass` / object base | `+0x6C` | int | current health checked as `> 0` | n/a | `CanPlaceTiberium` `0x0048393B..0x00483940` | Yes |

## 3. Core Logic

`CellClass::CanPlaceTiberium @ 0x004838E0` first confirms the candidate cell is in the playfield and not blocked by cell flags `0x500`. If `g_GameActive != 0`, it scans the cell object list at `CellClass+0xE4`.

Live-building branch:

1. Iterate objects in the cell object list.
2. Call each object's RTTI/`WhatAmI`; only RTTI `6` (`BuildingClass`) enters this branch.
3. If the building pointer is non-null and `Building+0x6C > 0`, load `Building+0x520` as `BuildingTypeClass*`.
4. Read `Type+0xC9A`. If nonzero, the building branch passes.
5. Otherwise read `Type+0x1701`. If nonzero, the building branch passes.
6. If both are zero, return false from `CanPlaceTiberium`.
7. After any building object is handled, the loop breaks; it does not continue scanning for a second building object. Normal cell occupancy should not contain multiple live building objects on one cell.

Important polarity: these bytes are exceptions to the live-building rejection. They do not by themselves approve the cell. The later gates still run: terrain object, land Buildable, no overlay, flat slope, and tile `AllowTiberium`.

Binary evidence:

- `0x00483937..0x00483942`: building pointer and health gate, then `Building+0x520` type pointer load.
- `0x00483948..0x00483950`: read/test `Type+0xC9A`, nonzero jumps past rejection.
- `0x00483952..0x0048395A`: read/test `Type+0x1701`, zero falls through to reject at `0x004839E9`.

## 4. INI Keys

| Key | Scope | Reader | Target field | Default | Stock YR users | Effect on TIBTRE building gate |
|---|---|---|---|---:|---|---|
| `Invisible=` | `TechnoTypeClass` for all technotypes, including buildings | `TechnoTypeClass::ReadINI @ 0x00712170`; string xref at `0x00714A9E`; write at `0x00714AAB` | `+0xC9A` | `0` | None found by exact-key grep in `ini/rules.ini` or `ini/rulesmd.ini` | If `yes`, live building does not reject TIBTRE placement |
| `InvisibleInGame=` | `BuildingTypeClass` | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`; string xref at `0x00460DF2`; write at `0x00460E01` | `+0x1701` | `0` | Sixteen stock light/lamp building types set `yes` in `rulesmd.ini` | If `yes`, live building does not reject TIBTRE placement |
| `RadarVisible=` | `TechnoTypeClass` | `TechnoTypeClass::ReadINI`; string `0x00843934` near `Invisible=` | `+0xC9B` | `0` | Several stock civilian UC buildings set `yes`, not part of this exception | `InvisibleInGame=yes` force-clears it; not read by `CanPlaceTiberium` |

Parser details:

- `TechnoTypeClass::Constructor @ 0x00710AF0` zeroes `+0xC9A` with `MOV byte ptr [ESI+0xC9A], BL` at `0x007113D8`; `BL` is zeroed at constructor entry `0x00710B00`.
- `BuildingTypeClass::Constructor @ 0x0045DD90` zeroes `+0x1701` with `MOV byte ptr [ESI+0x1701], BL` at `0x0045E206`.
- `TechnoTypeClass::ReadINI @ 0x00714A97..0x00714AAB` reads prior `+0xC9A`, pushes string `Invisible`, calls `CCINIClass::ReadBool`, and stores the result back to `+0xC9A`.
- `BuildingTypeClass_ReadINI_Water @ 0x00460DEB..0x00460E01` reads prior `+0x1701`, pushes string `InvisibleInGame`, calls `CCINIClass::ReadBool`, and stores the result to `+0x1701`.
- If the new `InvisibleInGame` value is nonzero, `0x00460E09` writes `+0xC9A = 1` and `0x00460E10` writes `+0xC9B = 0`. This means stock `InvisibleInGame=yes` buildings satisfy both exception bytes even though they do not explicitly write `Invisible=yes`.

Stock YR census from `ini/rulesmd.ini`:

| Building type | BuildingTypes line | `InvisibleInGame=yes` line | Notable stock values |
|---|---:|---:|---|
| `NEGLAMP` | `1236` | `17266` | `Strength=1000`, `Insignificant=yes`, `LightIntensity=-0.15` |
| `INGALITE` | `1233` | `17287` | `Strength=6000`, `Selectable=no`, `Insignificant=yes`, `LightIntensity=0.2` |
| `INYELWLAMP` | `1234` | `17437` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `INPURPLAMP` | `1235` | `17488` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `INORANLAMP` | `1218` | `17512` | `Strength=600`, `Selectable=no`, `Insignificant=yes` |
| `INGRNLMP` | `1219` | `17536` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `INREDLMP` | `1220` | `17561` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `INBLULMP` | `1221` | `17583` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `TEMMORLAMP` | `1446` | `17607` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `TEMDAYLAMP` | `1447` | `17631` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `TEMDUSLAMP` | `1448` | `17655` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `TEMNITLAMP` | `1449` | `17679` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `SNOMORLAMP` | `1450` | `17703` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `SNODAYLAMP` | `1451` | `17727` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `SNODUSLAMP` | `1452` | `17751` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |
| `SNONITLAMP` | `1453` | `17775` | `Strength=6000`, `Selectable=no`, `Insignificant=yes` |

Note: `rules.ini` contains the same sixteen base RA2 entries; `rulesmd.ini` repeats them and is the YR-priority source. Exact-key grep found no `Invisible=` assignments in either file.

## 5. Integration Points

TIBTRE path:

- `TerrainClass::AI @ 0x0071C730` calls `CellClass::SpreadTiberium(force=1)` at the animation midpoint.
- `CellClass::SpreadTiberium @ 0x00483780` checks adjacent candidates with `CellClass::CanPlaceTiberium @ 0x004838E0` before calling `PlaceTiberium(type, 3)`.
- Therefore the live-building exception bytes are active for TIBTRE target selection in standard YR gameplay.

The branch is conditional:

- If `g_GameActive == 0`, the object-list building scan is skipped.
- In normal gameplay `g_GameActive != 0`, so any live building candidate reaches the exception check.
- Dead buildings with `health <= 0` do not reject through this branch.
- Invisible/invisible-in-game buildings only bypass the building branch. Later gates may still reject the cell.

## 6. Current Rust Implementation Status

Current Rust parses `InvisibleInGame` but not plain `Invisible`:

- `src/rules/object_type.rs:646..649` defines `invisible_in_game`.
- `src/rules/object_type.rs:1083` parses `InvisibleInGame`.
- No `Invisible`/`TechnoType Invisible=` field was found in `src/rules` or `src/sim`.

Current Rust terrain spawning still has no explicit live-building gate:

- `src/sim/terrain_spawn.rs` implements `can_accept_tiberium`.
- As of the 2026-05-24 TIBTRE implementation pass, the predicate uses resource/overlay rejection plus resolved-terrain gates for flat slope, base-buildable terrain, bridge flags, and `AllowTiberium`.
- There is no entity/object-list or building-type exception check.
- Any path-grid or terrain occupancy approximation is not equivalent to `CanPlaceTiberium` because GameMD's building gate is type-aware and lets invisible/invisible-in-game live buildings pass.

Rust feasibility:

- For stock parity, an ordinary live-building reject is correct for most buildings, but not for the sixteen stock invisible light/lamp buildings if a valid TIBTRE candidate cell contains one.
- Since Rust already parses `InvisibleInGame`, the stock exception can be represented without adding `Invisible=` immediately.
- For normal modded parity, plain `Invisible=yes` on a building should also be represented, because GameMD accepts it even when `InvisibleInGame=no`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CanPlaceTiberium` live-building gate | verified | `0x00483937..0x0048395A` | none |
| `+0xC9A` identity as `Invisible=` | verified | string `0x00843944`; parser `0x00714A97..0x00714AAB`; constructor `0x007113D8` | none |
| `+0x1701` identity as `InvisibleInGame=` | verified | string `0x0081A8CC`; parser `0x00460DEB..0x00460E01`; constructor `0x0045E206` | none |
| `InvisibleInGame` side effect on `Invisible`/`RadarVisible` | verified | `0x00460E07..0x00460E10` | none |
| Stock exact-key INI census | verified | `rg` / PowerShell scan of `ini/rules.ini`, `ini/rulesmd.ini` | full map adjacency census deferred |
| Rust field parsing | verified | `src/rules/object_type.rs:646..649`, `:1083`; `rg Invisible` | none |
| Rust terrain-spawn building validation | verified-source-scan | `src/sim/terrain_spawn.rs` | exact future occupancy API design out-of-scope |
| Full stock scenario/map placement census | deferred | not needed for key identity | scan maps for TIBTRE adjacent to invisible lamps if a player-visible stock-map case is needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `CanPlaceTiberium` on the live TIBTRE path? -> yes, from `TerrainClass::AI -> SpreadTiberium(force=1) -> CanPlaceTiberium`.` (evidence: prior TIBTRE reports; `0x0071C730`, `0x00483780`, `0x004838E0`)
- `[RESOLVED] OQ2 - What does `+0xC9A` mean? -> inherited `TechnoTypeClass Invisible=` byte.` (evidence: `0x00714A97..0x00714AAB`, string `0x00843944`)
- `[RESOLVED] OQ3 - What is the default for `+0xC9A`? -> zero/false.` (evidence: `0x007113D8`, with `BL=0` from `0x00710B00`)
- `[RESOLVED] OQ4 - What does `+0x1701` mean? -> `BuildingTypeClass InvisibleInGame=` byte.` (evidence: `0x00460DEB..0x00460E01`, string `0x0081A8CC`)
- `[RESOLVED] OQ5 - What is the default for `+0x1701`? -> zero/false.` (evidence: `0x0045E206`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`)
- `[RESOLVED] OQ6 - Does `InvisibleInGame=yes` modify `+0xC9A`? -> yes, parser writes `+0xC9A=1` and `+0xC9B=0` after reading true.` (evidence: `0x00460E07..0x00460E10`)
- `[RESOLVED] OQ7 - What is the `CanPlaceTiberium` polarity? -> reject only when both bytes are zero; either nonzero bypasses the building rejection.` (evidence: `0x00483948..0x0048395A`)
- `[RESOLVED] OQ8 - Are these branches active in YR? -> yes during normal active gameplay when a candidate cell has a live building object.` (evidence: `g_GameActive` gate and TIBTRE path addresses)
- `[RESOLVED] OQ9 - Do stock YR buildings set plain `Invisible=yes`? -> no exact-key assignments found in `rules.ini` or `rulesmd.ini`.` (evidence: exact-key INI scan)
- `[RESOLVED] OQ10 - Do stock YR buildings set `InvisibleInGame=yes`? -> yes, sixteen lamp/light building types in `rulesmd.ini`.` (evidence: lines `17266`, `17287`, `17437`, `17488`, `17512`, `17536`, `17561`, `17583`, `17607`, `17631`, `17655`, `17679`, `17703`, `17727`, `17751`, `17775`)
- `[RESOLVED] OQ11 - Are the stock users actual BuildingTypes? -> yes, they are listed in `[BuildingTypes]` in `rulesmd.ini` lines `1218..1221`, `1233..1236`, and `1446..1453`.` (evidence: INI scan)
- `[RESOLVED] OQ12 - Can TIBTRE placement pass on a live building cell in stock data? -> yes for the stock invisible lamp/light building types if all later `CanPlaceTiberium` gates also pass; ordinary visible buildings reject.` (evidence: gate polarity plus stock INI census)
- `[RESOLVED] OQ13 - Can TIBTRE placement pass on a live building cell in normal modded data? -> yes if a building type has either `Invisible=yes` or `InvisibleInGame=yes`, subject to later gates.` (evidence: parser and gate)
- `[RESOLVED] OQ14 - Does Rust parse the stock exception? -> partially; it parses `InvisibleInGame` but not plain `Invisible`.` (evidence: `src/rules/object_type.rs:646..649`, `:1083`, `rg Invisible`)
- `[RESOLVED] OQ15 - Does Rust terrain spawning check live buildings? -> no direct type-aware building gate exists in `can_accept_tiberium`.` (evidence: `src/sim/terrain_spawn.rs`)
- `[DEFERRED] OQ16 - Which stock maps place TIBTRE adjacent to invisible lamp buildings?` (category: out-of-scope; reason: this slot targets byte identity and stock type users, not scenario placement census; next-step-if-pursued: scan stock maps' `[Terrain]` and `[Structures]` sections for TIBTRE/lamp adjacency)
- `[DEFERRED] OQ17 - Exact runtime presentation of ore under invisible lamp buildings?` (category: requires-different-system-context; reason: placement acceptance is proven, but draw/order interaction belongs to render/object visibility; next-step-if-pursued: runtime capture or render-path trace for ore overlays beneath invisible light buildings)

Adversarial checks answered from evidence:

- If `InvisibleInGame=yes` but `Invisible=no` is also specified earlier, the building reader runs after `TechnoTypeClass::ReadINI` and forces `Invisible=1`.
- If `Invisible=yes` and `InvisibleInGame=no`, `+0xC9A` alone bypasses the building rejection.
- If `RadarVisible=yes` and `InvisibleInGame=yes`, the building reader force-clears `+0xC9B`; `RadarVisible` is not used by this gate anyway.
- If the building is dead (`health <= 0`), this branch does not reject.
- If `g_GameActive==0`, this branch is skipped; normal TIBTRE gameplay is active-game and reaches it.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary live visible buildings reject TIBTRE target cells | `0x00483937..0x0048395A` | missing or only indirectly approximated by occupancy/path surfaces outside `can_accept_tiberium` | `src/sim/terrain_spawn.rs::can_accept_tiberium`; entity/occupancy lookup surface | Reject target cells containing a live structure whose type is neither invisible nor invisible-in-game | `tibtre_spread_rejects_live_visible_building_cell` | Do not rely on path walkability or generic occupancy as the only building gate; it loses the type exception and active/dead distinction |
| `InvisibleInGame=yes` live buildings do not reject TIBTRE target cells | `0x00460DF2..0x00460E10`; `0x00483948..0x0048395A`; stock lamp INI lines | partially representable; Rust parses `invisible_in_game` but terrain_spawn does not read it | `src/rules/object_type.rs`; `src/sim/terrain_spawn.rs`; structure query helper | Allow the building branch to pass when the structure type has `invisible_in_game=true`, then continue later cell gates | `tibtre_spread_allows_invisible_in_game_lamp_building_if_other_gates_pass` | Do not make "any building present" a final rejection if aiming beyond common visible-building cases |
| `Invisible=yes` also bypasses the building rejection | `0x00714A97..0x00714AAB`; `0x00483948..0x00483950` | missing parser field; no stock explicit users | `src/rules/object_type.rs` if modded parity is in scope | Add a generic/inherited `Invisible` type flag before supporting modded invisible buildings exactly | `tibtre_spread_allows_modded_invisible_building_without_invisible_in_game` | Do not conflate `Invisible`, `RadarInvisible`, and `InvisibleInGame`; only `Invisible` and `InvisibleInGame` are read here |
| `InvisibleInGame=yes` force-sets `Invisible=1` and clears `RadarVisible=0` | `0x00460E07..0x00460E10` | Rust parses `InvisibleInGame` but no `Invisible` side effect is represented | `src/rules/object_type.rs` | For any future `Invisible` field, preserve the building-reader side effect order | `building_invisible_in_game_forces_invisible_and_radar_visible_false` | Do not parse fields as independent final values if both keys exist on a building |

Negative Facts / Do Not Do:

- Do not label `+0xC9A` as `InvisibleInGame`; it is inherited `Invisible=`.
- Do not label `+0x1701` as `Invisible`; it is building-only `InvisibleInGame=`.
- Do not use `RadarInvisible`, `RadarVisible`, `Selectable`, `LegalTarget`, `CanC4`, or `Insignificant` for this TIBTRE building exception; none are read by `CanPlaceTiberium`.
- Do not state that all stock buildings reject TIBTRE placement. Sixteen stock invisible lamp/light building types bypass the building branch.
- Do not state that the exception guarantees placement. It only bypasses the building branch; later land, overlay, slope, and `AllowTiberium` gates still apply.

Remaining Uncertainty:

- Full stock scenario/map adjacency census is not done. This report proves stock type availability, not that a shipped map necessarily places TIBTRE next to one of these lamps on a valid target tile.
- Runtime visual layering of ore placed under an invisible light building is not traced here.

Stale Docs / Follow-up Docs:

- `CELL_VALIDATION_TIBERIUM_PLACEMENT_REPORT.md` is broadly correct for these two bytes but its statement "No standard YR building has `Invisible=yes` or `InvisibleInGame=yes`" is stale/misleading. Replacement: "No stock YR type explicitly sets `Invisible=yes`, but sixteen stock building light/lamp types set `InvisibleInGame=yes`; the building reader then forces inherited `Invisible=1` and `RadarVisible=0` for those types."
- `src/rules/object_type.rs:646..648` comment says logical-only buildings such as bridge anchors and no stock targetable building set this. Replacement: "`InvisibleInGame=yes` on BuildingType. Stock YR uses it on invisible light/lamp buildings; it also suppresses ordinary targeting interactions and bypasses the `CanPlaceTiberium` live-building rejection."

## Sources

- Ghidra decompiled/read-only: `CellClass::CanPlaceTiberium @ 0x004838E0`.
- Ghidra assembly contexts: `0x00483948`, `0x00483952`, `0x00714A97`, `0x00714AAB`, `0x00460DF2`, `0x00460E01`, `0x00460E09`, `0x00460E10`, `0x007113D8`, `0x0045E206`.
- Ghidra strings: `Invisible @ 0x00843944`, `InvisibleInGame @ 0x0081A8CC`, `RadarVisible @ 0x00843934`.
- INI files checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust files checked: `src/rules/object_type.rs`, `src/sim/terrain_spawn.rs`.
