# Final-load RecalcZoneType object occupation — Ghidra Research Report

**Addresses:** `0x00483C80`, owner `0x00686B20`, final caller `0x00568BB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active-YR `CellClass::RecalcZoneType` object-list/building/Terrain predicate and precedence reached by final `InitCellAttributes(0)`, including load-time list construction and current Rust comparison.
**Non-Scope:** save/replay reconstruction, post-load destruction, full pathfinding consumers, and intentional corrupt-memory recovery.
**Confidence:** HIGH for the claimed slice
**Active in YR:** Yes. Building fence sub-branches are executable but inactive in stock retail data; Terrain is active.

## 1. Overview

`ScenarioClass::Full_Init` performs one `RecalcAttributes(-1)` sweep before objects, constructs Terrain and Technos, then calls `MapClass::InitCellAttributes(0)`, whose real-cell sweep calls `RecalcAttributes(-1)` again. `RecalcZoneType` reads the live ground `FirstObject` list at that final sweep. On stock retail data, Terrain is the only object kind that changes the reduced zone; ordinary Buildings and all other object kinds do not.

The current pending-authored Rust load path does not carry constructed Terrain occupation into `ResolvedTerrainGrid`, so its final authored Recalc can retain `terrain_object_occupation=None` and zone `GROUND` where native returns `BUILDING` (mask other than 7) or `WALL` (mask 7).

## 2. Class Layout / Key Offsets

| Owner | Offset | Width | Verified role |
|---|---:|---:|---|
| `CellClass` | `+0x24` | coord | playfield query input |
| `CellClass` | `+0x44` | dword | overlay type index, `-1` absent |
| `CellClass` | `+0x4C` | dword | reduced zone output |
| `CellClass` | `+0xE4` | pointer | ground `FirstObject` head |
| `CellClass` | `+0xE8` | pointer | `AltObject`; never read here |
| `CellClass` | `+0xEC` | dword | base `LandType` (corrects stale `+0x48` claims) |
| `ObjectClass` | `+0x00/+0x30` | pointers | vtable / next ground-list object |
| `ObjectClass` | `+0x8C` | byte | list-layer selector; constructor writes zero for Terrain |
| `TerrainClass` | `+0xC8` | pointer | `TerrainTypeClass` |
| `BuildingClass` | `+0x21C/+0x520/+0x618` | pointers/dword | owner / type / laser-fence frame-state |
| `TerrainTypeClass` | `+0x2A8/+0x2AC` | i32 | temperate / snow occupation values |
| `BuildingTypeClass` | `+0x16BF/+0x16C0` | bool | `LaserFence` / `FirestormWall` |
| owner house | `+0x1FA` | bool | Firestorm activation byte; constructor default zero |

Terrain identity is not label-derived: `vtable__TerrainClass-4 @ 0x007F5228` contains COL `0x0080C3E0`; `COL+0x0C` contains TypeDescriptor `0x00842D48`, whose name is `.?AVTerrainClass@@`; slot `vtable+0x124 @ 0x007F5350` contains `0x0071BFB0`, whose body is the Terrain `Mark` path.

## 3. Core Logic

Priority in `CellClass::RecalcZoneType @ 0x00483C80` is strict and terminal unless stated:

1. Outside `MapClass::Is_Cell_In_Playfield(..., 1)` -> zone 7 (`0x00483C85..0x00483CA3`).
2. Overlay present: Crushable `+0x22D` -> 1; Wall `+0x2A8` -> 2; overlay Land Wheel speed exactly `0.0` -> 6; `IsARock +0x2B5` -> 6; `IsRubble +0x2B4` -> final default 0 (`0x00483CA4..0x00483D24`).
3. Base Land index 2 -> 4; index 6 -> 3; Wheel speed `<= 0.01` -> 6 (`0x00483D2A..0x00483D71`).
4. Walk ground list from `Cell+0xE4`, dispatching `WhatAmI` through vtable `+0x2C`, then advancing through `Object+0x30` (`0x00483D72..0x00483DD2`).
5. Finite fallthrough -> zone 0 (`0x00483DD4`).

### Object predicate and precedence

- Only `WhatAmI==6` (Building) and `WhatAmI==0x24` (Terrain) have cases; Units, Aircraft, Infantry, Anims, and every other type are ignored.
- Building reads type at `+0x520`. Ordinary Buildings only continue.
- `FirestormWall!=0` reads owner `Building+0x21C`, then byte `owner+0x1FA`; nonzero returns zone 6, zero continues (`0x00483D8E..0x00483DAE`).
- `LaserFence!=0` with `Building+0x618` neither 12 nor 8 writes zone 6 but does **not** return (`0x00483DB0..0x00483DCD`). Every finite normal termination overwrites it with 0; a later Terrain/active Firestorm return supplies its own result. It is not a final-output blocker by itself.
- Terrain selects snow only when `Scenario+0x1258 == 1`; every other YR theater uses the temperate field (`0x00483DDF..0x00483E18`).
- The selected full 32-bit occupation value is compared exactly with 7: exactly 7 -> zone 2; every other value, including 0, negative, or above 7 -> zone 5. There is no mask in this function.
- The first Terrain encountered returns. `AddContent @ 0x0047E8A0` prepends non-Buildings and appends Buildings, so later successfully constructed Terrain at a duplicate cell is encountered first and wins; Buildings loaded later remain behind it.
- Null `FirstObject` yields 0. `AltObject` is ignored. There are no alive, limbo, ownership, or pointer-validity filters: a stale-but-dereferenceable Terrain entry still counts. Invalid pointers can fault; an ignored-object cycle can loop forever.
- Overlay/base-land branches precede the list, so Terrain matters only when those branches fall through.
- Overlay Wheel uses exact equality to 0.0 (`TEST AH,0x40`); base Wheel uses `<=0.01` (`TEST AH,0x41`). Unordered/NaN follows the impassable branch in both comparisons.

### Load ordering and ordinary construction

- `Full_Init @ 0x00686B20`: overlay owner at `0x00687A34`; first real-cell Recalc loop at `0x00687A43..0x00687A6B`; Terrain reader at `0x00687A74`; object sections at `0x00687AA7/0x00687ABF/0x00687ACB/0x00687AEA`; final `InitCellAttributes(0)` at `0x00687B8C..0x00687B9C`.
- `InitCellAttributes @ 0x00568BB0` leaves `FirstObject` intact and calls `RecalcAttributes(-1)` at `0x00568DEA..0x00568DF4` for each real cell.
- `TerrainClass::Read_Map_Section @ 0x0071CA70` constructs each accepted `[Terrain]` row in entry order. `Find_Or_Allocate @ 0x0071E2A0` rejects only `none`/`<none>`, finds known types, or allocates an unknown type with constructor defaults 7/7 (`0x0071DA80`). Allocation failure produces no object.
- `ObjectClass` constructor writes Terrain layer byte `+0x8C=0` (`0x005F3909`, `0x005F396F`). `TerrainClass::Unlimbo @ 0x0071D000` calls base Unlimbo first; its Mark virtual reaches `TerrainClass::Mark @ 0x0071BFB0`, `EnterCell_AddToMultiCells @ 0x005683C0`, `AddContent @ 0x0047E8A0`, then immediate `RecalcAttributes(-1)` at `0x005684DF..0x005684E1`.
- Terrain Unlimbo clears a resource-source overlay only **after** that Mark/Recalc (`0x0071D012` before `0x0071D0E7..0x0071D12B`). The immediate zone can therefore be stale for that edge; final `InitCellAttributes(0)` repairs it from the post-clear overlay plus live Terrain list.
- The pre-object sweep sees an empty list. Terrain construction changes eligible cells immediately; ordinary Techno construction does not, except the stock-inactive Firestorm conditional. The final sweep reproduces live post-object state.

## 4. INI Keys / Retail Activation

| Key | Reader/default | Retail result | Effect |
|---|---|---|---|
| `TemperateOccupationBits` | `0x0071DEA0`, i32, default 7 | stock values 4/5/6/7 | exact 7 -> zone 2; otherwise 5 |
| `SnowOccupationBits` | `0x0071DEA0`, i32, default 7 | stock values 1/2/3/4/6/7 | exact 7 -> zone 2; otherwise 5 |
| `LaserFence` | `0x0045FE50`, bool, default false at `0x0045E145` | no active stock binding | executable but stock-inactive; dead final write |
| `FirestormWall` | `0x0045FE50`, bool, default false at `0x0045E14B` | no active stock binding | executable but stock-inactive |

`ini/rulesmd.ini` and the installed retail map set (`mapsmd03.mix`, `multimd.mix`, `expandmd01.mix`, `MAPS01.MIX`, `MAPS02.MIX`, `MULTI.MIX`, loose `.mmx/.yro/.map`) contain no literal active `LaserFence=` or `FirestormWall=` binding. OpenTS `code/cell.cpp::Recalc_Passability` was used only as a navigation lead; active-YR assembly above decides, including YR rock/rubble and theater behavior.

## 5. Integration Points

The sole direct caller of `RecalcZoneType` is `RecalcAttributes @ 0x0047D2B0`. The relevant owners are `Full_Init @ 0x00686B20`, final sweep owner `InitCellAttributes @ 0x00568BB0`, and construction chain `0x0071CA70 -> 0x0071BB90 -> 0x0071D000 -> 0x005F4EC0 -> vslot+0x124/0x0071BFB0 -> 0x005683C0 -> 0x0047E8A0/0x0047D2B0`.

## 6. Current Rust Implementation Status

- `src/map/resolved_terrain.rs:96` `recalc_zone_type` matches stock priority once it receives the right `Option<u8>`; `Some(7)` -> WALL, every other `Some`, including zero, -> BUILDING.
- `PendingAuthored` cells intentionally start with `terrain_object_occupation=None`; `LoadCellRecalcState` (`:425`) has no live object-list/occupation input, and `finish_authored_load_cell_projection` (`:1793`) reuses the cell field.
- `src/sim/terrain_spawn.rs:788` constructs Terrain after installing the resolved grid but calls only `mark_terrain_raw_occupation` (`:835`); it does not update resolved occupation/zone.
- Runtime `mark_terrain_occupation` at `src/sim/terrain_object.rs:433` already projects `Some(bits)`, including `Some(0)`, and recomputes zone, but load construction does not use that route.
- `occupation_bits_for` (`:386`) masks `&0x07`, unlike native's full-i32 comparison. This is harmless for stock retail values but remains a non-stock exactification residual. Rust also skips unknown Terrain names while native allocates them with default 7; stock retail names are known.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| owner and two sweeps | verified | `0x00686B20`, `0x00568BB0` | none |
| full Recalc priority | verified | assembly `0x00483C80..0x00483E28` | none |
| null/stale/list ordering | verified | `0x00483D72..0x00483DD4`, `0x0047E8A0` | none in well-formed load |
| Terrain construction/recalc | verified | `0x0071CA70`, `0x0071D000`, `0x005683C0` | none |
| Building flags/defaults | verified | `0x00483D8E..0x00483DCD`, `0x0045DD90`, `0x0045FE50` | none for stock retail |
| Rust comparison | verified | paths/lines in Section 6 | implementation required |
| corrupt-memory recovery | deferred | native has no checks | out of claimed parity scope |

## 8. Open Questions — Final State

- `[RESOLVED] OQ1 — Does final Init run after objects? -> yes, with argument 0 after Terrain and object sections` (evidence: `0x00687A74..0x00687B92`).
- `[RESOLVED] OQ2 — What list and precedence are used? -> ground FirstObject after overlay/base branches; first returning Terrain/active Firestorm wins` (evidence: `0x00483C80..0x00483E28`).
- `[RESOLVED] OQ3 — What do null, stale, and alternate entries do? -> null defaults 0; Alt ignored; no lifecycle validation` (evidence: `0x00483D72..0x00483DD4`).
- `[RESOLVED] OQ4 — Does construction change the first-sweep result? -> Terrain links and recalculates immediately; final sweep observes the list and repairs post-Mark ore clearing` (evidence: `0x0071D012..0x0071D12B`, `0x005684BB..0x005684E1`).
- `[RESOLVED] OQ5 — Are ordinary Buildings active blockers in retail? -> no; only data-disabled fence flags have cases` (evidence: `0x00483D8E..0x00483DCD`, retail data scan).
- `[RESOLVED] OQ6 — Is occupation masked? -> no in native comparison; exact full i32 equals 7` (evidence: `0x0071DEA0`, `0x00483DF5..0x00483E1E`).
- `[RESOLVED] OQ7 — Does current pending Rust finalization receive live occupation? -> no` (evidence: Rust surfaces in Section 6).
- `[DEFERRED] OQ8 — What should Rust do for intentionally corrupt cyclic/pointer-invalid lists?` (category: `out-of-scope`; reason: native has no recovery and active retail load constructs well-formed lists; next-step-if-pursued: define a Rust safety policy separately without calling it native parity).

Zero-add pass re-read `0x00483C80` and construction callees and added no open item. Adversarial checks answered: empty list, zero mask, duplicate Terrain, stale entry, Terrain-on-resource-overlay. Cold spot checks re-read `0x00483C80` and `0x0071D000/0x005683C0`; both confirmed the findings above.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| final Recalc reads live Terrain after construction | `0x00687B92`, `0x00483D72..0x00483E1E` | missing | `terrain_spawn::construct_terrain_objects`, resolved grid/load finalizer | project each constructed Terrain's selected occupation into live resolved state before final authored sweep | clear-land TREE mask 4 final zone BUILDING; mask 7 final zone WALL | do not pre-project before native construction point |
| zero is present and yields zone 5 | `0x00483DF5..0x00483E1E` | load path drops it | same plus `terrain_object_occupation` | preserve `Some(0)`, not absence | explicit type mask 0 yields BUILDING | do not use sparse nonzero occupation as presence authority |
| list ordering makes last successful duplicate Terrain decisive | `0x0047E8A0`, `0x0071CA70` | final load unchecked | Terrain construction ownership | deterministic last-entry projection for final load | two same-cell Terrain rows with 7 then 4 finish BUILDING | do not let later Buildings replace Terrain authority |
| post-Mark resource clear requires final recomputation | `0x0071D012..0x0071D12B`, `0x00687B92` | occupancy missing during final recompute | authored overlay final sweep | final sweep combines cleared overlay with live Terrain | ore-source Terrain ends with cleared overlay and Terrain-derived zone | do not certify the immediate pre-clear Recalc as final state |

**Stale docs / follow-up:** replace any `Cell LandType +0x48` claim with `+0xEC`; replace “LaserFence can leave zone 6” with “it writes 6 but finite fallthrough overwrites it; only a later terminal branch decides.”

## 10. Ghidra Annotation Candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x00483DCA` | existing body/comment | note LaserFence write is non-terminal and final-fallthrough-dead | comment | assembly CFG to `0x00483DD4` | worker-report-only |

No metadata was changed.

## Sources

- Live active-YR `gamemd.exe` functions cited above; image base `0x00400000`.
- `docs/research/CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`; `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`; `CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md`; `pathfinding/FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`.
- Retail `ini/rulesmd.ini`, installed retail map archives, current Rust files in Section 6, and OpenTS `code/cell.cpp` as a non-authoritative lead.
