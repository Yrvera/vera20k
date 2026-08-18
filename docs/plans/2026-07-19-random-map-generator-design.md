# Random Map Generator (RMG) Design

**Date:** 2026-07-19 · **Approach:** A (native-pipeline emulation module) — user-approved
**Parity bar:** bit-exact vs gamemd (same `.SED` seed+options → identical map), user-approved
**Scope:** full flow — Create Random Map dialog → `.SED` → generation pipeline → playable skirmish map. Map types 3/4 and the tiberium formula land after their approved RE follow-ups (blocked-on-research, not cut).

## Goal

A player can click Create Random Map, configure options, and launch a generated
skirmish map that is byte-identical to what gamemd.exe generates from the same
`.SED` — with the whole pipeline deterministic across peers.

## Architecture Context

- **Plug-in seam:** `MapLoadInitial { asset_manager, map_data }` (`src/app_init.rs:377`)
  → `load_map_from_initial`. Everything downstream (ResolvedTerrainGrid, atlas,
  lighting, spawn, radar) consumes the in-memory `MapFile` struct, not a file.
  A generated map only needs `MapHeader` + `cells` + `overlays` +
  `terrain_objects` + `waypoints` + `[Basic]/[Map]/[Lighting]` INI sections.
- **Cell authoring:** `MapCell { rx, ry, tile_index, sub_tile, z }`
  (`src/map/map_file.rs:140`); flat `tile_index` =
  `TilesetLookup::bounds()[set].start + offset` (`src/map/theater.rs:273`).
- **Launch flow:** `SkirmishLaunchSession.selected_map_file` →
  `classify_startup_session` → `pump_loading_after_present` →
  `load_map_initial_with_assets` (`src/app_loading.rs:432`, `src/app_init.rs:337`).
- **Existing utilities:** `util/native_x87.rs` (deterministic x87-subset
  soft-float), `util/ini_writer.rs`, `sim/rng.rs` (`SimRng`, r250-family —
  NOT reused here; RMG needs the exact native LFG+hash, see Components).
- **UI scaffolding already present:** sentinel record + Create Random Map
  button (log-stub at `src/app.rs:1333`), `RandMap.img` preview reader,
  mode gating via `random_maps_allowed`.

## Impact Analysis

New module + three integration points; no `sim/` changes.

| Surface | Change |
|---|---|
| `src/map/rmg/` (new) | entire generator |
| `src/app_init.rs` / `src/app_loading.rs` | `.SED`-sentinel branch producing `MapLoadInitial` from the generator instead of file load |
| `src/ui/skirmish_shell/*`, `src/app.rs`, `src/app_skirmish_shell_render/*` | Create Random Map modal (replaces log stub), preview integration |
| `src/skirmish_scenarios.rs` | sentinel capacity 2..8 (currently max 4 — confirmed drift) |
| `util/native_x87.rs` | extended op set (see x87 spike) |
| `util/ini_writer.rs` | `.SED` write |

Risk areas: x87-faithful `ln` (Box-Muller) is the single hardest bit-exactness
risk; RNG draw-count fidelity (rejection loops); tiberium formula currently
mis-documented (blocked on re-investigate). Determinism: generation is
pre-play, host-side; bit-exactness makes all peers agree; no tick order or
state-hash impact.

## Chosen Approach

**A — native-pipeline emulation module.** One `generate(options, deps) ->
GeneratedMap` in `src/map/rmg/` runs the exact native stage pipeline over
Rust-native state owners, emits a `MapFile`, and feeds the existing
`MapLoadInitial` seam. Dialog is a normal skirmish-shell modal writing
`RandMap.Sed` via `ini_writer`. Rust-native structure, gamemd-native
semantics: storage owners (`RmgScratch`), an order owner (the phase list in
`generate`), and per-phase plain functions that commit state in native order.

Rejected: B (generate-to-file; extra roundtrip native doesn't have — kept only
as a later debug/WAE-export tool), C (direct-to-resolved/sim construction;
bypasses the canonical map model, no parity gain).

## Tiny-Detail Ledger

Constraint set for /write-plan and implementation. Items marked **[AUDIT-FIX]**
supersede the named doc's original text per the 2026-07-19 verify-doc-swarm
(AUDIT_LOG.md). Items marked **[PENDING-RE]** are blocked on the approved
follow-up research and MUST NOT be implemented from the current doc text.

