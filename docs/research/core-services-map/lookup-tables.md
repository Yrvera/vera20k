# Core Service Profile — Lookup-Table Substrate (`lookup-tables`)

**Slug:** `lookup-tables`
**Primary doc:** `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md` (+ six family docs under `docs/research/substrate/tables/`)
**Date:** 2026-06-25
**Authority order:** binary → Ghidra → docs. The substrate study docs are Ghidra-verified and cite addresses; treated as the primary evidence base here. Edges below are sourced to the consumer-function tables in those docs (which name the exact reader function), not re-derived.

---

## Purpose

A **pure, read-only, deterministic** data layer: the static lookup tables that many other engine services consult but none of which hold mutable state, draw RNG, or read a global. Six families:

1. **Facing / direction** — 8-dir cell-delta + lepton-delta tables, drive-track turn/raw/point tables, DRAGON 32-way frame map, facing-quantization formula.
2. **Cell-spread** — the AoE spiral offset table (369 entries) + per-radius count table.
3. **Path-neighbor** — per-direction A* geometry: pointer-stride offsets, edge-cost base table, direction epsilons, bridge-flank/ramp consts, reopen tolerance.
4. **Bridge-overlay (classifier/range only)** — Latin-square jitter, overlay byte bands, destruction-overlay next-tile tables, tileset window. (Bridge damage *state* and RNG are out of scope.)
5. **LandType/SpeedType/Passability** — speed[12][8] float multipliers, buildable bits, the passability matrix const.
6. **Remap/Palette/Sound** — Priority→ColorScheme table + ColorScheme remap palettes (render-facing); VocClass name→index table + control/type/priority flag tables (audio-facing).

It is the **oracle** stateful systems consult. Migration kind is **DATA-PARITY + API-CONSOLIDATION** (exact-dump-equality), *not* the shadow→invert ceremony — a constant table is byte-equal to gamemd or it is a bug.

In the canonical service graph this maps to several frontier slugs that haven't been profiled as their own services; this profile is the home for the **static read-only tables** themselves. Note: family 6 deliberately straddles `lookup-tables` (the table data) and `rules-class` (rules-parsed/embedded color-scheme + Voc data) — see Open edges.

---

## Owns

Pure const data + integer/SimFixed helpers. No mutable state, no globals, no RNG, no interior mutability.

- **Direction (family 1):** `g_DirectionOffsets` (cell-delta `short[8][2]`, `0x0089F688`, BSS/runtime-filled), `g_DirectionDeltaX/Y` (lepton-delta, `0x0089F6D8`/`0x0089F6DC`, =cell×256, BSS), RawTrack array `0x007E7A28`, TurnTrack table `0x007E7B28`, TrackPoint arrays (`0x007E64F8`…), DRAGON frame map `0x007F4890` (`(28-i)&31`), quantization formula `((f>>4)+1)>>1 & 7`. Init routines `0x0049F2F0` (cell) + `0x0049F3A0` (lepton).
- **Cell-spread (family 2):** `OFFSET_TABLE[369]` `0x00ABD490`, `COUNT_TABLE[12]` `0x007ED3D0` (`[1,9,21,37,61,89,121,161,205,253,309,369]`).
- **Path-neighbor (family 3):** `g_CellNeighborOffsets_8Dir` `0x007e3774` (`{-512,-511,1,513,512,511,-1,-513}`), `g_AStar_EdgeCost_BaseTable` `0x0081870c` (`{1,1000,1,1,60,20,8,10000}` f32), `DirectionEpsilon` `0x0081872c` (`{.001...008, tube=0}`), delta→dir `0x007e3760`, reconstruct-dir `0x0081874f`, bridge-flank `0x007e3710`/`0x007e3730`, bridge-diag consts `0x007e37b4/b8/bc`, reopen tolerance (double) `0x007e37c0` (`1.00903`), closed-index offsets `0x0089a304` (width-derived).
- **Bridge-overlay (family 4):** Latin square `0x0081CC30`, overlay byte bands (0x4A..0x63 / 0xCD..0xE6), destruction-overlay tables, `TILESET_WINDOW=0x10`.
- **LandType (family 5):** speed `0x0089EA40` (BSS), `PASSABILITY[[u8;8];13]` `0x0082A594`, shares `EDGE_COST_BASE` with family 3.
- **Remap/Palette/Sound (family 6):** `PRIORITY_TO_SCHEME[9]` `0x0083ed14` (`{3,11,21,29,13,25,17,15,5}`) + fallback `0x0083ed1c` (`0xFFFFFFFF`); ColorScheme object layout (remap palette `+0x04`); Voc control/type/priority flag tables `0x008160c0`/`0x00816048`/`0x00816018`.

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `Foundation_direction_table_init` | `0x0049F2F0` | fills cell-delta table at startup |
| (lepton init, adjacent fn) | `0x0049F3A0` | fills lepton-delta table |
| `MapCoord_StepByDir_GetCell` | `0x00481810` | (dx,dy) neighbor primitive; 47+ callers |
| `MapCoord_Step_By_Direction` | `0x0042D490` | generic step; reads cell table |
| `AStar_main_loop` | `0x00429a90` | 9-dir neighbor expansion; reads `0x007e3774`/`0x0089a304`/`0x0081872c` |
| `AStar_compute_edge_cost` | `0x00429830` | per-edge cost; reads `0x0081870c`/flank tables |
| `AStar_reconstruct_path` | `0x0042aa90` | dir emission; reads `0x0081874f` |
| `Path_smooth_corners` | `0x0042b210` | reads `g_DirectionOffsets` |
| `SessionClass__PriorityToColorScheme` | `0x0069A310` | priority→scheme lookup |
| `HouseClass__InitColor` | `0x50B840` | extracts base RGB from scheme |
| `HouseClass__ComputeRemap` | `0x50BA00` | computes bright RGB |
| `VocClass__FindByName` | `0x007514d0` | name→index (case-sensitive, lowest-index) |
| `VocClass__PlayAtPos` | `0x00750920` | positional SFX dispatch; ~75 callers |
| `VocClass__CalcVolumeAndPan` | `0x00750ac0` | volume/pan/shroud gate |
| `CCINIClass__ReadSoundList` | `0x00525430` | name-list → index DVC |
| AoE cell-spread init | `0x00561910` | fills `OFFSET_TABLE[369]` |

