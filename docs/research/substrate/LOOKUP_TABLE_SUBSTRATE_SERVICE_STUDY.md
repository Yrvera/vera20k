# Lookup-Table Substrate Service — Master Study

**Date:** 2026-06-04
**Kind:** Synthesis study (research + design). **No Rust written or modified.**
**Authority order:** binary → Ghidra → docs. Every cross-family anchor re-verified live
this session is cited inline; family-doc facts are cited to the source doc.
**Burden of proof:** DRIFT by default. Equivalence is downgraded only with algebraic proof,
boundary-inclusive bit-identity, or exhaustive caller verification — same bar as the six
family docs this synthesizes. There is **no INTERNAL-ONLY escape** for the active
gameplay/render/audio/parser behavior these tables drive.

## Source family docs (read all six in full before this)

1. Facing / Direction — [`tables/FACING_DIRECTION_SUBSTRATE_STUDY.md`](tables/FACING_DIRECTION_SUBSTRATE_STUDY.md)
2. Cell-spread — [`tables/CELL_SPREAD_SUBSTRATE_STUDY.md`](tables/CELL_SPREAD_SUBSTRATE_STUDY.md)
3. Path-neighbor — [`tables/PATH_NEIGHBOR_SUBSTRATE_STUDY.md`](tables/PATH_NEIGHBOR_SUBSTRATE_STUDY.md)
4. Bridge-overlay — [`tables/BRIDGE_OVERLAY_SUBSTRATE_STUDY.md`](tables/BRIDGE_OVERLAY_SUBSTRATE_STUDY.md)
5. LandType/SpeedType/Passability — [`tables/LANDTYPE_SPEEDTYPE_PASSABILITY_SUBSTRATE_STUDY.md`](tables/LANDTYPE_SPEEDTYPE_PASSABILITY_SUBSTRATE_STUDY.md)
6. Remap/Palette/Sound — [`tables/REMAP_PALETTE_SOUND_SUBSTRATE_STUDY.md`](tables/REMAP_PALETTE_SOUND_SUBSTRATE_STUDY.md)