### RNG (`RmgRng`) — the determinism spine
1. LFG-XOR, R=250/S=103, 253-dword struct: +0x0 locked flag, +0x4/+0x8 lag
   indices, +0xC..0x3F3 state[250] (end offset corrected) [AUDIT-FIX slot 2].
2. Seed clamped 0..0xFFFF before seeding [doc: SED_WRITER §; verified 0x005975E0].
3. `Random__Seed 0x0065C6D0`: one 4-round hash pass per output dword; table-2
   instruction displacement is `0x00839690` with an `ADD EBX,4` pre-increment,
   so the **effective fetches are `0x839694..0x8396A0`** (the audit's claim
   that `0x48AAD7E4 @ 0x839690` is consumed was re-refuted; all 16 dwords are
   now recorded in the patched RNG doc §2.3) [AUDIT_LOG 2026-07-20 PATCHED
   entry]; unconditionally clears the locked byte at `0x0065C770`
   [AUDIT-FIX slot 2, re-verified].
4. Generator copies 0xFD (253) dwords into `g_MapGenRng` at pipeline entry
   [GHIDRA 0x0059897B..99B].
5. Range reduction constant at `0x007ED898` is `0x3DF0000000100000` — NOT
   bit-exact 2^-32; use the literal bit pattern, never `1.0/2^32`
   [AUDIT-FIX slot 2].
6. Draw-count contract includes rejection loops: uniform draws re-roll while
   out of range (defensive or real); region seed-cell pick can consume >1 draw
   [AUDIT-FIX slot 6]; Gaussian rejection windows re-roll (hills, patch sizes).
7. Uniform helper `0x00598030(min,max)` = `ftol(rand × (max−min+1) × K + min)`,
   reject > max [GHIDRA disasm 0x00598030].
8. Gaussian `0x005980C0`: Box-Muller — reject r²≥1, scale √(−2·ln(r²)/r²),
   **caches the second value**; cache consumption order is part of the draw
   contract [doc: RMG_TIBERIUM (confirmed by slot 5)].
9. Single stream (`g_MapGenRng`) for every phase; match seed decoupled
   [doc: RMG_RNG §4 (confirmed); Rust: match_bootstrap.rs:650].

### Options / .SED (`RmgOptions`)
10. `[RandomMap]` keys: Description, Width, Height, NumPlayers, Seed, MapType,
    Theater, Time, RegionSize, Ruggedness, Accessibility, WaterAmount,
    Tiberium, TiberiumLayout, Vegetation, UrbanPresence, Resources; ReadInt
    default = current field value (carry semantics) [doc: SED_WRITER, GREEN].
11. Constructor defaults: theater=0, maptype=1, resources=1, time=1,
    players=2, seed=−1, all else 0 [GHIDRA 0x00595680, audited ×2].