---

## Tick / render position

**N/A — has no tick phase.** It is a stateless data layer with no per-tick step in the `LogicClass` spine. It is *read synchronously inside* other services' phases:
- **Commands → ground/air movement** phase: facing/lepton tables + drive-track + path-neighbor geometry (locomotors, A*).
- **Turrets + combat** phase: cell-spread spiral + bridge-overlay classifiers (AoE), facing quantization (muzzle 8-way).
- **Render pass:** ColorScheme remap palette (per-frame tint, radar dots), DRAGON frame map (rotating projectile SHP).
- **Audio (event-driven, off the tick):** Voc name→index + flag tables + volume/pan.
- **Startup (pre-gameplay):** the BSS direction/lepton tables are filled by their init routines before the first tick.

---

## Depends-on (outgoing edges)

This is a **leaf**. Its only true dependency is `util/fixed_math` (SimFixed / lepton helpers, ftol), which is below the service layer and not a canonical service slug. There are **no outgoing edges to any canonical core service** — the tables read no other service's state, draw no RNG, and call no other subsystem. (Master doc §1: "never holds mutable state, never draws RNG, never reads a global.")

- `util/fixed_math` (NOT a service slug) — via SimFixed arithmetic + `ftol` (cell-spread index `ftol(CS+0.99)`, threshold `ftol(CS*256)`); facing quantization integer math. Evidence: master doc §2 module tree ("The ONLY dep of the sim table service"); cell-spread §4c.

Family 6 (rules-parsed color-scheme + Voc data) is *populated from* INI at load, but that population is the rules loader's job, not the table service calling into `ini-parsing`/`rules-class` at read time — so it is not modeled as a runtime outgoing edge. The const tables themselves are embedded data with no caller. (See Open edges for the family-6 ownership split.)

---

## Used-by (incoming edges)

Every edge below names the **specific reader function/global** in gamemd that creates it (from the family-doc consumer tables).