**Program context:** master roadmap `docs/plans/2026-05-29-core-engine-substrate-todo.md`
(item **#7 map/cell substrate** is the home for the sim cell/movement table services);
Factory+House substrate `docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
(the shadow→invert ceremony — **not** what these tables use). Translation rule throughout:
**Rust-native structure, gamemd-native semantics.**

---

## 1. The unified substrate boundary (the core claim)

**These six families are mostly CONSTANT, pure, read-only, deterministic lookup data**
(facing offsets, cell-spread spiral, neighbor offsets, passability matrix, land/speed
multipliers, overlay byte ranges, color ramps, sound indices). Their migration is therefore
**DATA-PARITY + API-CONSOLIDATION** — replace the approximated/duplicated/invented Rust
tables with **one verified gamemd-exact lookup service per layer**, behind named accessors,
proven by exact-table-equality dumps against gamemd. It is **NOT** the stateful
shadow→invert→authoritative ceremony used for Factory/Economy: there is no per-house mutable
state to shadow; a constant table is either byte-equal to gamemd or it is a bug.

**One layer is the wrong frame.** The families split cleanly across the existing layering
invariant (`sim/` never depends on `render/ui/audio/net`), so the unified boundary is
**three sibling pure services in two layers**, not one monolith:

- **`sim/` cell/movement table service** (master-TODO #7) — families 1, 2, 3, 5, and the
  bridge-overlay *classifier/range* tables of family 4. Pure const data + integer/fixed-point
  helpers; depends only on `util`. Consumed by pathfinding, locomotors, turret, combat AoE,
  zone builder, bridge damage dispatch.
- **`rules/` render/audio data service** (family 6) — the Priority→ColorScheme table +
  ColorScheme palettes (render-facing) and the VocClass table + flag tables (audio-facing).
  These are rules-parsed/embedded data + a deterministic name→index contract. **It must NOT
  become a `sim/` dependency** — sim never reads palette/sound; render/audio consume the
  rules service. (One narrow exception, §3c.)
- **`rules/` movement-data service** — the LandType×SpeedType float table + buildable bits +
  the passability matrix const (family 5). It is INI-derived + a baked const, so it lives in
  `rules/` like `TerrainRules`/`SpeedType` already do; `sim/` holds the per-cell *state* and
  *queries* the service.

The genuinely stateful concerns these tables are *consulted by* stay **out of scope** of the
table services and remain where they are: **bridge damage state** (CellClass+0x11E
transitions, collapse, repair re-stamp — `sim/bridge_state`), **cell occupancy / object
lists / reservation bits** (master-TODO #7's mutable side), and the **two RNG streams**
(master-TODO #2; the table services are RNG-free — LOW-variant/BridgeStrength callers bind
the correct instance, §4 D9). The table service is the *oracle* those stateful systems
consult; it never holds mutable state, never draws RNG, never reads a global.

**One-paragraph statement:** *Consolidate the six lookup-table families into three pure,
read-only, deterministic substrate services — one in `sim/` (cell/movement geometry + cost +
AoE-spiral + bridge classifiers), two in `rules/` (movement-data float-table/matrix;
render/audio color-scheme + Voc/flag tables) — each owning gamemd-exact const data behind
named accessors, migrated by data-parity + API-consolidation (exact-dump-equality tests, not
shadow→invert), with bridge damage state, cell occupancy, and RNG explicitly left as
out-of-scope stateful concerns the services merely feed.*

---

## 2. Module-tree placement (text diagram)

```
src/
├── util/
│   └── fixed_math.rs        // SimFixed, lepton helpers. The ONLY dep of the sim table service.
│                            //   (homing/DRAGON atan2 quarantined here, NOT in the general facing API)
│
├── sim/
│   └── world/substrate/     // master-TODO #7 "map/cell substrate" — PURE table services
│       ├── direction_tables/         // FAMILY 1 (facing/direction)
│       │   ├── mod.rs                 //   //! purpose + deps (util only)
│       │   ├── cell.rs                //   CELL_DELTAS [(i32,i32);8]  == g_DirectionOffsets (0x0089F688, dx/dy frame)
│       │   ├── lepton.rs              //   LEPTON_DELTAS == CELL_DELTAS×256 (0x0089F6D8) — currently MISSING in Rust
│       │   ├── quantize.rs            //   dir<->facing (8/16-bit), opposite, muzzle 8-way, facing8_to_16
│       │   ├── drive_track.rs         //   TurnTrack[72]/RawTrack[16]/TrackPoint (moved from sim/movement)
│       │   └── dragon.rs              //   DRAGON_FRAME_TABLE [i32;32] (0x007F4890) sim-side data
│       ├── neighbor_tables.rs         // FAMILY 3 (path-neighbor): cell_ptr_offset (512-stride 0x007e3774),
│       │                              //   closed_index_offset(width) (0x0089a304), EDGE_COST_BASE (0x0081870c),
│       │                              //   DIR_EPSILON (0x0081872c), bridge flank/ramp consts, REOPEN_TOLERANCE(f64)
│       │                              //   — re-exports Dir8 + NEIGHBOR_DELTA from direction_tables (single source)
│       ├── cell_spread.rs             // FAMILY 2: OFFSET_TABLE[369] (0x00ABD490) + COUNT_TABLE[12] (0x007ED3D0)
│       │                              //   + splash_count_index ftol(CS+0.99), splash_threshold_leptons ftol(CS*256),
│       │                              //   reveal_count_index(min(sight,10)) + reveal corner-gate
│       └── bridge_overlay_tables.rs   // FAMILY 4 (classifier/range data only):
│                                      //   LATIN_SQUARE[16] (0x0081CC30), overlay-byte band consts,
│                                      //   DESTRUCTION_OVERLAY_*[16]×4, TILESET_WINDOW, SM base-relative helpers
│                                      //   (consumes cell_spread for AoE; does NOT own damage STATE or RNG)
│
└── rules/
    ├── movement_tables.rs   // FAMILY 5: speed[12][8] SimFixed (0x0089EA40, capped 1.0), buildable[12],
    │                        //   PASSABILITY [[u8;8];13] (0x0082A594), EDGE_COST_BASE (shared value w/ #3)
    └── tables/
        ├── color_scheme_substrate.rs  // FAMILY 6a (render): PRIORITY_TO_SCHEME[9] (0x0083ed14),
        │                              //   priority_to_scheme(u32), ColorScheme{remap_palette[256]},
        │                              //   ColorSchemeTable (doubled DVC), extract_base_rgb/compute_bright_rgb
        └── voc_substrate.rs           // FAMILY 6b (audio): VocTable (Vec — index IS the domain),
                                       //   VocControl/VocType/VocPriority flag tables (0x008160c0/0x00816048/0x00816018),
                                       //   find_by_name (case-sensitive, lowest-index), read_sound_list

CONSUMERS (call into, never owned by, the services):
  sim/pathfinding/{core,path_smooth}.rs, sim/movement/{turret,bump_crush,drive,walk}.rs,
  sim/combat/{combat_aoe,mod}.rs, sim/map/bridge_topology.rs (TILESET_WINDOW), zone builder,
  render/palette_textures.rs (ColorScheme.remap_palette), render/minimap_helpers.rs (house RGB),
  audio/sfx.rs (VocEntry + CalcVolumeAndPan port)

OUT OF SCOPE (stateful — services feed them, don't own them):
  sim/bridge_state (CellClass+0x11E damage SM, collapse, repair),
  master-TODO #7 mutable occupancy/object-lists/reservation,
  master-TODO #2 two RNG streams (g_GlobalRng for LOW-variant; Scen->Random for BridgeStrength)
```

Placement rationale, per layer:
- **sim/world/substrate/** because families 1–4(classifier) carry sim-authoritative behavior
  tables (drive tracks, neighbor cost, AoE spiral, bridge classify) read inside the tick by
  pathfinding/locomotor/combat. They sit in the substrate tier next to the
  branch's existing `sim/world/substrate.rs`, `bridge_topology.rs` services.
- **rules/** for families 5 and 6 because they are INI-parsed or scheme-data-loaded plus baked
  consts, exactly like the existing `TerrainRules`/`SpeedType`/`color_scheme.rs` residents;
  keeping them in `rules/` lets headless servers and the zone builder share one authority and
  preserves "sim never depends on render/audio."

---

## 3. Cross-family reconciliation (concrete dedup proposals)

### 3a. The 8-neighbor (dx,dy) table is ONE table — collapse all Rust duplicates behind it

`g_DirectionOffsets @ 0x0089F688` (the `(i16 dx, i16 dy)[8]` cell-delta frame, compass order
N,NE,E,SE,S,SW,W,NW) is consumed by **facing** (cell step / opposite), **path-neighbor**
(A* blocker-predict + smoothing + `MapCoord_StepByDir_GetCell`'s 51 callers), **cell-spread
seed** (the R1 ring is exactly these 8), and the AI/`Mission_Hunt`-class scans. It reads as
all-zero in the static image — **runtime-filled (BSS)**, verified live this session
(`read_memory 0x0089F688` → all zeros), so its values are *constrained* to the standard
compass deltas (matched by 0x007e3774 and inverted by the 0x007e3760 delta→dir map), not
statically dumped. Mark UNVERIFIED-from-static; do not claim PROVEN.

Rust currently has **four+ independent literal copies** of this one table:
`util/direction.rs::DIRECTION_DELTAS`, `pathfinding/core.rs::NEIGHBORS`,
`movement/bump_crush.rs::NEIGHBOR_OFFSETS`, `pathfinding/path_smooth.rs::DIR_DELTAS` (alias),
plus the `fixed_math.rs::dir_to_cell_delta` forwarder.

**Proposal:** one canonical `direction_tables::cell::CELL_DELTAS` (+ `Dir8`, `is_diagonal`).
`neighbor_tables.rs` re-exports it; `bump_crush.rs`, `core.rs::NEIGHBORS`, the `path_smooth`
alias, and the `dir_to_cell_delta` forwarder all collapse to that single accessor. Values are
byte-equal across all consumers today (no current numeric drift), so this dedup is
hash-neutral; the risk it removes is a future one-sided edit. (Facing-doc §4.9, Path-doc §4.2.)

### 3b. **CRITICAL non-collapse: keep the (dx,dy) frame separate from the 512-stride index table**

The path-neighbor lane proves gamemd uses **two physically distinct tables** that LOOK
interchangeable but are NOT:
- `g_DirectionOffsets @ 0x0089F688` — the `(dx,dy)` frame (BSS).
- `g_CellNeighborOffsets_8Dir @ 0x007e3774` — `int32[8] = {-512,-511,1,513,512,511,-1,-513}`,
  the **CellClass\* pointer-array index** (512-stride ±1). Verified live this session
  (`read_memory 0x007e3774` → exactly those eight int32s).
- and a THIRD, `closed_index_offset @ 0x0089a304`, runtime-derived on the **zone-grid width**
  `DAT_0089c2dc`, not 512.

On Rust's single-width flat grid these three coincide *by construction*. **A naive
consolidation that exposes one "neighbor offset" helper is a latent DRIFT** the moment a
non-square / bordered / non-512-effective-width grid is introduced (the scale target is 20k
units / 30 players — large maps make this real). The service must expose `cell_delta(dir)`
[(dx,dy)], `cell_ptr_offset(dir)` [512-stride], and `closed_index_offset(dir, width)`
[width-stride] as **three separate functions** so they can never be silently unified.
(Path-doc §5 "Two distinct strides coexist", §4.1 representation note.)

### 3c. Bridge-overlay tables span map + pathfinding + combat — split owner by purpose

Family 4 touches three subsystems and must be partitioned, not lumped:
- **Classifier/range data** (Latin square 0x0081CC30, overlay byte bands 0x4A..0x63 /
  0xCD..0xE6, destruction next-overlay tables, TILESET_WINDOW=0x10, SM `(tile-base)+1`) →
  `sim/world/substrate/bridge_overlay_tables.rs`. Today these literals are scattered across
  THREE files (`bridge_specs.rs`, `overlay_types.rs`, `bridge_facts.rs`) with no shared
  const — the headline structural dedup (Bridge-doc §4.2/§7).
- **The CellSpread tables it consumes** (0x007ED3D0 / 0x00ABD490) are **general AoE infra,
  owned by family 2's `cell_spread.rs`**, NOT duplicated into the bridge service. Verified
  identical bytes in both docs and live this session (`read_memory 0x007ED3D0` =
  [1,9,21,37,61,89,121,161,205,253,309,369]). The bridge AoE block *imports* `cell_spread`.
- **Bridge damage STATE + RNG draws** stay in `sim/bridge_state` (out of table scope).
- **Render Latin-square jitter:** the only render consumer (`app_instances/bridges.rs`)
  imports the sim pure `bridge_body_frame`/`latin_jitter` (render→sim is allowed) instead of
  holding its own `BRIDGE_BODY_LATIN_SQUARE` copy.

### 3d. Family 6 must NOT become a sim dependency — its substrate is rules-data + name→index

Remap/palette/sound is render-output (per-frame tint, radar dots) and audio-output
(per-event SFX) plus a deterministic name→index contract (VocClass index domain). Its
substrate is the **rules-parsed/embedded data + the name→index resolution rule**,
architecturally separate from sim. The service lives in `rules/`; render and audio consume
it; **sim never reads it.** The one shared-surface hook to reconcile (not a sim dep): the
house base/bright RGB bytes (House+0x56F9..56FE) feed BOTH the unit-remap GPU path AND the
radar-dot pack path — `extract_base_rgb`/`compute_bright_rgb` must store those bytes once on
the house record so both render paths read one source, not re-derive RGB twice (Remap-doc
cross-family hook).

### 3e. Shared scalar consts to single-source

- **`EDGE_COST_BASE [1,1000,1,1,60,20,8,10000]`** appears in BOTH path-neighbor (0x0081870c)
  and LandType (same 0x0081870c) docs — it is **one table** keyed by the Can_Enter_Cell return
  code. Verified single xref = `AStar_compute_edge_cost`. One const, exported from
  `neighbor_tables.rs`, referenced by the movement-data service (LandType-doc §2/§4.4).
- **`DirectionEpsilon [0.001..0.008, tube=0.0] @ 0x0081872c`** is path-neighbor-only but is
  the float-A* tie-break the integer `DIR_TIEBREAK` approximates — owned by `neighbor_tables.rs`.
- **The facing-quantization formula `((f>>4)+1)>>1 & 7`** is shared by facing (cell step),
  combat (Fire_At 8-way muzzle), and render (DRAGON 32-way, deferred VXL bucket) — one
  `quantize::dir_from_facing8/16` feeds all three (Facing-doc cross-family hook).

---

## 4. Consolidated cross-family DRIFT ledger (top defects)

Every row is DRIFT by default (no equivalence proof). Severity = player-visibility ×
trigger-frequency; the trigger clause is mandatory. Float-bit comparison required for f32 rows.

| # | Family | Rust file:line | Current (DRIFT) | gamemd-correct (cite) | Severity + trigger-frequency |
|---|---|---|---|---|---|
| D1 | Cell-spread | `combat/cell_spread.rs:30-64` | generated sort `(d²,\|dx\|,\|dy\|,dy,dx)`; 363/369 positions differ; element-set differs R6,8,9,10,11 | embed verbatim 369-entry table (init 0x00561910) | **HIGH** — fires every match any CellSpread≥1 warhead hits ≥2 targets/ore/walls (scan order = ReceiveDamage/RNG/chain order); element-set fires when a CS≥6 SW (nuke/weather/dominator) hits a boundary cell. |
| D2 | Cell-spread | `combat_aoe.rs:104`, `mod.rs:1162` | `floor(CS)` radius | `ftol(CS+0.99)` (0x007E5160) | **HIGH** — diverges for every fractional-CellSpread warhead (stock `.5` family); CS=0.5: gamemd 9 cells, Rust 1. |
| D3 | Cell-spread | `combat_aoe.rs:96` | `floor(CS)×256` leptons | `ftol(CS×256)` (0x007E2224) | **HIGH** — collapses the fine filter to 0 for CS<1.0; fires every fractional-CS detonation (CS=0.5 → Rust rejects all incl. impact). |
| D4 | LandType | `core.rs:1263-1267` | `step_cost = base*100/terrain_cost` (terrain speed weights A*) | A* step = `EDGE_COST_BASE[code]` only; terrain speed never weights search (0x00429830, single xref 0x0081870c) | **HIGH** — every path search crossing any ≠100% cell (ore/ice/tiberium); units detour where gamemd drives straight. |
| D5 | Facing | (missing table); `facing_table.rs:88` sin/cos used for cardinal/diagonal steps | normalized diagonal ≈(181,181) | exact integer `LEPTON_DELTAS[dir]=(±256,0)/(±256,±256)` (0x0089F6D8) | **HIGH** — every tick a ground unit steps diagonally; √2 speed/path difference. |
| D6 | Path-neighbor | `core.rs:2028-2032` g/f `i32` (×1000); `:2079` h `isqrt`; reopen tol | integer A* | IEEE float32 g/f (0x0042a460); reopen tol = **f64** 1.00903 `FADD double[0x007e37c0]` | **HIGH** — every pathfind every match; integer scaling can't represent the irrational Euclidean h nor the 1.00903 double → path divergence. |
| D7 | Facing | `fixed_math.rs:280-311` f32 atan2 → `movement/mod.rs:208`, `turret.rs:64`, etc. | float atan2, "1 ULP same bucket" asserted-unproven | vehicle facing = integer table-derived; infantry WalkLoco genuinely uses atan2+ftol (must match EXACT form); homing/DRAGON atan2 | **HIGH** — facing derived every move/turret tick; float in lockstep = cross-platform desync risk at boundary buckets. |
| D8 | Remap/Sound | `audio/sfx.rs:55-97` | no LOCAL/GLOBAL/SHROUD gates, no pan, hard 0.5 & 0.05 | flag-gated viewport subtract, MinVolume floor, shroud silence, pan, real globals (0x00750ac0) | **HIGH** — every positional SFX (~75 callers); enemy sounds audible through shroud + no stereo pan, every engagement. |
| D9 | Remap/Palette | `rules/house_colors.rs:61-188` | synthesized base RGB + brightness-gradient ramps | ColorScheme remap palette (+0x04) + InitColor extract (0x50B840) + ComputeRemap (0x50BA00) | **HIGH** — every owned object's tint + every radar dot, every frame; priority outputs 11/21/29 unreachable (idx≥9→Gold), so colors land on the wrong scheme. |
| D10 | Path-neighbor | `core.rs:103,1262` | uniform `STEP_COST=1000` base + per-code factors | per-code `EDGE_COST_BASE` lookup; **code-4=60.0 has NO Rust handling**, code-1/7 unmodeled | **HIGH** — every edge; code-2 flat-vs-multiplier and missing code-4 break ratio-equivalence beyond codes 0/5/6. |
| D11 | Path-neighbor | `core.rs:371-384` | `DIR_TIEBREAK{1,5,2,6,3,7,4,8}` int + `TUBE=9` | `DIR_EPSILON{0.001..0.008}` float, tube `0.0` (0x0081872c) | **HIGH** — every neighbor every search; tube sorts wrong (last vs first) and magnitude drifts vs multiplied edges. |
| D12 | Remap/Sound | `rules/sound_ini.rs:46-178` | HashMap, case-insensitive, no index | ordered Vec, case-sensitive strcmp, stable 0-based index, "Invalid Voc" (0x007514d0) | **MEDIUM** — index-driven consumers (CreditTicks/Lightning/101 Rules fields) unreproducible; fires on credit ticks/lightning/sell every match. |
| D13 | Bridge-overlay | `combat_aoe.rs:42,:231` | threshold `level + bridge_height/2 = +2` levels (half deck) | full deck `+4` levels (`DAT_0089E864 = 4×per_level`, writer is FADD not FMUL; GetEffectiveHeight Level+4) | **MEDIUM** — every AoE on/under a high bridge with units on both layers; routes under-bridge splash to deck list and vice versa. **Stage-2 corrective value `2` is REFUTED — fix is dropping the `/2`.** |
| D14 | Bridge-overlay | `combat_aoe.rs` whole AoE path | no bridge-tile damage block; no BridgeStrength draw | gated `DestroyableBridges & Wall`; per-block `RandomRanged(1,BridgeStrength)` from **Scen->Random**, strict `<`, sequential A/B/C/D, Ion bypass+3 retries (0x489280) | **MEDIUM** — every `Wall=yes` warhead on a bridge; bridges take no AoE tile damage now, and the RNG instance must be Scenario not global or replays desync. |
| D15 | Bridge / Cell-spread / Path | `bridge_specs.rs`+`overlay_types.rs`+`bridge_facts.rs`; `bump_crush.rs:75`; `path_smooth.rs:31` | overlay band edges + neighbor deltas as inline literals in 3+ files | single named-const source per table | **MEDIUM** — every bridge-damaging hit / every bump-crush check; no current value drift but a one-site edit silently desyncs the others. |
| D16 | LandType | `core.rs:118,1270-1272` | extra `×4` on `height != neighbor_height` | ramp ×4 only on `cell+0x140 & 0x40000` marker; no height-diff ×4 in 0x00429830 | **MEDIUM** — every path near ramps/level changes (hilly/bridge maps); Rust over-charges height steps gamemd does not. |
| D17 | Path-neighbor | `core.rs:141-145` | bridge-diag 2.0/10.0/1.0 helpers unwired | apply per flank tables 0x007e3710/0x007e3730 (0x00429830) | **MEDIUM** — bridge-diagonal pathing only, but every bridge crossing there; different bridge approach routes. |
| D18 | LandType | `terrain_cost.rs:23,122-172` | `COST_ROUGH=75` + fabricated rough penalties | stock `[Rough]`=100/100/100; no rough slowdown (rulesmd.ini L30212) | **MEDIUM** — whole `TerrainCostGrid` is the retired A* weight; fabricated values diverge wherever applied. |
| D19 | Remap/Sound | `sound_ini.rs:28-43,:117` + `sfx.rs:179` | no Control/Type flags; Priority int default 1; `random_counter%len` | flag bit tables (0x008160c0/48); Priority name table default 2; RANDOM-flag-gated RNG | **MEDIUM** — RANDOM/LOOP/INTERRUPT/priority eviction unmodeled + wrong sample + no lockstep RNG; every multi-sample play. |
| D20 | Facing | `drive_track.rs:194-3393`, `:44-62` | 5 TurnTrack + 6 RawTrack spot-checked; rest + TrackPoints + transform UNCHECKED | full byte-equality vs 0x007E7A28/0x007E7B28 + Transform_Track_Coords | **MEDIUM/UNCHECKED** — drives every vehicle turn; spot-checks pass, full-table equality unproven. |
| D21 | Facing | `facing_class.rs:85-116` timer interpolator for general body/turret turn | timer model | gamemd general body turn = drive-track tables; trio is homing-only; per-frame ClampToROT sequence unproven-equal | **MEDIUM/UNCHECKED** — turret tracking visible every combat tick; per-frame sequence not proven bit-identical. |
| D22 | LandType | `passability.rs:40-179` | 8-bucket LandType + `zone_layer_for_speed_type` + `is_passable_for_speed_type` | matrix keyed by MovementZone only; 12-row LandType; no SpeedType→row table | **MEDIUM (structural)** — port-invented remap, unproven across inputs; fires on fallback paths without INI speed data. |
| D23 | Facing | `app_fire_effects.rs` DRAGON frame; missing 0x007F4890 | origin→target cell-delta formula | `(28-i)&31` lookup over `bam=ftol((atan2(-VelY,VelX)-π/2)×(-32768/π))` | **MEDIUM** — every `Rotates=yes`/DRAGON projectile in flight (e.g. Aegis AA); wrong projectile sprite frame. |
| D24 | Bridge-overlay | `bridge_specs.rs:154-315` | partial LOW selector; no mask decode, no RNG instance, no coord-delta | full mask + **single** `g_GlobalRng` draw (NOT two — corrected) + coord-delta stamp (0x579620) | **LOW now / HIGH when wired** — not on a live path yet; player-visible (wrong connector tile) + lockstep-critical the moment wood-bridge repair lands. |
| D25 | Facing / Cell-spread | `direction.rs:77-92` masked accessors; `combat_aoe.rs:91` CS=0 early-return | `&7`/`None`; CS=0 → zero AoE | gamemd unmasked OOB read; CS=0 scans 1 cell (impact) via table | **LOW/UNCHECKED** — only differs if `dir>8` passed (callers sanitize) / CS=0 net depends on the separate direct-hit path. |
| D26 | Remap/Palette | `color_scheme.rs:39-47` | `priority.max(0)` clamp; i32 sentinel | uint; p≥9 returns p unchanged; `-2`==0xFFFFFFFE (0x0069A310) | **LOW** — only diverges on negative-non-`-2` priorities, which stock lobby slots don't produce. |

**Defects that survived adversarial re-check in all six docs and are NOT to be "fixed" with
the stage-2 value:** D13 (use **4 levels / drop the `/2`**, not the refuted constant `2`),
D24 (**one** g_GlobalRng draw, not two). Also note D2's count fix and D3's threshold fix are
*both* required — for CS=0.5 the damage output may coincide on the `combat_aoe` path (ring
cells exceed 128 leptons) but the **ore path has no lepton filter and WILL under-scan** —
fix the rule regardless of the sampled coincidence (Cell-spread §4c).

---

## 5. Consolidated migration roadmap (ordered slices across families)

**Ordering principle:** *pure-data const-embed + exact-dump-equality FIRST (hash-neutral,
independently shippable), then re-point consumers, then the few genuinely behavior-changing
or stateful slices LAST behind lockstep state-hash guards.* Each slice maps to the substrate
program's "foundational helper-service slice first, behavior re-pointing after" convention.
Pure-data slices need NO shadow→invert ceremony (constant tables); only the stateful slices
(S-marked) touch hashed mutable state and get the lockstep guard.

| Slice | Type | Family | What | Depends on | Acceptance (exact-dump-equality, boundary-inclusive) |
|---|---|---|---|---|---|
| **U1** | P | 1 | `direction_tables/cell.rs` `CELL_DELTAS`+`Dir8`+accessors; `lepton.rs` `LEPTON_DELTAS`=×256 | — | `CELL_DELTAS==g_DirectionOffsets` constrained deltas; `LEPTON_DELTAS[1]==(256,-256)` NOT (181,181); `opposite_dir==(d±4)&7` |
| **U2** | P | 1 | `quantize.rs` (dir↔facing 8/16, opposite, muzzle 8-way, facing8_to_16) | U1 | `dir_from_facing8` == `((f>>4)+1)>>1&7` == `(f+16)/32&7` for ALL 256; 16-bit ignores low byte |
| **U3** | P | 1 | `drive_track.rs` move + **full** TurnTrack[72]/RawTrack[16]/TrackPoint byte-equality + transform | U1 | all 72/16 rows + ~492 points == `read_memory 0x007E7A28/0x007E7B28` + ptrs; `transform_track_point` vs Transform_Track_Coords (BLOCKS slice) — closes D20 |
| **U4** | P | 1 | `dragon.rs` `DRAGON_FRAME_TABLE`+`dragon_frame_index`; `muzzle_anim_index_8way` | U2 | table == `(28-i)&31` == dump 0x007F4890; muzzle `+1` rotation |
| **U5** | P | 2 | `cell_spread.rs` `OFFSET_TABLE[369]`+`COUNT_TABLE[12]` const-embed (R6/R11 defects verbatim) | — | count==[1,9,21,…,369]; idx0==(0,0); R1 sweep exact; R11 dup preserved (regression guard) |
| **U6** | P | 2 | splash `ftol(CS+0.99)` index + `ftol(CS×256)` threshold + `air_flag(CS>0.5)` helpers | U5 | 0.0→1,0.5→9,1.5→21; threshold 0.5→128,1.0→256; air 0.5→false (strict `>`) |
| **U7** | P | 3 | `neighbor_tables.rs` geometry primitives; **3 separate index spaces** (dx/dy, 512-stride, width); re-export `Dir8`/`CELL_DELTAS` from U1 (kills `bump_crush`/`path_smooth`/core dups) | U1 | `cell_ptr_offset`=={-512,…,-513} (0x007e3774); `closed_index_offset(dir,w)` for w∈{1,64,512,65535}; delta→dir + recon table exact — closes D15(neighbor) |
| **U8** | P | 3 | cost consts: `EDGE_COST_BASE`(shared), `DIR_EPSILON`, bridge flank/ramp, `REOPEN_TOLERANCE:f64` | U7 | f32-bit-equal 0x0081870c/0x0081872c/flank tables; reopen tol == double 0x3ff024dd2f1a9fbe |
| **U9** | P | 5 | `rules/movement_tables.rs`: `PASSABILITY[[u8;8];13]` + `EDGE_COST_BASE` (= U8's) consts; matrix off `passability.rs` | U8 | matrix bit-identical 104 cells (col7=3, row12==row2); EDGE_COST exact 8 codes |
| **U10** | P | 5 | INI-parse `speed[12][8]` SimFixed (cap 1.0 at parse, Winged=1.0, 12 fixed-order sections) + `buildable[12]` | U9 | per §2 table: Tiberium×Track=0.70, Water×Track=0.0; cap 150%→1.0; missing→1.0 |
| **U11** | P | 4 | `bridge_overlay_tables.rs`: LATIN_SQUARE, band consts, DESTRUCTION_OVERLAY_*×4, TILESET_WINDOW, SM helpers; **consume U5 cell_spread** | U5 | latin==dump 0x0081CC30; `classify_overlay_byte` full 0..255; destruction full 0..15 (licenses §4.3 downgrade) |
| **U12** | P | 6a | `rules/tables/color_scheme_substrate.rs`: `PRIORITY_TO_SCHEME[9]`+`priority_to_scheme(u32)`; ColorSchemeTable (doubled) + exact `find_by_name` | — | priority_to_scheme: 0..8→{3,11,…,5}, 9→9, 0xFFFFFFFE→0xFFFFFFFF, 0x80000000 passthrough (closes D26) |
| **U13** | P | 6b | `rules/tables/voc_substrate.rs`: VocTable (Vec), 3 flag tables, `find_by_name`(case-sensitive lowest-index), `read_sound_list` | — | control/type/priority bits exact (0x008160c0/48/18); first-match lowest index; case-sensitive miss |
| **U14** | P | 1/3/4 | **Re-point** pathfinding/bump/smoothing/render-latin to the services; delete all dup tables/aliases/forwarders | U1,U7,U11 | A* neighbor coords == `CELL_DELTAS`; render frame == `bridge_body_frame`; no remaining dup (review) — closes D15 fully |
| **U15** | P/S | 2 | **Re-point** splash + ore consumers to U6 helpers (fix floor→ftol, threshold) | U6 | CS=0.5 ore → 9 reduction reqs (was 1); CS=1.5 → 21 cells/384 lep; damage-list order == embedded scan order — closes D1/D2/D3 |
| **U16** | P | 7 | `bridge_topology.rs` imports `TILESET_WINDOW`; `movement_tables` API replaces `is_passable_for_speed_type`/`zone_layer_for_speed_type`; RecalcZoneType impassable tests via `is_terrain_passable(Wheel)` | U10,U11 | tileset predicates unchanged; recalc-zonetype wheel thresholds (overlay `==0`, base `<=0.01`) — closes D22 |
| **U17** | S | 5 | **A* edge cost keyed by return code only** (drop `base*100/terrain_cost` + height ×4); `EDGE_COST_BASE[code]`; delete `TerrainCostGrid` as A* weight | U9,U14 | `astar_ignores_terrain_speed` (Tiberium route == Clear route); per-code edge-cost table; ramp == flat clear. **Lockstep state-hash guard.** — closes D4/D16/D18 |
| **U18** | S | 3 | Float A* g/f storage (`f32`); `DIR_EPSILON` tie-break; `REOPEN_TOLERANCE` f64 x87-promoted; per-code base table; code-2 flat; ramp marker-only; bridge-diag flank costs | U8,U17 | g==1.001f32; euclidean h matches sqrt-approx 0x004cac40 (gate); bridge-diag 2.0/10.0/1.0. **Lockstep guard; bridge-free replay byte-identical.** — closes D6/D10/D11/D17 |
| **U19** | S | 1 | FacingClass/turret turn parity (per-frame ClampToROT); quarantine homing/infantry/DRAGON atan2 to one gamemd-exact function; remove float facing from vehicle move path | U2,U4 | `clamp_to_rot_per_frame` vs emulated sequence (0x8000 tiebreak, within-ROT snap); cardinal step +256 leptons; homing yaw matches atan2+ftol form — closes D5/D7/D21/D23 |
| **U20** | S | 4 | BridgeStrength gate + **Scen->Random** binding on live AoE; full LOW selector (mask + **single** g_GlobalRng draw); AoE deck-height **full-deck (4 levels, drop `/2`)** | U11,U15 | strict `<` (equality fails); sequential A/B/C/D non-exclusive; Ion bypass+3 retries; RNG-instance is Scenario (AT) / g_GlobalRng for LOW (AT). **Lockstep guard.** — closes D13/D14/D24 |
| **U21** | S | 6 | RulesClass 101 sound fields + 3 DVCs via `find_by_name`/`read_sound_list` (keep-previous-on-fail); InitColor/ComputeRemap house RGB; flag-aware CalcVolumeAndPan + pan + shroud/LOCAL/GLOBAL gates; RANDOM-gated RNG sample pick | U12,U13 | keep-previous-on-fail; bright (0,0,0)→(255,255,255); shroud gate silences; pan sign from signed X; RANDOM consumes lockstep RNG — closes D8/D9/D12/D19 |

**Independently shippable:** U1–U13 are pure const-embed/parser slices with no behavior
change (hash-neutral) and ship in any order after their listed dep. U14–U16 re-point
consumers to already-proven data. U17–U21 are the behavior-changing/stateful tail, each gated
behind a lockstep state-hash guard and exact-output tests. The dependency spine is
U1→U7→U14 (geometry), U5→U6→U15 (AoE), U8→U9/U10→U17→U18 (cost model), U11→U20 (bridge).

### Global parity-test harness strategy

- **Reference dumps come from gamemd via Ghidra MCP, embedded into tests as the oracle.**
  Static tables: `read_memory` of the address (0x007e3774, 0x0081870c, 0x0081872c, 0x007ED3D0,
  0x0082A594, 0x0081CC30, 0x007F4890, 0x0083ed14, 0x008160c0/48/18, drive-track 0x007E7A28/B28).
  Each test asserts the Rust const equals the **dump bytes**, not a hand-copied literal, to
  catch self-consistent drift. **BSS/runtime-filled tables** (0x0089F688, 0x0089F6D8, 0x0089EA40,
  the theater bridge bases, g_DD_*) are zero in the static image — their values come from the
  **initializer routine's instruction stream** (e.g. 0x0049F2F0/0x00561910/0x00674000) or a
  live-debugger capture post-map-load; mark those UNVERIFIED-from-static and pin via the init
  decode, never claim a static dump.
- **Boundary coverage is mandatory** per the rigor bar: full input space where small (all 256
  facing bytes, all 8 dirs incl. tube sentinel + OOB, all 0..15 destruction indices, full 0..255
  overlay byte space, all 104 matrix cells, all 32 DRAGON entries); boundary samples where large
  (CS at 0.0/0.5/1.0/1.5/2.0/10.0/10.01; equal-distance 0x8000 facing arcs; zone-width 1/64/512/65535).
- **Stateful slices add a lockstep guard:** a fixed-seed skirmish replay before/after, asserting
  the per-tick state hash is identical OR the diff is fully explained by the corrected
  rule (e.g. the float-A* tie-break). Bridge slices run a bridge-free replay that MUST be
  byte-identical. This is where the two-RNG-stream contract (master-TODO #2) is co-verified for
  D14/D24.
- **Cross-family shared-table tests:** `splash_cells(3.0)` and `reveal_cells(3)` return the
  same offset-table slice (one shared table); A* neighbor expansion produces the same 8 coords
  as `CELL_DELTAS` (one shared (dx,dy) table); the three index spaces (U7) stay separate under
  a non-512-width fixture.

---

## 6. Unified API sketch (the three services)

```rust
// ── sim/world/substrate/direction_tables (FAMILY 1; util only) ───────────────
pub enum Dir8 { N, NE, E, SE, S, SW, W, NW }           // ordinal == gamemd dir 0..7
pub const CELL_DELTAS: [(i32,i32);8];                   // g_DirectionOffsets (dx,dy)
pub const LEPTON_DELTAS: [(i32,i32);8];                 // = CELL_DELTAS×256 (currently MISSING)
pub fn cell_delta(dir:u8)->Option<(i32,i32)>;           // None for dir>7 (checked)
pub fn lepton_delta(dir:u8)->Option<(i32,i32)>;
pub fn dir_from_facing8(f:u8)->u8;                      // ((f>>4)+1)>>1 & 7
pub fn dir_from_facing16(f:u16)->u8;                    // ((f>>12)+1)>>1 & 7
pub fn opposite_dir(dir:u8)->u8;                        // (dir-4)&7
pub fn muzzle_anim_index_8way(f16:u16)->u8;            // (dir_from_facing16+1)&7
pub const TURN_TRACKS:[TurnTrack;72]; pub const RAW_TRACKS:[RawTrack;16];
pub fn select_turn_track(from:u8,to:u8)->u8;            // to + from*8, fallback from*9
pub const DRAGON_FRAME_TABLE:[i32;32];                 // (28-i)&31
// (homing/infantry/DRAGON atan2 quarantined in util/fixed_math, NOT in this API)

// ── sim/world/substrate/neighbor_tables (FAMILY 3; util only) ────────────────
pub fn cell_ptr_offset(dir:Dir8)->i32;                 // 0x007e3774 (512-stride ±1)
pub fn closed_index_offset(dir:Dir8, zone_grid_width:u32)->i32; // 0x0089a304 derivation
pub const EDGE_COST_BASE:[f32;8];                      // {1,1000,1,1,60,20,8,10000} (shared w/ FAMILY 5)
pub const DIR_EPSILON:[f32;9];                         // {0.001..0.008, tube=0.0}
pub const BRIDGE_DIAG_BOTH:f32; pub const BRIDGE_DIAG_NEITHER:f32; pub const BRIDGE_DIAG_ONE:f32;
pub const RAMP_MULT:f32;                               // 4.0 (0x40000 marker, NOT height-diff)
pub const REOPEN_TOLERANCE:f64;                        // 1.00903 (FADD double, x87-promoted)

// ── sim/world/substrate/cell_spread (FAMILY 2; util only) ────────────────────
pub fn count_table()->&'static [u32;12];               // 0x007ED3D0
pub fn offset_table()->&'static [(i16,i16);369];       // 0x00ABD490 exact order (R6/R11 verbatim)
pub fn splash_cells(cs:SimFixed)->&'static [(i16,i16)]; // count[ftol(cs+0.99)]
pub fn splash_threshold_leptons(cs:SimFixed)->i64;     // ftol(cs*256)
pub fn splash_air_flag(cs:SimFixed)->bool;             // cs > 0.5
pub fn reveal_cells(sight:u32)->&'static [(i16,i16)];  // min(sight,10); caller adds corner gate

// ── sim/world/substrate/bridge_overlay_tables (FAMILY 4 classifier; consumes cell_spread) ─
pub fn latin_jitter(x:u16,y:u16)->u8;                  // 0x0081CC30, idx ((y&3)<<2)|(x&3)
pub fn bridge_body_frame(state:u8,x:u16,y:u16)->u8;    // state 0|9 -> +jitter
pub fn classify_overlay_byte(b:u8)->BridgeOverlayClass; // 0x4A..0x63 LowBody, 0xCD..0xE6 HighBody, finals
pub const TILESET_WINDOW:i32;                          // 0x10
pub fn destruction_overlay(check:u8, axis:Axis, high:bool)->Option<u8>; // 0xFF/>=16 -> None
pub fn low_destroyed_tile(mask:u8, roll:u32)->LowVariantResult; // ONE g_GlobalRng roll (caller binds)

// ── rules/movement_tables (FAMILY 5; rules/ipnarser + baked) ─────────────────
pub fn is_terrain_passable(&self, land:LandType12, sp:SpeedType)->bool; // exact ==0.0 gate
pub fn speed_multiplier(&self, land:LandType12, sp:SpeedType)->SimFixed; // capped 0..=1.0
pub const PASSABILITY:[[u8;8];13];                     // 0x0082A594, ==1 passable
pub fn edge_base_cost(code:u8)->SimFixed;              // EDGE_COST_BASE keyed by Can_Enter_Cell code

// ── rules/tables/color_scheme_substrate (FAMILY 6a; render-facing) ───────────
pub const PRIORITY_TO_SCHEME:[u8;9]=[3,11,21,29,13,25,17,15,5]; // 0x0083ed14
pub fn priority_to_scheme(p:u32)->u32;                 // uint; p>=9 (≠0xFFFFFFFE) returns p
pub fn extract_base_rgb(t:&ColorSchemeTable, idx:i32, dd:&DdFormat)->[u8;3]; // House+0x56F9..
pub fn compute_bright_rgb(base:[u8;3], low_cutoff:u8)->[u8;3];               // House+0x56FC..

// ── rules/tables/voc_substrate (FAMILY 6b; audio-facing + deterministic index) ─
pub struct VocTable { /* Vec — index IS the domain */ }
pub fn find_by_name(&self, name:&str)->i32;            // case-sensitive, lowest-index, "Invalid Voc", else -1
pub fn read_sound_list(&self, raw:&str)->Vec<i32>;     // strtok, skip-NULL, INI order
pub fn parse_control(t:&str)->VocControl;              // OR, unknown noop (0x008160c0)
pub fn parse_type(t:&str)->VocType;                    // exclusion 0x60/0xc00 last-wins (0x00816048)
pub fn parse_priority(t:&str)->VocPriority;            // unknown -> Normal(2) (0x00816018)
```

Determinism guarantees (all three services): no `f32/f64` in the **sim integer-table paths**
(cell/lepton step, quantization, drive-track select, AoE index, bridge classify all integer/
SimFixed); the path-neighbor cost service is float A* by gamemd contract (f32 g/f, f64 reopen
tol) and is the one place float is *required* for parity, validated by the lockstep guard;
family 6 presentation math (mixer, GPU remap) is float per gamemd and lives in render/audio,
not the rules service. No interior mutability, no globals, no RNG inside any service.

---

## 7. Open questions needing a USER decision

1. **Float A* migration (U18) is the single highest-leverage, highest-risk parity move.**
   Converting integer-scaled g/f to gamemd's float32 (with the f64 1.00903 reopen tolerance in
   x87-promoted precision) changes chosen paths wherever integer scaling diverged, and the
   Euclidean-h sqrt-approx (0x004cac40) is not yet decoded — if it cannot be made bit-exact,
   U18 ships a documented residual h-DRIFT. **Decision:** do we commit to full float A* now
   (matching gamemd, risking a lockstep-hash churn that must be re-baselined), or defer it and
   ship only the hash-neutral data slices (U1–U16) first? (Affects D6/D10/D11/D17.)

2. **The two stateful corrections whose stage-2 values were REFUTED** — confirm the corrected
   targets before any cutover: D13 bridge AoE deck-height = **drop the `/2`, compare full deck
   (4 levels)**, NOT set the constant to 2 (the `bridge_topology::BRIDGE_DECK_HEIGHT_LEVELS=2`
   shadow encodes the refuted premise and must be re-audited); D24 LOW-variant selector draws
   `g_GlobalRng` **once**, not twice (a two-draw port desyncs replays). User sign-off that these
   corrected values are the cutover targets.

3. **`g_DirectionOffsets` (0x0089F688) values are UNVERIFIED-from-static (BSS).** The (dx,dy)
   table is constrained to standard compass deltas by two anchors but not bit-dumped. Accept the
   constrained values as the embedded const (with the UNVERIFIED note), or schedule a
   live-debugger capture post-map-load before U1 claims PROVEN? Same question for the lepton
   table (0x0089F6D8) and the theater-loaded bridge bases / g_DD_* (family 4/6 stateful slices).

4. **Bounded follow-up Ghidra read pass for family 6 exact-number slices.** U21's exact tests
   are gated on globals not yet read (`_DAT_007e5168`/`_DAT_007e8ae8`/`_DAT_007e1748` for
   CalcVolumeAndPan; `_DAT_007e5f78`/`_DAT_007eaa50` for ComputeRemap; sound default globals
   0x008464b4/c0/b8/c4; Voc delim 0x00846570; the 101 RulesClass sound fields via 0x006691e0).
   Run that bounded read pass before U21, or ship U21's structural parts and defer the
   exact-number assertions? (The pure structural slices U12/U13 are unblocked now.)
```

---

## Anchors re-verified live this session (cross-family)

| Address | `read_memory` result | Confirms |
|---|---|---|
| `0x007e3774` | int32[8] {-512,-511,1,513,512,511,-1,-513} | 512-stride table is DISTINCT from g_DirectionOffsets (§3b) |
| `0x0089F688` | all zeros (BSS) | g_DirectionOffsets runtime-filled; values UNVERIFIED-from-static (OQ-3) |
| `0x0089F6D8` | all zeros (BSS) | lepton table runtime-filled (= cell×256) |
| `0x007ED3D0` | [1,9,21,37,61,89,121,161,205,253,309,369] | CellSpread count table shared by family 2 + 4 (§3c) |
| `0x0082A594` | row0 {1,2,2,2,2,2,2,3}, row1 {1,1,2,2,2,2,2,3} | passability matrix (family 5) |
| `0x0083ed14` | {3,11,21,29,13,25,17,15,5} pad 0xFFFFFFFF | priority→scheme + default (family 6) |