12. Normalizer `0x005975E0` clamps (parent-verified this session): Resources
    0..3, MapType 0..4, Time 0..3, NumPlayers 2..8, Tiberium 1..100,
    Width 0..3, Height 0..3, Seed 0..0xFFFF, all percents 0..100,
    **Theater UNCLAMPED**. (Parent generator doc's "theater 0..4" is stale.)
13. Description encodes options as hex-CSV (formats "%d","%x", selector
    variants "%Xh"/"$%X") [doc: SED_WRITER §3.5, GREEN].
14. `RMGMD.INI [General]`: `MaxTrees`→+0x2FC, `RMGMinimumTiberium`→+0x2BC,
    `RMGMaximumTiberium`→+0x2C0, `RMGLevelLightSettings`,
    `RMGVegetationMinimums/Maximums`, per-theater Ambient/tint vectors,
    `TemperateOrePatchLamps`/`SnowOrePatchLamps`; absent file ⇒ **outer-ctor
    defaults** (0x00595740, verified 2026-07-20): RMGMinimumTiberium=2500,
    RMGMaximumTiberium=5500, MaxTrees=500, +0x30C=4, +0x310=0 — the earlier
    "zero trees when absent" claim was refuted. **Extracted 2026-07-20 from `ra2md.mix`
    (also present in `ra2.mix`):** `RMGMinimumTiberium=900`,
    `RMGMaximumTiberium=1050`, `RMGLevelLightSettings=3,3,3,3`,
    `RMGVegetationMinimums=60,60,60,60,60`,
    `RMGVegetationMaximums=100,100,100,100,100`,
    `TemperateOrePatchLamps=TEMMORLAMP,TEMDAYLAMP,TEMDUSLAMP,TEMNITLAMP`,
    `SnowOrePatchLamps=SNOMORLAMP,SNODAYLAMP,SNODUSLAMP,SNONITLAMP`,
    `TemperateAmbientLight=75,100,75,35`, `SnowAmbientLight=75,100,75,55`,
    ambient RGB vectors per file, `MaxTrees=600`. File comments name the
    option buckets: MapType 0–4 = archipelago, continent, team continent,
    inland, mountainous; Time 0–3 = morning, day, dusk, night.

### Stage order (in `generate`)
15. water (types 3/4 && Water≠0 → water34 driver; else base water) →
    post-water finalizer → region partition → [3/4 only: rebuild+terracing,
    bridges, cliff drops, tile re-anchor] → **green spread 0x0059B740** →
    recalc → starts → tech buildings (maptype≠0) → tiberium → scratch clear →
    recalc → hills → LAT/trees(/rocks) → recalc → growth queues → cleanup →
    InitCellAttributes(1) → radar [GHIDRA decompile 0x00598960 this session;
    green-spread stage was missing from the parent doc].

### Water, base types (RMG_WATER doc + slot-3 fixes)
16. Shape selection/gating per doc §3 (confirmed); archipelago half-width
    `max(2, NumPlayers/2)` [confirmed].
17. Flood-fill iteration caps are **100** (counter 0..99 inclusive), not 99
    [AUDIT-FIX slot 3].
18. Finalizer 2×2 anchor checks neighbors **E (dir 2), S (dir 4), SE (dir 3)**
    — doc's "N/S/E" was the direction-mislabel bug class [AUDIT-FIX slot 3].
19. Finalizer tile placement is a 4-arg call: tile_id = `DAT_00AA0738 +
    variant_offset` with variant_offset = (mod-242 draw)/10, edge-cased for
    draws 240–241; position ptr arg; region-lookup arg; −1 [AUDIT-FIX slot 3].
20. Water variant bands: mod-201 draw / 40 ⇒ **6 outputs** (bands 0–4 span 40
    each; band 5 = draw 200 only, p=1/201) [AUDIT-FIX slot 3].
21. `CellClass+0x11A == 0` sub-tile guard on variant rewrite [doc, confirmed].
22. `0x87F7E8` is the MapClass cell iterator, not an RNG [doc correction,
    re-confirmed slot 3].

### Region partition (REGION doc + slot-6 fixes)
23. Scratch stride 0x50: +0x00 coord, +0x38 region id, +0x3C stamp, +0x45
    water flag (+0x4B per REGION doc — reconcile field at implementation
    against both docs' cited disassembly) [doc + this session].
24. Region object fields +0x08 id, +0x0C cell_count, +0x10 height/terrain,
    +0x14 flag, +0x16 packed seed cell, +0x1A done, +0x2C cell array
    [doc, audited exact].
25. BFS pass count = `4 + (cell_count>8000) + !mode34` (0x1F40) [audited exact].
26. Region seed-cell pick = rejection-sampling loop (may draw >1)
    [AUDIT-FIX slot 6].
27. BFS enqueue gate = `IsClearTile() OR green-LAT-membership(0x004867B0)` —
    no bridge check [AUDIT-FIX slot 6].
28. `0x0058E740/E9B0/D010` contain zero RNG draws [audited].
29. `g_DirectionOffsets` (0x0089F688) is runtime-initialized (zero on disk) —
    read live values before implementing any neighbor iteration [slot 6].

### Green spread (0x0059B740)
30. Seeds = green-LAT cells' clear neighbors in the 4 even directions;
    converts `min(list/3, 1000)` cells; one uniform pick (+rejection) per
    conversion; re-scans converted cell's even-dir neighbors
    [GHIDRA 0x0059B740 this session].

### Hills (this session's report)
31. Skip entirely if `Ruggedness × 0.0025 < 0.025` (R<10).
32. Random-walk: smooth W/N ×0.5 (height+velocity), NW diagonal tilt term
    clamped ±(R×0.0001+0.1); boundary velocity R×0.0025; shore-flagged cells:
    height≥0, velocity:=0.0025, shore-adjacent height seed 0.5; two
    rejection-sampled Gaussian draws/cell; |height|≤2.0; final ftol truncation.
33. Corner grid: h=level×0xF from 19-pattern table + level; cap 0xB4;
    lock on overlay/occupier/water-flag/non-morphable (IsoTileType+0x2E0);
    ±0xF per step; slope constraint |Δ|≤0xF propagated recursively with undo
    stack; locked-neighbor ⇒ fail + rollback.
34. Finalize: only if corner spread <0x10 && no overlay/occupier && !waterflag
    && tile morphable/clear/0xFFFF; level=min/15; slope from pattern match;
    tile = g_ClearTile (slope 0) else g_RampBase+slope−1.
35. Ramp patterns 1–18 (NW,NE,SE,SW ×15): as dumped (read_memory 0x0083FDD8).
36. 2×2 quad cleanup: slopes {5,6,7,8}→flatten; {11,12,9,10}→flatten, level+1;
    tile:=0xFFFF, +0x11A:=0, +0x11C:=0.

### LAT / trees / rocks (this session's report)
37. TEMPERATE probabilities: rough=Veg×0.0002 (shore-adjacent ×10 and sand:=0),
    sand=0.005, green=0.005; test order rough→sand→green with fresh draws.
38. Patch-size means: three uniform draws `ftol(rand×21/2³² + 20)` ∈ [20,40]
    (order: rough, sand, green); size = Gaussian×20+mean, clamp [4,80].
39. Patch placer: min-heap, priority = √(dx²+dy²) + rand×5/2³² jitter; one
    draw per admitted neighbor; admits clear, slope 0, no overlay/occupier,
    not in this patch.
40. Non-TEMPERATE: scratch fill rough=0.005 (green=0.001 written, unread);
    rough-only patches Gaussian 20±15 clamp [4,60]; **no rocks**.
41. Trees: count = `ftol((Width_opt×0.1 + 0.7) × Veg×0.01 × MaxTrees)`; ≤100
    patch iterations; density Gaussian 0.2±0.1 [0.05,0.4]; size Gaussian
    25±10 [10,35]; region ≤ size×25 cells (visited flag +0x47, water flag
    blocks); tree name draw uniform 0..25 → "TREE%d%d" (TREE00 miss quirk —
    runtime behavior pending); excluded land type 3.
42. Rocks (TEMPERATE only): quota = uniform [0, (H+4)×W×2/200] inclusive;
    attempts = quota×5; empty-overlay cells only; sand-LAT → SROCK01-05
    (overlay idx 168+0..4), clear/green-LAT → TROCK01-05 (173+0..4);
    `+0x11E:=0`; RecalcAttributes(−1). Index = 0-based [OverlayTypes]
    position (comment-gap at key 183 shifts key numbers — never use keys).

### Starts (START/SCORE docs + audits)
43. Loop: inner `0x00594B50` until nonzero, then `0x005A1FB0`; both return 1
    unconditionally (defensive loop) [audited].
44. Quota `DAT_00ABE028` default 4 (two write sites); proportional per-region
    distribution [audited].
45. Waypoints: `ScenarioClass+0x632+i*4` + metadata mirror
    `+0x11BC+(i+1)*4`; NumPlayers signed `<` loop; 400-cell flood-fill cap
    [audited exact].
46. Candidate attempt = exactly 2 draws (lane, index); selector `0x00594F40`
    = farthest-first, Euclidean √, +20 cross-zone bonus
    (`-(zone≠zone)&0x14` FADDP), zero RNG [audited exact].
47. 6×6 gate: rect is **dword-packed {x,y,6,6} (16 bytes)**; diamond-bounds
    pre-check on all 4 corners (DAT_00ABED04/08) BEFORE the cell walk
    [AUDIT-FIX slot 7].
48. Selector null-return: `[(cand_count<quota_snapshot)||quota_snapshot==0]
    && region_quota==0` [AUDIT-FIX slot 7].
49. View-region margin reads 4 globals starting at `DAT_0087F8E4` (+4 inset)
    [AUDIT-FIX slot 7].

### Tech buildings (this session)
50. maptype≠2: neutral house; count uniform 0..4; building uniform from
    `NeutralTechBuildings` pool (verified as Rules **[AI]** section reader,
    2026-07-20); ≤100 attempts; foundation cells all clear, unoccupied, same
    level as anchor, land≠3, !waterflag; place at cell×256+0x80 leptons;
    destruct on failure. maptype==2 (0x00595400): U{0,1,2} passes drawn once,
    one type per region call, ≤100 anchors from region cell list, same
    foundation rules [GHIDRA 0x00595400, decoded 2026-07-20].

### Tiberium (re-derived 2026-07-20 — RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK doc; RED doc patched to GREEN)
51. Ore base idx 102 (span 12), gems 27; density cap 11 (`+0x11E`); TIBTRE
    variant draw is 1..3 (never TIBTRE00), ore fields only [recheck doc].
52. Field count (NO RNG): `lerp = trunc(Tiberium% × 0.01 × (RMGMaximum −
    RMGMinimum) + RMGMinimum)` — truncated BEFORE the next multiply;
    `regionTotal = trunc(lerp × max(startCount(region+0x20), 0.5))`; skip
    region if slot count (region_sub[+0x10]) == 0 or regionTotal == 0;
    `perFieldBase = regionTotal IDIV slots` (signed truncating); per-slot
    size = `trunc(perFieldBase + Gaussian×50.0)` rejection-clamped ±100,
    negative ⇒ slot skipped. ftol truncates toward zero (CW 0x0E7F).
53. Placer gate order: in-playfield diamond (0x00578460) → real CellClass →
    IsClearTile (0x00486380) → fresh-empty admit (`+0x44==−1` && scratch
    `+0x3C != field_id`) → else revisit admit (`+0x11E<11` &&
    GetTiberiumType≠−1). Overlay/density writes land on the REAL CellClass;
    claims on scratch `+0x3C`; reseed wipes all `+0x3C` map-wide, 10 seeds
    max; BFS priority jitter is uniform [0,5] (never Gaussian).
53b. TiberiumLayout (MapSeed+0x58 = DAT_00ABE030, sole reader FUN_00594F40):
    per-region field-slot count = `trunc((TibLayout × 0.01 × 12.0 /
    NumPlayers + 2.0) × startCount)`; slots farthest-point-sampled with
    +20 cross-region penalty; that array IS region_sub. (Closes the
    start-scoring doc's DAT_00ABE030 MEDIUM item.)
53c. Gem/second pass: unconditional per region, one placer call per START:
    size = `trunc((meanDistToSlots − min) × 15.0) + 500` (closest start
    exactly 500), origin = start waypoint cell, gems iff Resources==3 &&
    maptype ∈ {1,3,4}, else ore.

### Map types 3/4 (internals decoded 2026-07-20 — RMG_MODE34_WATER_BRIDGES_TECH doc)
54. Driver: Water>20 ⇒ up to 10 tries of river carver 0x0059D510; always up to
    10 tries of lake grower 0x0059C920; +0x308 counters [GHIDRA 0x0059C580].
54b. River carver 0x0059D510: random diamond edge U[0,3]; Gaussian heading
    mean 7π/4−edge·π/2 σ=π/6 clamp ±π/4; width U{1..ftol(max(0.07·Water,1))}
    with ±w/2 walk (σ=0.5, skipped at w=1); per-step heading wobble σ=π/10
    clamped start±π/2 from step 6; 1%/step single branch (mean h+π/2 in
    [h+π/6,h+5π/6], failure kills parent); ≤1 bridge (needs MapSeed+0x310,
    step > U[35,125], |Δh|<1 rad, straight cross-section); 0.5%/step
    termination, min 40 steps; only rand-terminated rivers get a terminal
    lake; success → shore finalize (0x0057A0C0) + dilate + green; 70% canyon
    event (6-ring dilate, +4 levels on non-river cells, gated +0x30C==4);
    rollback restores tiles to 0 and level to (u8)+0x30C.
54c. MapSeed+0x310 (river-bridge enable) = one draw rand01 < 0.25f
    (0x007ED630) in init — ~25% of maps; draw consumed regardless.
54d. Lake grower 0x0059C920: quota = ftol(genW·genH·Water·0.008 + 100);
    phase ends when remaining ≤ 75; 200-attempt seed pick; size = bounded
    Gaussian [75, remaining] mean rem/3 σ rem/6; heap growth priority =
    dist·0.5 + 10·rand01 − 0.02·placed, cardinal neighbors; stale-queue-entry
    hard-fail; success needs placed>75 and >size/4.
55. Region rebuild threshold `ftol(2×W×H×(RegionSize×0.005+0.05))`; terracing:
    carve blob ~count/3−count/8, angle uniform [0,2π) (2π/2³² constant),
    Gaussian×π/8 steering, sub-region heights in ±4 steps (no 0 for 3/4),
    <101 cells ⇒ merge [GHIDRA 0x0058EBC0/0x0058D620].
56. Bridge pass: FUN_004A8BF0(0) = clear placement-cursor bits (no RNG);
    PlaceBridgeRamp_Low(cell,−1) per cell (only cell write +0x11B += 4,
    8-neighbor recursion; failure branch unreachable at group −1); regions
    rebuilt; then 0x0058F0C0 (neighbor-id vectors, zero RNG) and connector
    0x005905D0: land = 1 ramp per different-level neighbor pair (U[0,100]
    roll ALWAYS consumed though bonus dead via DAT_00ABE044=0), 100 attempts,
    5×5 density threshold ftol(10·(1−0.01·attempt)+5), g_RampBase tiles;
    water = 3-wide low-bridge decks (overlay 0x4A..0x62 patterns) between
    same-level land neighbors, 200 attempts, accept len < attempt/25+8
    [GHIDRA 0x0058EF10/0x0058F0C0/0x005905D0; deck-helper internals deferred].
57. Cliff drops: adjacency masks 0x83/0x38 with dir-3 neighbor match ⇒ 1-in-2
    draw ⇒ neighbor level −4; mask 0xE0 (+dir-5/dir-1 match) ⇒ own level +4,
    no draw [GHIDRA 0x005A19E0].
58. Water tile re-anchor variant families and block re-anchoring by
    subtile/blockW [GHIDRA 0x005A17F0/0x005A1350].

### Dialog / preview / launch (dialog doc GREEN + this session)
59. Control-ID→field map, trackbar ranges, init-from-defaults per dialog doc.
60. Randomize (0x621): theater = RandomRanged(0,100)>0x31; maptype
    RandomRanged(1,4); time/resources/width(=height) RandomRanged(0,3);
    derived fields via 0x00597260 (tiberium = resources×0x14); seed
    RandomRanged(0,0xFFFF); then clamp; preview destroyed; 0x6C5/0x6C3
    disabled.
61. Generate (0x620): disables exactly 13 controls incl. Cancel during
    generation; runs pipeline with preview arg 1; repaints preview at each
    stage checkpoint; copies 0x5E dwords + 0x178 bytes into cached state
    (DAT_00ABE150).
62. Accept: saves `RandMap.Sed` via writer 0x00597730; Choose Map selects the
    sentinel; launch keeps filename `RandMap.Sed` and runs `generate` with
    preview arg 0 (no repaint blocks) [launch-branch doc].
63. Preview surface dims/colors per GENERATETERRAINPREVIEW doc;
    `RandMap.img` written for the Choose Map thumbnail.
64. Progress strings/percent checkpoints (shell progress only when
    ScenarioClass+0x3598==0) — cosmetic order preserved for the dialog UX.

### x87 / FP
65. All FP phase math ran on x87 with 80-bit intermediates (float10); Rust
    implementation uses `util/native_x87.rs`-style deterministic emulation
    for: mul/add/sub/div chains in hills walk, Box-Muller (incl. `ln` via
    FYL2X semantics — SPIKE), sqrt (FSQRT is IEEE-exact), ftol truncation
    (`gamemd_ftol` semantics), and comparisons via FCOMP flag semantics.
66. FP constants are used verbatim by bit pattern (0.7 = 0x3FE6666666666666,
    0.005 = 0x3F747AE147AE147B, K = 0x3DF0000000100000, 2π/2³², π/8, etc.).

## Design

### Components (`src/map/rmg/`)
- `options.rs` — `RmgOptions` (16 fields), normalizer clamps (item 12), `.SED`
  read/write (carry-default semantics, Description hex-CSV), `RandMap.Sed`
  writer via `util/ini_writer`.
- `settings.rs` — `RmgSettings` from `RMGMD.INI` (zero defaults; loaded via
  asset manager MIX lookup).
- `rng.rs` — `RmgRng`: exact LFG state (253 dwords), seed-hash with corrected
  tables, `raw()`, `uniform(min,max)` (draw-exact incl. rejection),
  `gaussian()` (Box-Muller with cached-second-value semantics).
- `x87.rs` (or extension of `util/native_x87.rs`) — the op subset from ledger
  items 65–66.
- `scratch.rs` — `RmgScratch`: per-cell entries (coord, height f64, velocity
  f64, probs, region id, stamp, flags), corner grid, region objects, diamond
  bounds. Owns all intermediate state; no globals.
- `phases/` — one file per stage in ledger order (water.rs, water34.rs,
  regions.rs, terracing.rs, green_spread.rs, hills.rs, lat_patches.rs,
  trees.rs, rocks.rs, starts.rs, tech_buildings.rs, tiberium.rs). Each is a
  plain function `fn run(scratch, rng, opts, deps)` committing in native order.
- `emit.rs` — scratch → `MapFile`: cells via `TilesetLookup` bounds, overlay
  entries + data pack, terrain objects (trees), waypoints, `[Basic]`,
  `[Map]`/`LocalSize`, `[Lighting]` from Time/theater tables.
- `preview.rs` — generated-surface → preview RGB + `RandMap.img` write.
- `mod.rs` — `RmgGenerator::generate(opts, settings, theater, rules, preview_hook) -> GeneratedMap`.

### Interfaces / Contracts
- `GeneratedMap { map_file: MapFile, start_waypoints: Vec<(u8, CellCoord)> }`
  — consumed by the launch branch to build `MapLoadInitial`.
- Dialog state machine in `ui/skirmish_shell/state/random_map_dialog.rs`
  mirroring native control IDs/enable-disable lists; render in
  `app_skirmish_shell_render`; command `0x583` opens it (replacing the stub).
- Launch: `load_map_initial_with_assets` gains a branch — requested name
  `RandMap.Sed` (`.sed` suffix per native) ⇒ read `.SED`, `generate`,
  wrap as `MapLoadInitial`.

### Data Flow
Button → dialog (options, Randomize, Generate-with-preview) → accept writes
`RandMap.Sed` + `RandMap.img` → Choose Map sentinel selected → Start →
launch branch reads `.SED` → `generate` (no preview) → `MapLoadInitial` →
existing pipeline → playable match.

### Error Handling
Native semantics: malformed/missing `.SED` keys fall back to current field
values (carry); missing `RMGMD.INI` ⇒ zero settings (⇒ no trees — matches
native); generator failures do not exist on the native path (loops are
defensive) — any Rust-side invariant break is a hard error (anyhow) surfaced
at load, never a silent fallback map.

### Testing Strategy
- **RNG golden vectors (P0 instrument) — spike run 2026-07-20, verdict below.**
  `Random__Seed 0x0065C6D0` and `Random__Next 0x0065C780` are fully decoded to
  the instruction level and reproduced in Python straight from the
  disassembly (seed = lag-fill with the two constant tables at 0x00839644 /
  effective 0x00839694..; next = lag-103 XOR over a 250-dword buffer, returns
  the freshly-XORed `state[idx_a]`, both indices ++ wrapping at 250). Seed
  1234 → `state[0..3] = 29B13EEC D3F8C8D3 451AA367 62564DB1`, first six draws
  `74AED1F3 DCD6C7FC 983F31F3 969AF076 87D277F7 F1E4CE03`. **VERIFIED against
  gamemd 2026-07-20:** the unicorn harness (`tools/rmg_oracle/`) ran the real
  `Random__Seed`/`Random__Next` for seeds 0/1/1234/0x7FFF/0xFFFF and every one
  of 1250 state dwords and 80 chained draws matched bit-exactly (0 mismatches).
  Goldens: `tools/rmg_oracle/vectors/rng.json`.
- **Instrument verdict (RESOLVED 2026-07-20 — harness built, RNG half done):**
  `emulate_function` alone is INSUFFICIENT as the RNG/x87 golden oracle. It ran the seed fill loop to completion (compute path
  works) but (a) returns only registers, never output memory — so it cannot
  emit the seeded 1012-byte struct, and (b) faults on a bare `PUSH` at the
  initial ESP for `Random__Next`, so it cannot drive the stack-using draw or
  Box-Muller routines. The real oracle needs a full-CPU harness (unicorn-engine
  or a live-gamemd capture) that runs seed→N-draws and dumps memory. **This
  becomes P0 task #6 (revised): build that harness, then lock the Python
  vectors bit-exact.** Do NOT ship the Python vectors as certified goldens
  until the harness confirms them.
- **x87 ln (Box-Muller 0x005980C0):** same harness dependency; the `ln` via
  FYL2X is the single hardest bit-exactness risk and must be validated through
  the harness before any FP-phase implementation.
- Per-phase unit tests from ledger constants (clamps, ramp table, band
  splits, formulas), all named `rmg_*`.
- Determinism: same options ⇒ identical `MapFile` byte hash across runs.
- **Certification:** full-map byte-golden vs gamemd requires a native capture
  instrument (live gamemd cell-grid dump for a fixed `.SED`). Until that
  spike lands, full-pipeline parity is UNVERIFIED-pending-instrument;
  sub-phase certification via emulation vectors proceeds meanwhile.

## Architectural Decisions
- Follows: in-memory `MapFile` as the single map contract; app-layer modals;
  `thiserror`/`anyhow` split; no floats in `sim/` (generator is map-layer,
  pre-sim); module `//!` headers; ≤600-line files (phases split per stage).
- Deviates: introduces a second RNG implementation (`RmgRng`) beside
  `SimRng` — justified: the native map-gen LFG (with hash seeding and the
  non-exact 2⁻³² constant) is a different, byte-specified machine; sharing
  code would invite drift in both.
- Tech debt: none planned; B-style map export may be added later as a debug
  tool.

## Prerequisites (P0 — before /write-plan tasks are written against them)
1. Re-investigate tiberium §4–§6 (field-count lerp, gate set) — RED doc.
2. Re-investigate water-3/4 internals (0x0059C920/0x0059D510) + bridge-pass
   helpers + tech maptype-2 path (0x00595400).
3. Patch RNG doc (§2.2/§2.3/§3.2/§5) and water doc (§5, iteration caps) per
   audit correction facts; patch parent generator doc (stage list, theater
   clamp).
4. Extract `RMGMD.INI` from retail MIX; record actual values.
5. Live-read `g_DirectionOffsets` values.
6. **[DONE for RNG 2026-07-20; x87 outstanding]** Full-CPU golden-vector harness
   built at `tools/rmg_oracle/harness.py` (unicorn 2.1.4, maps gamemd by PE
   section headers, real stack, dumps memory) — `emulate_function` had proved
   insufficient (register-only return, unreliable stack).
   **RNG: locked.** `gen_rng_vectors.py` → `vectors/rng.json`; the Python
   reproduction matches 1250/1250 state dwords and 80/80 draws bit-exactly.
   **Still outstanding:** the same treatment for the Box-Muller helper
   `0x005980C0` and the x87 `ln` path — those remain
   UNVERIFIED-pending-instrument and block the FP-heavy terrain phases.

## Implementation Phasing (after P0)
- **P1:** options/settings/rng/x87/scratch/emit + base pipeline (map types
  0–2, all theaters): water → regions → green spread → hills → LAT/trees/
  rocks → starts → tech buildings → tiberium.
- **P2:** dialog + preview + launch wiring + sentinel capacity fix.
- **P3:** map types 3/4 (terracing, bridges, cliff drops, re-anchor) +
  maptype-2 tech path.

## Alternatives Considered
- **B — generate-to-file:** extra LZO/base64 roundtrip native doesn't have;
  kept as future debug export only.
- **C — direct-to-sim construction:** bypasses canonical `MapFile`,
  duplicates terrain resolution, kills headless tests; no parity gain.