| Consumer slug | Via (symbol/global) | Evidence |
|---|---|---|
| **pathfinding-helpers** | `AStar_main_loop 0x00429a90` reads `g_CellNeighborOffsets_8Dir 0x007e3774`, `0x0089a304`, `DirectionEpsilon 0x0081872c`; `AStar_compute_edge_cost 0x00429830` reads `EdgeCost_BaseTable 0x0081870c` + bridge-flank `0x007e3710/30`; `AStar_reconstruct_path 0x0042aa90` reads recon-dir `0x0081874f`; `Path_smooth_corners 0x0042b210` reads `g_DirectionOffsets` | Path-neighbor §2.3 consumer table |
| **cell-map** (CellClass/MapClass) | `MapCoord_StepByDir_GetCell 0x00481810` (47+ callers) + `MapCoord_Step_By_Direction 0x0042D490` read `g_DirectionOffsets`; A* fetches `CellClass*` via `0x007e3774` 512-stride on the cell pointer array | Facing §2 reader fns; Path-neighbor §2.1 (512 = CellClass* pointer stride) |
| **techno-foot** (FootClass locomotors) | `WalkLocomotionClass__ProcessMovement 0x0075B5C4`, `DriveLocomotionClass__Process_Movement 0x004B2630` + `__Can_Use_Track 0x004B4B00` read lepton table `0x0089F6D8` + drive-track tables `0x007E7A28/B28` + quantization | Facing §2 reader fns ("EVERY locomotor's per-tick body translation") |
| **techno-foot** (turret/body facing) | `UnitClass__Constructor 0x007353C0` ROT clamp/shift; FacingClass trio `0x005B2950/90/C0` (homing) | Facing §2 reader fns |
| **damage-helpers** (warhead AoE / CellSpread) | combat AoE radius = `ftol(CS+0.99)` over `COUNT_TABLE 0x007ED3D0` + `OFFSET_TABLE 0x00ABD490`; scan order = ReceiveDamage/RNG/chain order | Cell-spread §1/§4 (master doc D1–D3) |
| **target-scoring** | `Mission_Hunt`-class neighbor scans use the same 8 compass deltas (`g_DirectionOffsets`) | Master doc §3a ("AI/`Mission_Hunt`-class scans") |
| **bridge-helpers** | bridge AoE / damage dispatch reads Latin square `0x0081CC30`, overlay bands, destruction tables, `TILESET_WINDOW`, and *imports* the cell-spread tables for AoE | Bridge-overlay §4.2/§7; master doc §3c |
| **cell-validation** | passability/occupancy validators key the `PASSABILITY` matrix `0x0082A594` (==1 passable) | LandType §2; master doc §6 (`PASSABILITY[[u8;8];13]`) |
| **factory-house** (color extraction) | `HouseClass__InitColor 0x50B840` + `__ComputeRemap 0x50BA00` consume `PRIORITY_TO_SCHEME 0x0083ed14` (via `SessionClass__PriorityToColorScheme 0x0069A310`) + ColorScheme remap palette → write House+0x56F9..56FE | Remap §2.1; fires at `Create_Houses` |
| **drawing-helpers / render** | ColorScheme remap palette (`+0x04`, 256×RGB) recolors every owned object every frame; DRAGON frame map `0x007F4890` selects rotating-SHP frame; muzzle 8-way anim in `Fire_At 0x006FDD50` | Remap §1; Facing §2 (render quantization) |
| **frontier-audio** | `VocClass__PlayAtPos 0x00750920` (~75 callers) + `CalcVolumeAndPan 0x00750ac0` resolve name→index via `FindByName 0x007514d0` and read control/type/priority flag tables `0x008160c0/48/18` | Remap §1/§2.2 |
| **rules-class** (load-time fill) | rules load resolves `[SoundList]`/per-sound + `[AudioVisual]` names to Voc indices via `CCINIClass__ReadSoundList 0x00525430`; 101 RulesClass sound fields keyed by Voc index | Remap §2.2; master doc U21 |

---

## Open / unverified edges

- **`g_DirectionOffsets` (`0x0089F688`) + lepton table (`0x0089F6D8`) values are UNVERIFIED-from-static (BSS/runtime-filled).** Read live = all zeros; values constrained to standard compass deltas by two anchors (the 512-stride table `0x007e3774` and the delta→dir map `0x007e3760`) but not bit-dumped. The *edges* are proven (consumer functions verified); the *table contents* are inferred. (Master doc OQ-3; Path-neighbor §2.2.)
- **`speed[12][8]` (`0x0089EA40`) is also BSS** — INI-derived, filled at rules load; static dump is zero. The edge to `cell-validation`/`pathfinding-helpers` is real but the values come from the init/INI path, not a static dump.
- **Family-6 ownership straddle (`lookup-tables` vs `rules-class`).** The master doc places the color-scheme + Voc *data* in `rules/`, not `sim/`. So the "owner" of family-6 tables is arguably `rules-class`; this profile lists them under `lookup-tables` because they are static read-only lookup data by nature. The graph may legitimately attribute the family-6 incoming edges (render/audio/factory-house) to either `lookup-tables` or `rules-class`. Flagged so the edge isn't double-counted or dropped.
- **Bridge-overlay → bridge damage STATE / RNG is explicitly NOT an edge of this service.** Bridge damage SM (CellClass+0x11E), collapse/repair, and the `BridgeStrength` RNG draws (Scen->Random) live in `sim/bridge_state` and belong to `bridge-helpers` + `random-scenario`, not here. The table service only feeds the classifier/range data. (Master doc §1, §3c.)
- **`EDGE_COST_BASE` is one shared table** read by both `pathfinding-helpers` and `cell-validation` (`0x0081870c`, single xref `AStar_compute_edge_cost`). Listed under both incoming edges intentionally; it is not two tables. (Master doc §3e; LandType §2.)
