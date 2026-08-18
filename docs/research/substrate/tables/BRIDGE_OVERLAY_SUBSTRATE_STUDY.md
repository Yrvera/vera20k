# Bridge-Overlay Tables — Substrate Study

**Family:** Bridge-overlay tables (static frame-jitter table, runtime tileset-base globals,
overlay-byte dispatch ranges, tile-set membership sets, LOW tile-variant selectors, CellSpread AoE infra).
**Date:** 2026-06-04
**Scope:** Pure/read-only table + classifier data and their consumers. The mutable bridge
*damage state machine* (CellClass+0x11E transitions, collapse propagation, repair re-stamp) is
explicitly OUT of pure-table scope and only referenced where a table is consumed by it.
**Stage:** 3 (adversarial re-verification + in-place correction; see Verification Log at end). Stage-1
decode (decompile/read_memory facts) is integrated verbatim where load-bearing; every binary fact carries
an inline Ghidra MCP call citation. **Stage-3 corrected 2 WRONG load-bearing claims: the AoE deck-height
identity (§4.8, writer is `FADD`/`4 × per_level`, NOT `2 ×` — corrective value `2` REFUTED) and the LOW
selector RNG draw count (§4.4, ONE g_GlobalRng draw, not two).**
**Translation rule:** Rust-native structure, gamemd-native semantics. No literal C++ class port; reproduce
the verified behavior contract (ordering, indexing, RNG instance, units, range boundaries).
**Confidence:** per-claim, stated inline. Default verdict on any unproven Rust-vs-gamemd difference is **DRIFT**.

---

## 1. Active-YR responsibilities

The "bridge-overlay tables" family is the set of static/runtime tables and helper functions that drive
three player-visible bridge behaviors in a stock YR skirmish:

1. **Per-frame bridge body/shadow tile selection.** The only *static-data* table in the family that
   touches drawing is `g_LatinSquare @ 0x0081CC30`. For a bridge body cell, the visible SHP frame is the
   cell `bridge_damage_state (+0x11E)` directly, EXCEPT for boundary states 0 and 9, which add a per-cell
   jitter from a 4×4 Latin square (indexed by the low 2 bits of cell x,y). This produces the visible
   4-way variety in healthy bridge decks. (decompile_function 0x47F6A0.)
2. **Bridge damage/destruction dispatch by overlay byte and tile-index set membership.** When a `Wall=yes`
   warhead detonates with `DestroyableBridges` on, the AoE routine and `ApplyDamageToCell` use overlay-byte
   ranges (LOW `0x4A..=0x63`, HIGH `0xCD..=0xE6`) and tile-index set membership (relative to runtime
   `g_BridgeSet`/`g_WoodBridgeSet` bases + the `DAT_00abad30`/`DAT_00aa1028` bridgehead-class sets) to pick
   which destruction path runs. Player-visible: the right bridge collapses, the right deck tiles change to
   damaged/destroyed art. (disassemble_function 0x489280; decompile_function 0x587180.)
3. **LOW-bridge tile-variant selection at repair/edge events.** `SelectBridgeTileVariant_Low @ 0x57ACF0`
   (healthy) and `SelectDestroyedBridgeTile_Low @ 0x579620` (destroyed) compute the next wood-bridge tile
   via an adjacency bitmask + PRNG, then stamp it with a coordinate delta from runtime coord-delta tables
   (`0xABDB64` healthy, `0xABDDA4` destroyed). No HIGH equivalent exists. (decompile_function 0x579620;
   get_function_callers 0x57acf0 / 0x579620 = `MarkBridgesForRepair_*` only.)

The CellSpread cell-count table `DAT_007ED3D0` and offset table `DAT_00ABD490` are *shared* AoE
infrastructure (not bridge-specific) but live inside the same dispatch routine and govern which cells a
bridge-damaging splash touches. (disassemble_function 0x489280 @ 0x4895A3, 0x4895C7.)

---

## 2. Full inventory

### 2.1 Static-data tables (non-zero in the binary image)

| Symbol | Address | Shape | Dump / facts | Consumer | Citation |
|---|---|---|---|---|---|
| `g_LatinSquare` | `0x0081CC30` | 16 × i32 (dword stride) | `{0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2}` | `DrawOverlay_Body` | read_memory 0x0081CC30 len 64; decompile_function 0x47F6A0 (`*4` stride confirmed) |
| `DAT_007ED3D0` (CellSpread count) | `0x007ED3D0` | i32[] cumulative | first 12 dwords `1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309(0x135), 369(0x171)` | `Apply_area_damage` per-spread loop bound | read_memory 0x007ED3D0 len 64; disassemble_function 0x489280 @ 0x4895A3 / 0x4899C7 |
| `DAT_00ABD490` (CellSpread offset) | `0x00ABD490` | (i16 dx, i16 dy) per entry | offset added to impact cell x/y per CellSpread cell | `Apply_area_damage` per-cell offset | disassemble_function 0x489280 @ 0x4895C7 / 0x4895CF |

`g_LatinSquare`: indexed `[((cell+0x26 & 3) << 2) | (cell+0x24 & 3)]`, value 0..3, then `*4` for the dword
stride. `DAT_007ED3D0` and `DAT_00ABD490` are **shared AoE infra**, not bridge-only.

### 2.2 Runtime-populated globals (static-zero in image; filled at theater/startup load)

These read as all-zero in the static image **only because the theater/IsoTileType loader fills them at map
load**, not because they are dead. Their relative offsets/strides are the contract; their absolute values
require a live-debugger capture post-map-load (see §5 Open/UNCHECKED).

| Symbol | Address | Size/shape | Role | Citation |
|---|---|---|---|---|
| `g_BridgeSet` | `0x00AA0E28` | 16 (HIGH/concrete tile-set base) | `(cell+0x38 − g_BridgeSet)+1` high SM classifier | read_memory 0x00AA0E28 len 64 = zero; disassemble_function 0x489280 @ 0x489ECC (`MOV EBX,[0x00aa0e28]; SUB ESI,EBX; INC ESI`); decompile_function 0x587180 |
| `g_WoodBridgeSet` | `0x00ABAD1C` | 16 (LOW/wood tile-set base) | `(cell+0x38 − g_WoodBridgeSet)+1` low SM classifier | read_memory 0x00ABAD1C; disassemble_function 0x489280 @ 0x48A0A8 (`MOV ESI,[0x00abad1c]; SUB EAX,ESI; INC EAX`); decompile_function 0x587180 |
| `g_OverlayTypeClass_Array` | `0x00A83D84` | 256 × ptr | overlay byte `cell+0x44` → OverlayTypeClass; vtable+0x9C = SHP | read_memory 0x00A83D84; disassemble_function 0x489280 @ 0x48961A (`MOV ECX,[0x00a83d84]`); decompile_function 0x47F6A0 |
| `g_DirectionOffsets` | `0x0089F688` | 8 × (i16 dx, i16 dy) | bridge walkers / ramp updaters | read_memory 0x0089F688 |
| `DAT_00abad30` (NS bridgehead class base) | `0x00ABAD30` | 4 consecutive (+0..+3) | bridgehead-class membership test (high SM) | disassemble_function 0x489280 @ 0x489F27 (`MOV EBX,[0x00abad30]`); decompile_function 0x587180 |
| `DAT_00aa1028` (EW bridgehead class base) | `0x00AA1028` | 4 consecutive (+0..+3) | bridgehead-class membership test | disassemble_function 0x489280 @ 0x489F46 (`MOV EDX,[0x00aa1028]`); decompile_function 0x587180 |
| `DAT_00ABC210` (concrete railings) | `0x00ABC210` | 10 × 16 bytes = `{shp_frame+1, surface_ptr, dx, dy}` | railing emit | read_memory 0x00ABC210 len 160 = zero |
| `DAT_00ABC2D0` (shadow-caster railings) | `0x00ABC2D0` | 5 × 16 bytes | railing emit | read_memory 0x00ABC2D0 len 80 = zero |
| `DAT_00ABDB64` (LOW healthy coord-delta) | `0x00ABDB64` | (i16 dx, i16 dy), stride 4 | LOW healthy variant stamp offset | read_memory 0x00ABDB64 len 64 = zero |
| `DAT_00ABDDA4` (LOW destroyed coord-delta) | `0x00ABDDA4` | (i16 dx, i16 dy), stride 4 | LOW destroyed variant stamp offset | read_memory 0x00ABDDA4 len 64 = zero; decompile_function 0x579620 (`&DAT_00abdda4 + iVar6*4` dx, `&DAT_00abdda6 + iVar6*4` dy) |

Tile-set range globals read by `FUN_004863D0` (all runtime-populated; sizes verified
decompile_function 0x004863D0): `DAT_00aa1020` (ramp set, 0x28=40), `DAT_00aa073c` (4), `DAT_00abb110` (4),
`DAT_00aa1050` (4), `DAT_00aa10a0` (4), `DAT_00abbebc` (0x14=20), `DAT_00abad24` (4), g_BridgeSet (0x10),
g_WoodBridgeSet (0x10), `DAT_00abc2c8` (2), `DAT_00aa101c` (0x1c=28).

Rules pointers read in the AoE block: `Rules+0x1740` (`BridgeStrength`, default 1500), `Rules+0xFF0`
(`IonCannonWarhead` ptr), `Rules+0xFA8` (`C4Warhead` ptr). (disassemble_function 0x489280 @ 0x489FE0,
0x489FD8, 0x48A363.)

### 2.3 Selector / classifier / consumer functions (live-verified)

| Function | Address | Role | Citation |
|---|---|---|---|
| `DrawOverlay_Body` | `0x47F6A0` | per-frame body SHP draw + Latin-square jitter; early-outs overlay `0xA7`/`0xB2`; HasBridge branch gated `cell+0x140 & 0x80` | decompile_function 0x47F6A0 |
| `ApplyDamageToCell` | `0x587180` | inner overlay-first damage dispatcher | decompile_function 0x587180; get_function_callers 0x587180 = `Apply_area_damage`, `FUN_006e0490`, `FUN_006e2050`, `0x574000`, `0x574c20` |
| `Apply_area_damage` | `0x489280` | outer AoE: layer selector + CellSpread loop + 4 sequential bridge blocks A/B/C/D | disassemble_function 0x489280 |
| `FUN_004863D0` | `0x004863D0` | `(cell+0x38 tile_index → bool)` membership over 11 theater tile-set ranges. NOT an overlay classifier | decompile_function 0x004863D0 |
| `HasBridgeOverlay` (MISNAMED) | `0x4865D0` | tests `cell+0x38` (tile_index), not `cell+0x44` overlay byte | decompile_function 0x004865D0 |
| `SelectBridgeTileVariant_Low` | `0x57ACF0` | LOW healthy variant chooser | get_function_callers 0x57acf0 = `MarkBridgesForRepair_High @ 0x57a0c0` only |
| `SelectDestroyedBridgeTile_Low` | `0x579620` | LOW destroyed variant chooser; inline mask+PRNG, NOT a lookup table | decompile_function 0x579620; get_function_callers 0x579620 = `MarkBridgesForRepair_Low @ 0x578e60` only |
| `UpdateBridgeTile_Low` | `0x57A430` | LOW tile updater (search_functions confirms only `_Low`) | search_functions |
| `FUN_00598030` (Rand_in_range) | `0x598030` | rejection-sample range RNG from `g_GlobalRng @ 0x00ABE890` | decompile_function 0x598030 + disassemble_function 0x598030 (`MOV ECX,0xabe890` @ 0x59805E) |

**No single-symbol classifier / display table exists.** search_functions: `SelectBridgeTileVariant` → only
`_Low`; `SelectDestroyedBridge` → only `_Low`; `UpdateBridgeTile` → only `_Low`. No `_High` runtime tile
selector. The alleged static "next-overlay tables" at `0x57E7A0`/`0x57ED00`/`0x57DD50`/`0x57E2A0` are
**function prologues, not data**: read_memory 0x57E7A0 and 0x57DD50 both = `81 EC CC 00 00 00`
(`SUB ESP,0xCC`). They are `ApplyBridgeDestruction_*` bodies; the next-overlay values come from inline
16-entry local arrays inside those functions, not standalone globals.

**No vtable/COM slots in this family** beyond `OverlayTypeClass.vtable[0x9C]` (Get_Image_Data, the SHP
resolver) used by `DrawOverlay_Body`.

---

## 3. Active vs legacy/dormant TS split

| Item | Verdict | Trigger / reachability |
|---|---|---|
| `g_LatinSquare` | **ACTIVE** | consumed every frame by `DrawOverlay_Body` boundary states 0/9; visible (deck variety); no flag gate |
| `g_BridgeSet`, `g_WoodBridgeSet`, bridgehead-class bases, coord-delta tables, railing tables, `g_OverlayTypeClass_Array`, `g_DirectionOffsets` | **ACTIVE** | runtime-populated; consumed by live draw/damage paths; zero in image is a load-timing artifact, not death |
| `DAT_007ED3D0`, `DAT_00ABD490` | **ACTIVE** | every CellSpread detonation; not bridge-specific but in the dispatch path |
| LOW selectors (`SelectBridgeTileVariant_Low`, `SelectDestroyedBridgeTile_Low`, `UpdateBridgeTile_Low`) | **ACTIVE** | only at repair/edge-refresh events (callers `MarkBridgesForRepair_*`), NOT per frame; live in YR |
| `FUN_004863D0` (tile-set membership) | **ACTIVE but light** | adjacency / shore-fallback; no flag gate; reachable in YR; not per-frame |
| `HasBridgeOverlay @ 0x4865D0` | **ACTIVE** (misnamed) | operates on tile_index; PROOFED for chronominer-locomotion in Ghidra plate; not dead |
| Bridge damage dispatch (blocks A/B/C/D, ApplyDamageToCell) | **ACTIVE, gated** | requires `SpecialFlags & 0x8000` (`DestroyableBridges`, default `yes` in YR) AND `warhead+0x144` (`Wall=yes`); both default-reachable. Map-init/hut-death paths (`DestroyBridge_*_OnHutDeath`) are unconditional |
| FoggedObject snapshot walker `FUN_004D1890` | **TS-legacy dormant** | gated on FogOfWar `SpecialFlags & 0x1000`, default off in YR; not a bridge *table* |
| `RMG_PlaceBridge @ 0x59E740` | **TS-legacy** | random-map generator; YR ships pre-authored maps; not a bridge *table* |
| `DAT_00880940` render-cache token | **dead TS residual** | always 0; 0 writers in binary; not a bridge *table* |

No bridge-overlay *table* in the family is TS-dead. The two genuinely dead items (`DAT_00880940`,
FoggedObject path) are outside the table scope; flagged for completeness.

---

## 4. Comparison vs current Rust (table-by-table, helper-by-helper)

Default verdict on any unproven difference is **DRIFT**. Each row states the gamemd value, the Rust value,
and the proof status.

### 4.1 Latin-square frame-jitter table — PARITY (data), but SCATTERED + LAYER-MISPLACED

- **Data:** `BRIDGE_BODY_LATIN_SQUARE` @ `src/app_instances/bridges.rs:30` = `[0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2]`.
  Byte-identical to gamemd `g_LatinSquare` dump `{0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2}`
  (read_memory 0x0081CC30). **Values: PARITY.** Test `latin_square_table_is_canonical_4x4`
  (`src/app_instances/bridges.rs:601`) pins it.
- **Index math:** Rust `idx = ((ry & 3) << 2) | (rx & 3)` (`src/app_instances/bridges.rs:72`). gamemd
  `((cell+0x26 & 3) << 2) | (cell+0x24 & 3)` (decompile_function 0x47F6A0). **PARITY** — same low-2-bits
  selection, same x-in-low-nibble convention.
- **Boundary gate:** Rust applies jitter ONLY for `DamageState::Healthy { variant: 0 }`
  (`src/app_instances/bridges.rs:71`), which maps to state bytes 0 (NS) / 9 (EW). gamemd:
  `if (state == 0 || state == 9)` (decompile_function 0x47F6A0). **PARITY** — both gate to exactly
  the two boundary states; variants 1..5 and damage states draw `frame = state` directly.
- **DRIFT (placement/layer, not value):** the table + frame formula live in `src/app_instances/` (app/render
  layer). This is correct for render-only consumption, BUT the family-substrate boundary (§6) should expose
  the *jitter function* as a pure helper so render and any future headless preview share one source. Today
  the only consumer is render, so this is a **structural** drift (single-source risk), not an output drift.
  Severity: LOW — fires every frame a healthy bridge is on screen, but currently single-consumer so no
  divergence is observable yet; flag it because a second consumer (e.g. minimap/replay preview) would
  re-derive and could drift.

### 4.2 Overlay-byte dispatch ranges (LOW `0x4A..=0x63`, HIGH `0xCD..=0xE6`) — PARITY (values), DISPERSED

- gamemd `ApplyDamageToCell`: `cell+0x44 ∈ [0x4A..=0x63]` → DestroyBridge_Low; `∈ [0xCD..=0xE6]` →
  DestroyBridge_High; inclusive endpoints, signed `JL/JG` compares (decompile_function 0x587180).
- Rust LOW overlay damage step `low_bridge_overlay_damage_step_ra2` (`src/sim/bridge_specs.rs:97`) uses
  `in_range_inclusive(center, 0x4a, 0x63)` and `(center, 0xcd, 0xe6)` (`src/sim/bridge_specs.rs:105-106`).
  **PARITY** on the raw-body ranges and inclusivity.
- The connected-section selector `low_bridge_connected_section_selector_yr` (`src/sim/bridge_specs.rs:154`)
  uses the wider gate `0x4a..0x65` (wood) / `0xcd..0xe8` (concrete) including the final-destroyed bytes
  `0x64/0x65` / `0xE7/0xE8` (`src/sim/bridge_specs.rs:173-177`). gamemd's `SelectDestroyedBridgeTile_Low`
  gate and the state-machine gate both include the final bytes (BRIDGE_DISPLAY_TABLE §9.1). **PARITY** for
  this distinct gate; note the two gates legitimately differ (raw-body dispatch vs connected-section
  classify) — keep them separate in the substrate.
- `is_bridge_overlay_index` (`src/map/overlay_types.rs:32`) membership = `24|25|237|238|74..=101|122..=125|
  205..=232|233..=236`. The LOW wood band `74..=101` = `0x4A..=0x65` (raw body `0x4A..0x63` + final
  `0x64/0x65`). The HIGH bands here are `205..=232` (`0xCD..0xE8`) + `233..=236`. **PARITY** for "is this a
  bridge overlay byte" membership (it intentionally unions raw + final). `is_high_bridge_index`
  (`src/map/overlay_types.rs:45`) = `24|25|237|238` = the four HIGH anchors. **Cross-check:** gamemd's
  `HasBridgeOverlay @ 0x4865D0` operates on tile_index, not overlay byte, so it is *incomparable* to these
  Rust overlay-byte ranges (decompile_function 0x004865D0) — no byte-range disagreement exists to flag.
- **DRIFT (dispersion):** the byte ranges `0x4A/0x63/0xCD/0xE6/0x64/0x65/0xE7/0xE8` and the HIGH anchor IDs
  `0x18/0x19/0xED/0xEE` are restated as magic literals across `bridge_specs.rs`, `overlay_types.rs`, and
  `bridge_facts.rs` (`high_bridge_stamp_for_overlay`, `src/map/bridge_facts.rs:114`). No single
  named-constant source. Severity: MEDIUM — fires on every bridge-damaging hit; a future edit to one site
  (e.g. widening a band) silently desyncs the others.

### 4.3 Destruction next-overlay tables — PARITY (values), but MODELED AS STATIC TABLE vs gamemd INLINE

- Rust embeds four static 16-entry tables in `src/sim/bridge_specs.rs`:
  `DESTRUCTION_OVERLAY_HIGH_NS` (L419), `_HIGH_EW` (L425), `_LOW_NS` (L435), `_LOW_EW` (L443), consumed by
  `pick_destruction_overlay` (`src/sim/bridge_specs.rs:397`).
- gamemd does NOT store these as static globals. The addresses `0x57E7A0`/`0x57ED00`/`0x57DD50`/`0x57E2A0`
  are FUNCTION prologues (read_memory 0x57E7A0 / 0x57DD50 = `81 EC CC 00 00 00`). The next-overlay values
  are computed from inline 16-entry LOCAL arrays initialized in each `ApplyBridgeDestruction_*` body
  (BRIDGE_DISPLAY_TABLE §2.4).
- **Verdict: PARITY on observable output, INTERNAL-ONLY representation difference — DOWNGRADED from DRIFT
  with proof.** The Rust static table values (HIGH_NS `0xFF,0xD2,0xD5,0xFF,0xD1,0xD3,0xD5,0xFF,0xD4,0xD4,
  0xE7,...`; LOW_NS `0xFF,0x4F,0x52,0xFF,0x4E,0x50,0x52,0xFF,0x51,0x51,0x64,...`; etc.) are the materialized
  results of gamemd's inline switch, verified entry-by-entry against the function bodies in
  `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.2 and the LOW peers, and the `0xFF` sentinel maps
  to gamemd's `-1` "no transition". A const table indexed by the same `CheckBridgeNeighbors_*` result, with
  the same 16 entries and the same sentinel, produces bit-identical output for the full 0..15 input space
  (Rust bounds-checks `>= 16 → None`, `src/bridge_specs.rs:402`). This is the project-sanctioned
  "Rust-native structure, gamemd-native semantics" pattern. **Caveat:** this downgrade holds ONLY because
  the inline arrays were proven equal across all 16 indices; it is NOT a generic license to swap inline
  computation for a table elsewhere without the same exhaustive check.

### 4.4 LOW tile-variant selection (mask + PRNG) — STRUCTURAL DRIFT (RNG instance + inline arithmetic)

- gamemd `SelectDestroyedBridgeTile_Low @ 0x579620`: tile index computed **inline** by if/else on an 8-bit
  adjacency mask, with PRNG-derived variants drawn from `g_GlobalRng @ 0x00ABE890` via
  `FUN_00598030(lo,hi)` (rejection sample). Forms: `(uVar4 % 3) + 9`, `+0xf`, `+0x17`, `+5`, `+0xc`,
  `+0x23`, `(uVar4 & 1) + 0x1d`; plus fixed `0x22,0x27,0x21,0x28,0x12,0x1a,0x15,8,2,0x16,0x26,1`. NOT a
  16-entry lookup table. (decompile_function 0x579620; disassemble_function 0x598030.)
  **CORRECTED 2026-06-04:** stage-2 said "TWO RNG-derived variant choices". `SelectDestroyedBridgeTile_Low`
  draws `g_GlobalRng` exactly **ONCE** (`uVar4 = FUN_00598030()` at function entry); that single `uVar4` is
  reused for every variant form (`% 3` and `& 1`). There is ONE draw per call, not two.
  (decompile_function 0x579620, re-verified.) The `SelectBridgeTileVariant_Low @ 0x57ACF0` (healthy peer)
  was NOT re-decompiled this pass — its draw count is UNCHECKED; do not assume it matches.
- Rust `low_bridge_connected_section_selector_yr` (`src/sim/bridge_specs.rs:154`) + `classify_low_bridge_band`
  (`src/sim/bridge_specs.rs:298`) + `pattern_a_new_index`/`pattern_b_new_index` (`src/sim/bridge_specs.rs:278/288`)
  model a SUBSET (band classification + a deterministic next-index step) and explicitly note they are "not
  yet fully wired into the live runtime" (`src/sim/bridge_specs.rs:1` doc). The full mask-driven,
  TWO-RNG-variant inline arithmetic of gamemd's `SelectDestroyedBridgeTile_Low` is **not** reproduced.
- **DRIFT (UNCHECKED → DRIFT):** the Rust pure helpers do not yet model (a) the 8-direction adjacency mask
  decode, (b) the two RNG-derived variant choices, or (c) the **RNG instance** (gamemd draws LOW variants
  from `g_GlobalRng`, distinct from the Scenario RandomClass used by the BridgeStrength gate — §4.5). Until
  the live wood-bridge repair path is implemented and proven bit-identical, this is DRIFT. Severity: LOW for
  now — these helpers are not on a live runtime path (callers are `MarkBridgesForRepair_*` in gamemd; the
  Rust runtime repair path is a stub), so the divergence is not yet player-observable; it becomes
  HIGH the moment wood-bridge repair/destroy is wired (visible wrong connector tile, and a lockstep RNG
  desync if the wrong RNG instance is consumed).

### 4.5 RNG-instance contract for the BridgeStrength gate — must be honored (lockstep)

- gamemd: the `RandomRanged(1, BridgeStrength)` draw in the AoE bridge blocks calls `0x65c7e0` with
  `ECX = SpecialFlags_base(0x00a8b230) + 0x218` = the **Scenario RandomClass** (`Scen->Random`), at
  0x489FEF/0x48A173/0x48A23F/0x48A299 (disassemble_function 0x489280). This is DIFFERENT from the LOW
  tile-variant RNG (`g_GlobalRng @ 0xABE890`, §4.4). Two distinct instances.
- Rust `low_bridge_overlay_damage_step_ra2` takes the roll as a parameter
  (`random_ranged_1_bridge_strength: i32`, `src/sim/bridge_specs.rs:102`) rather than drawing it, so the
  *instance binding* is deferred to the (not-yet-written) caller. The combat AoE path
  (`src/sim/combat/combat_aoe.rs`) currently does NOT draw a BridgeStrength roll at all (no bridge-tile
  damage block — only the object-layer split is implemented).
- **DRIFT (missing + unbound):** the BridgeStrength gate and its Scenario-RandomClass binding are not yet
  present on the live AoE path. Severity: MEDIUM — fires whenever a `Wall=yes` warhead hits a bridge in a
  skirmish; absence means bridges currently can't take AoE tile damage at all, and when added the RNG
  instance MUST be `Scen->Random`, not the global RNG, or replays desync.

### 4.6 CellSpread tables `DAT_007ED3D0` / `DAT_00ABD490` — PARITY via a SEPARATE module (cross-family)

- gamemd loop bound `DAT_007ED3D0[ftol(CellSpread)]`; per-cell offset `(i16)DAT_00ABD490[i*4]` X /
  `DAT_00ABD492[i*4]` Y (disassemble_function 0x489280).
- Rust: `cell_spread::cells_in_spread(radius)` is consumed in `combat_aoe.rs:106`
  (`for &(dx, dy) in cell_spread::cells_in_spread(spread_radius)`). The spread cell-list lives in a shared
  `cell_spread` module, NOT in the bridge family.
- **Verdict: cross-family shared table.** This is the correct ownership (CellSpread is general AoE infra,
  not bridge data). The bridge substrate must NOT duplicate it; it should consume `cell_spread`. Flagged for
  the synthesis stage so the AoE/CellSpread family and the bridge family agree on a single owner. Equality
  of the Rust spread list to gamemd's `DAT_007ED3D0`/`DAT_00ABD490` is out of THIS lane's scope (it is the
  CellSpread family's parity item) — marked **UNCHECKED here**, owned elsewhere.

### 4.7 Tileset-window constant (`0x10`) and bridge-set classifiers — PARITY (value), GATED CORRECTLY

- gamemd: a concrete/wood bridge tile-set occupies the first 16 tiles `[base, base+0x10)`; the high/low SM
  classifier uses `(cell+0x38 − base) + 1` (disassemble_function 0x489280 @ 0x489ECC / 0x48A0A8).
- Rust: `BRIDGE_TILESET_WINDOW = 0x10` (`src/sim/map/bridge_topology.rs:80`); `is_bridge_tileset` /
  `is_wood_bridge_tileset` (`src/sim/map/bridge_topology.rs:193/208`) use `[base, base+0x10)` gated on
  `base >= 0`. **PARITY** on window width and the `base >= 0` (≈ `!= -1`) gate. Tests
  `is_bridge_tileset_distinct_from_structural_flag` (`bridge_topology.rs:385`) pin the bounds (lower
  inclusive, `base+0x10` exclusive).
- **DRIFT (base subtraction `+1`):** gamemd computes `rel = (tile_index − base) + 1` and tests `rel ∈`
  bridgehead-class sets; Rust `is_bridge_tileset` only tests window membership (`base..base+0x10`) and does
  NOT compute the `+1`-shifted `rel` used by the high/low SM classifier's bridgehead-class membership test
  (`rel ∈ {DAT_00abad30..+3} ∪ {DAT_00aa1028..+3}`). The Rust SM/bridgehead-class routing path is not
  implemented. Severity: MEDIUM — fires when distinguishing body vs bridgehead vs ramp on a damaging hit;
  absent today because the SM classifier isn't on a live path.

### 4.8 AoE object-layer selector deck height — KNOWN DRIFT (already flagged in-tree)

This is adjacent to (not strictly part of) the table family, but it shares the bridge-damage dispatch
routine and is the most consequential bridge DRIFT currently live:

- gamemd layer selector: `impact_z > GetGroundHeight + (DAT_0089E864 − sign)/2` (strict `>`; `JLE` stays
  ground). (disassemble_function 0x489280 @ 0x48955E-0x48958D, re-verified 2026-06-04.)
- **CORRECTED 2026-06-04 (stage-2 deck-height number was WRONG):** the stage-2 text claimed
  `DAT_0089E864 = 2 × per_level`, half-deck = "1 level (value 2)", and recommended Rust `= 2`. The writer
  byte at `0x00489115` is **`FADD double ptr [0x007e1738]`** (= +0.5 rounding), NOT `FMUL` — so the writer
  computes `DAT_0089E864 = ftol(DAT_0089E870 × 4 + 0.5) = round(4 × DAT_0089E870)`, i.e. **`4 × per_level`,
  not `2 ×`.** (get_assembly_context 0x00489120 → `LEA ECX,[EAX*0x4]; FILD; FADD [0x007e1738]; CALL ftol;
  MOV [0x0089e864],EAX`; read_memory 0x007e1738 = 0.5.) The two prior cross-docs
  (`DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY` §3 and `GATE_BRIDGE_DECK_HEIGHT_RESOLUTION` §3) both mis-decoded
  this byte as `FMUL` and propagated the false `2 ×` identity. **The DAT_00AC13BC deck-Z writer at
  0x005F3880 genuinely uses `FMUL 0.5` (= `2 × per_level`), but that is a DIFFERENT global** — the two were
  conflated.
- Net gamemd threshold: `ground + DAT_0089E864/2 = ground + 2 × DAT_0089E870`. With `DAT_0089E870` the
  per-level lepton step (nominally 104; same family as the deck-Z per-level), this is `ground + 208 leptons
  = ground + 4 cell-levels` — i.e. the threshold equals the **FULL deck height (4 levels)**, the same height
  `GetEffectiveHeight @ 0x487D50` adds for a bridge cell (`Level + 4`, decompile_function 0x487D50,
  re-verified). gamemd selects the deck list only when `impact_z` is strictly above ground + full deck.
- Rust authoritative selector `select_object_damage_layer` (`src/sim/combat/combat_aoe.rs:217`) does
  `impact_z > cell.level + bridge_height/2` (`combat_aoe.rs:231`) where
  `bridge_height = max(deck_level − level, BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS=4) = 4` levels
  (`combat_aoe.rs:238-239, :42`). Effective Rust threshold = `level + 4/2 = level + 2 levels` — i.e. the
  **HALF-deck midpoint**, half of gamemd's full-deck threshold. (Here Rust `impact_z`/`level`/`deck_level`
  are in cell-LEVEL units, not leptons — frame difference noted; the divide-by-two is applied in level
  units.)
- **DRIFT: confirmed, but the stage-2 corrective VALUE was wrong.** Rust is too LOW (half deck = `+2`
  levels) vs gamemd's full deck (`+4` levels); the error is the `/2` divide in `select_object_damage_layer`,
  not the constant magnitude. gamemd parity in Rust's level frame requires the threshold offset to be the
  **full deck (4 levels)**: either drop the `bridge_height/2` divide (compare `impact_z > level +
  bridge_height` with `bridge_height = 4`), or set the selector constant so the post-divide value is `4`.
  The shadow `bridge_topology::BRIDGE_DECK_HEIGHT_LEVELS = 2` (`bridge_topology.rs:76`) is **also suspect**:
  it encodes `2`, which matches the now-refuted `2 × per_level` premise — do NOT treat it as the
  proven-correct value until `aoe_object_layer` (`bridge_topology.rs:248`) is re-audited against the full
  4-level threshold. Severity: MEDIUM — fires whenever an AoE detonation lands on/under a high bridge with
  units on both layers; the threshold being half too low routes some under-bridge splash to the deck list
  (and vice versa). Hashed-state; deferred cutover. **The cutover must NOT adopt the stage-2 value `2`.**

---

## 5. gamemd-native behavior contract (integrated, with boundaries)

### A. Latin-square frame jitter (per-frame, bridge body SHP) — `DrawOverlay_Body @ 0x47F6A0`
- Input: `state = cell.bridge_damage_state (u8 @ +0x11E)`, range 0..17; `x = cell+0x24 & 0xFFFF`,
  `y = cell+0x26` (high half).
- Index: `idx = ((y & 3) << 2) | (x & 3)` → 0..15; read `g_LatinSquare[idx]` (value 0..3, dword stride).
- Output frame: `if state == 0 OR state == 9: frame = state + g_LatinSquare[idx]; else: frame = state`.
  States 1..8 and 10..17 → `frame = state`, no jitter.
- Z value to blit: `z = (height + HasBridge*4) * -15 + -2`, `height = (i8)cell+0x11B`,
  `HasBridge = (cell+0x140 >> 7) & 1`. Blit flag `0x4E00`.
- Boundary: overlay `0xA7` and `0xB2` early-return. No overlay-range validity check otherwise — trusts
  `cell+0x44`.
- **Port invariant:** keep dword-stride value range 0..3 and the exact boundary-state gate (ONLY 0 and 9).
  Jitter on all states, or full x/y instead of low-2-bits, is DRIFT.

### B. Overlay-first damage dispatch — `ApplyDamageToCell @ 0x587180`
- Cell fetch: `index = y*0x200 + x` (512 stride), valid `0 <= index < 0x40000`, non-null
  `g_CellArray_Base[index]`; else fallback `&DAT_00abdc50`, store coord to `DAT_00abdc74`.
- Dispatch ORDER (must preserve): (1) `cell+0x44 ∈ [0x4A..=0x63]` → `DestroyBridge_Low`, tail.
  (2) `cell+0x44 ∈ [0xCD..=0xE6]` → `DestroyBridge_High`, tail. Signed compares, inclusive endpoints,
  exclusive of `0x64/0x65` and `0xE7/0xE8`. (3) On miss: high SM classifier
  `high_rel = (cell+0x38 − g_BridgeSet) + 1`; if `flags & 0x100` set, look up self coord (`flags & 0x80`
  set) or anchor `*(cell+0x2c)+0x24` (clear), route High if that neighbor's `+0x44 ∈ {0x18, 0x19}`; else
  test `high_rel ∈ {DAT_00abad30..+3} ∪ {DAT_00aa1028..+3}`. (4) low SM:
  `low_rel = (cell+0x38 − g_WoodBridgeSet) + 1`; route Low if neighbor `+0x44 ∈ {0xed, 0xee}` OR `low_rel ∈`
  same two sets.
- NO deck-level read; classifier reads only `+0x38, +0x44, +0x140, +0x24, +0x2c`. A Rust port using
  `deck_level >= 4` for high/low routing is DRIFT.
- Base subtraction direction `cell.tile_index − base + 1` (sign verified `SUB ESI,EBX; INC ESI`).

### C. Outer AoE blocks A/B/C/D — `Apply_area_damage @ 0x489280`
- Gate: `SpecialFlags (g @ 0x00a8b230) & 0x8000` (`TEST CH,0x80` @ 0x489EB2) AND `warhead+0x144` (`Wall`) —
  both required.
- Blocks evaluated SEQUENTIALLY; A/B (high/low SM candidate → `ApplyDamageToCell`) fall through to C/D
  (direct overlay → `DestroyBridge_Low/High`). NOT mutually exclusive. Each non-Ion block has its OWN
  `RandomRanged(1, BridgeStrength)` draw. A "first winner stops" Rust model is DRIFT.
- **RNG instance (lockstep):** BridgeStrength gate calls `0x65c7e0` with
  `ECX = 0x00a8b230 + 0x218` = Scenario RandomClass (`Scen->Random`), at 0x489FEF/0x48A173/0x48A23F/0x48A299.
  Compare `RandomRanged(1, BridgeStrength) < damage` (`JGE skips`) — strict `<`; equality fails to damage.
- IonCannon: `warhead == Rules+0xFF0` bypasses the RNG gate and gets up to 3 retries on the A/B path
  (`MOV ESI,3; DEC ESI` @ 0x48A015/0x48A199). C/D are single-shot.
- Layer selector: ONCE from impact cell. `impact_cell+0x140 & 0x100` set AND
  `impact_z (param+8) > GetGroundHeight + (DAT_0089E864 − sign)/2` (strict `>`) → bridge list `cell+0xE8`,
  else ground `cell+0xE4`. Same selector for every CellSpread cell. (0x48955E-0x48958D.)
- CellSpread loop bound `DAT_007ED3D0[ftol(wh+0x124)]`; per-cell offset `(i16)DAT_00ABD490[i*4]` X /
  `DAT_00ABD492[i*4]` Y added to impact x/y. Cell world center `x*0x100 + 0x80`.

### D. LOW tile-variant selection — `SelectDestroyedBridgeTile_Low @ 0x579620`
- RNG: `FUN_00598030(lo,hi)` = rejection sample from `g_GlobalRng @ 0x00ABE890`. `lo + floor(Random() *
  scale * (hi-lo+1))`, redrawn while `> hi`.
- Adjacency: `ComputeBridgeAdjacencyMask_Low(cell)` → 8-bit mask; further calls on +y, +x, +x+y neighbors
  (512-stride bounds, `&DAT_00abdc50` fallback).
- Tile index: inline if/else on mask bits with PRNG variants (`(uVar4 % 3)+9`, `+0xf`, `+0x17`, `+5`,
  `+0xc`, `+0x23`, `(uVar4 & 1)+0x1d`) plus fixed values. NOT a 16-entry lookup.
- Stamp: tile `g_IsometricTileTypeClass_Array[-4 + (DAT_00aa1020 + iVar6)*4]`; coord
  `(cell.x + (i16)DAT_00abdda4[iVar6*4], cell.y + (i16)DAT_00abdda6[iVar6*4])`; then `ApplyBridgeTile`.
  Returns 1 unless `iVar6 < 1`.
- **Port invariant:** mask-driven inline arithmetic, a SINGLE RNG draw (one `g_GlobalRng` value reused for
  every `%3`/`&1` variant form — corrected 2026-06-04, decompile_function 0x579620), drawn from
  `g_GlobalRng` (NOT Scen->Random). Modeling it as a static next-overlay table is DRIFT.

### E. Tile-set membership — `FUN_004863D0`
- Input `cell+0x38` (i32 tile_index), output bool. 11 runtime bases, each `base != -1 && base <= idx <
  base + size` (sizes 0x28,4,4,4,4,0x14,4,0x10,0x10,2,0x1c). The 4 end-piece sets additionally gate on
  `cell+0x11a` (sub_tile) ∈ specific pairs. NOT an overlay-byte classifier — do not feed `cell+0x44`.

**Boundary/edge summary:** overlay-range tests are inclusive endpoints, signed compares; `0x64/0x65`
(LOW final destroyed), `0xE7/0xE8` (HIGH final destroyed), and `-1` (no overlay) all MISS the inner
`ApplyDamageToCell` direct dispatch (only `0x4A..0x63` / `0xCD..0xE6` hit). Latin square only at states 0/9.
TWO distinct RNG instances (Scen->Random for BridgeStrength; g_GlobalRng for LOW variant) — mixing them is a
lockstep DRIFT.

**Open / UNCHECKED (out of this lane's static reach):** runtime values of all theater-populated bases
(g_BridgeSet, g_WoodBridgeSet, DAT_00abad30, DAT_00aa1028, railing tables, coord-delta tables) — zero in
the static image; require a live-debugger capture post-map-load. Treat as opaque tags whose relative
offsets (+0..+3 for bridgehead sets, ×16 stride for railings, ×4 for coord deltas) are the contract, not
their absolute values. CellSpread spread-list equality to `DAT_007ED3D0`/`DAT_00ABD490` is owned by the
CellSpread family (§4.6), UNCHECKED here.

---

## 6. Designed Rust-native substrate boundary

**One service, read-only, deterministic: `sim::map::bridge_overlay_tables`** (new module, sibling to
`sim/map/bridge_topology.rs`). It owns the *pure, gamemd-native table data + classifiers* for the family;
it does NOT own mutable bridge damage state (that stays in `sim/bridge_state` / `bridge_orchestrator`) and
does NOT own the CellSpread spread-list (that stays in the `cell_spread` AoE module).

One-sentence boundary: *a single pure, render-free, RNG-free table service that exposes the Latin-square
frame jitter, the overlay-byte dispatch classification, the tileset-window/SM base-relative classifiers, and
the destruction next-overlay lookup as named-constant-backed functions, with the two PRNG-driven LOW
selectors expressed as pure functions that take the roll + adjacency mask as inputs so the caller binds the
correct RNG instance.*

### 6.1 Where it lives / layering
- Module path: `src/sim/map/bridge_overlay_tables.rs`. In `sim/` (invariant #1: never depends on
  render/ui/audio/net). Depends only on `map/bridge_facts` (flag bits, overlay-byte→stamp), `map/overlay_types`
  (overlay-byte taxonomy), and `util` (no float in sim math).
- The render-side Latin-square frame builder (`app_instances/bridges.rs`) becomes a *consumer* of the
  service's pure `bridge_body_frame(...)` function (or its `latin_jitter(...)` primitive) instead of holding
  its own copy of `BRIDGE_BODY_LATIN_SQUARE`. Render imports a `sim` pure fn — allowed (render depends on
  sim).

### 6.2 Data ownership / construction source
- **Static tables (embedded const, from verified gamemd dumps):** `LATIN_SQUARE: [u8; 16]`,
  `DESTRUCTION_OVERLAY_{HIGH,LOW}_{NS,EW}: [u8; 16]`, the named overlay-byte range constants
  (`LOW_BODY_LO=0x4A`, `LOW_BODY_HI=0x63`, `LOW_FINAL_NS=0x64`, `LOW_FINAL_EW=0x65`, `HIGH_BODY_LO=0xCD`,
  `HIGH_BODY_HI=0xE6`, `HIGH_FINAL_NS=0xE7`, `HIGH_FINAL_EW=0xE8`, `HIGH_ANCHOR_IDS=[0x18,0x19,0xED,0xEE]`),
  and `TILESET_WINDOW=0x10`. These are gamemd-image-dumped values (not INI), so embed as `const` with the
  read_memory citation in the doc comment.
- **Runtime/theater bases (NOT embedded):** g_BridgeSet, g_WoodBridgeSet, the bridgehead-class bases, the
  railing tables, and the coord-delta tables are theater-load state. The service takes them as *parameters*
  (`BridgeTilesetBases { concrete_base: Option<i32>, wood_base: Option<i32>, ns_head_base: Option<i32>,
  ew_head_base: Option<i32> }`), constructed by the existing theater loader. The service never reads them
  from a global. This keeps the service pure and lets tests pin synthetic bases.
- **RNG (NOT owned):** the two PRNG-driven LOW selectors are pure functions taking the already-drawn roll
  and adjacency mask. The CALLER (a future `sim/bridge_state` repair step) draws from `g_GlobalRng` for LOW
  variants and from `Scen->Random` for the BridgeStrength gate — the service documents which instance each
  caller must use but never holds an RNG.

### 6.3 API surface (signatures; pure, `#[inline]` where hot)
```
// --- Per-frame body frame (replaces app_instances/bridges.rs copy) ---
pub fn latin_jitter(x: u16, y: u16) -> u8;                 // 0..3, idx = ((y&3)<<2)|(x&3)
pub fn bridge_body_frame(state_byte: u8, x: u16, y: u16) -> u8;
//   state 0|9 -> state + latin_jitter; else state. (Caller maps DamageState->state_byte.)

// --- Overlay-byte dispatch classification (ApplyDamageToCell ranges) ---
pub enum BridgeOverlayClass { LowBody, HighBody, LowFinal, HighFinal, NotBridge }
pub fn classify_overlay_byte(overlay: u8) -> BridgeOverlayClass;   // inclusive signed-equiv ranges

// --- Tileset-window + SM base-relative classifiers ---
pub fn in_bridge_tileset(tile_index: i32, base: Option<i32>) -> bool;     // [base, base+0x10)
pub fn sm_base_relative(tile_index: i32, base: i32) -> i32;               // (tile_index - base) + 1
pub fn is_bridgehead_class(rel: i32, ns_head_base: Option<i32>, ew_head_base: Option<i32>) -> bool;
//   rel ∈ {ns_head_base..=+3} ∪ {ew_head_base..=+3}

// --- Destruction next-overlay (materialized inline-array tables) ---
pub fn destruction_overlay(neighbor_check: u8, axis: Axis, is_high: bool) -> Option<u8>;
//   0xFF sentinel -> None; index >= 16 -> None.

// --- LOW variant selection (pure; caller binds g_GlobalRng) ---
// CORRECTED 2026-06-04: SelectDestroyedBridgeTile_Low draws g_GlobalRng ONCE per call
// (single `uVar4`, reused for all `%3`/`&1` forms) — pass ONE roll, not two.
pub fn low_destroyed_tile(mask: u8, roll: u32 /*..*/) -> LowVariantResult;
pub fn low_healthy_tile(mask: u8, roll: u32 /*..*/) -> LowVariantResult;
//   LowVariantResult { rel_tile_index: i32, coord_delta: (i16, i16) }
//   coord_delta sourced from the theater coord-delta tables (passed in or indexed by rel).
```
Determinism guarantees: no `f32/f64`; no global reads; no RNG draws inside the service; all indexing
bounds-checked; `BTreeMap`/array iteration only. The service is a leaf — a wrong call site can be replayed
deterministically because every input is explicit.

### 6.4 Stateful-vs-pure boundary
- **Pure (this service):** everything in §6.3.
- **Stateful (stays out):** the bridge damage state machine (`apply_ramp_transition`, collapse
  propagation), the rim-refresh writer (`UpdateBridgeEdgeTiles_*`), the OverlayGrid/BridgeRuntimeState
  mutation, and the actual RNG draws. The service is the *table oracle* those stateful systems consult.

---

## 7. Retire list (ad hoc / duplicated / scattered Rust to fold into the service)

| Item | File:line | Why it retires |
|---|---|---|
| `BRIDGE_BODY_LATIN_SQUARE` const | `src/app_instances/bridges.rs:30` | duplicate of the service `LATIN_SQUARE`; render should call `sim` `latin_jitter`/`bridge_body_frame` |
| Inline Latin index + jitter logic | `src/app_instances/bridges.rs:71-84` | move the `state 0|9` jitter gate into `bridge_body_frame`; render keeps only the `DamageState→state_byte` mapping |
| `DESTRUCTION_OVERLAY_HIGH_NS/_HIGH_EW/_LOW_NS/_LOW_EW` | `src/sim/bridge_specs.rs:419/425/435/443` | move verbatim into the service as the canonical destruction tables; `bridge_specs` consumes via `destruction_overlay()` |
| `pick_destruction_overlay` | `src/sim/bridge_specs.rs:397` | becomes a thin caller of `destruction_overlay()` (or relocates wholesale) |
| Inline overlay-byte range literals `0x4a/0x63/0xcd/0xe6` | `src/sim/bridge_specs.rs:105-106` (and the wider `0x65/0xe8` gate `:173-177`) | replace with `classify_overlay_byte()` + named consts; single source for the band boundaries |
| HIGH anchor IDs `0x18/0x19/0xED/0xEE` | `src/map/bridge_facts.rs:114-122` (`high_bridge_stamp_for_overlay`) | reference the service's `HIGH_ANCHOR_IDS` const instead of inline match arms |
| Bridge overlay-byte membership literals | `src/map/overlay_types.rs:32-42` (`is_bridge_overlay_index`), `:45-47` (`is_high_bridge_index`) | derive the LOW/HIGH band edges from the service consts so `74..=101`/`205..=232` can't drift from `0x4A..0x65`/`0xCD..0xE8` |
| `BRIDGE_TILESET_WINDOW = 0x10` | `src/sim/map/bridge_topology.rs:80` | move to the service as `TILESET_WINDOW`; `bridge_topology` imports it (single source for the 16-tile window) |
| LOW band/next-index helpers (subset of the gamemd selector) | `src/sim/bridge_specs.rs:154-315` (`low_bridge_connected_section_selector_yr`, `classify_low_bridge_band`, `pattern_a/b_new_index`) | reconcile into the service's `low_destroyed_tile`/`low_healthy_tile`, which must model the FULL mask + two-RNG arithmetic (§4.4) — these partial helpers retire once the full model lands |

**Explicit duplications across files** (the headline reason to centralize): the overlay-byte band edges and
HIGH anchor IDs appear independently in THREE files — `bridge_specs.rs`, `overlay_types.rs`,
`bridge_facts.rs` — with no shared constant. The `0x10` tileset window appears in `bridge_topology.rs`
while the SM `(tile−base)+1` math is unimplemented elsewhere. The Latin square is duplicated between the
render layer and (conceptually) the sim contract.

---

## 8. Migration slices + acceptance tests

Ordered, each independently shippable. Slices 1-4 are **pure-data-parity** (hash-neutral, no behavior
change); slices 5-6 are **genuinely stateful** (touch the live runtime / RNG; hash-relevant, separate
review). Map to the substrate program convention: ship the pure oracle first, flip authority last.

### Slice 1 — create `sim/map/bridge_overlay_tables.rs` with the static consts (pure, hash-neutral)
Embed `LATIN_SQUARE`, the named overlay-byte band consts, `HIGH_ANCHOR_IDS`, `TILESET_WINDOW`, and the four
`DESTRUCTION_OVERLAY_*` tables, each with the read_memory/decompile citation in its doc comment. Implement
`latin_jitter`, `bridge_body_frame`, `classify_overlay_byte`, `in_bridge_tileset`, `sm_base_relative`,
`is_bridgehead_class`, `destruction_overlay`. No callers yet.
- **AT-1a `latin_square_exact_dump`:** assert `LATIN_SQUARE == [0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2]`
  (gamemd read_memory 0x0081CC30, exact 16-entry equality).
- **AT-1b `bridge_body_frame_boundary_and_nonboundary`:** for every `state_byte ∈ 0..=17` and a grid of
  `(x,y)` covering all 16 low-2-bit combos: state 0 → `0 + LATIN[idx]`, state 9 → `9 + LATIN[idx]`, all
  other states → `state` unchanged. (input space includes both boundary states and all 16 jitter cells.)
- **AT-1c `classify_overlay_byte_boundaries`:** exact map for the full `0..=255` byte space: `0x4A..=0x63`→
  LowBody, `0x64`→LowFinal, `0x65`→LowFinal, `0xCD..=0xE6`→HighBody, `0xE7`→HighFinal, `0xE8`→HighFinal,
  `0x49`/`0x66`/`0xCC`/`0xE9`/`0xFF`(−1)→NotBridge. (boundaries: one below/above each band, the two finals,
  the −1 sentinel.)
- **AT-1d `destruction_overlay_full_index_space`:** for `neighbor_check ∈ 0..=15` × {NS,EW} × {high,low},
  assert each entry equals the gamemd-verified table value, `0xFF→None`, and `>=16 → None`. (full 0..15 plus
  the out-of-range case — this is the exhaustive check that licenses §4.3's downgrade.)
- **AT-1e `tileset_window_and_sm_base`:** `in_bridge_tileset(base, Some(base))==true`,
  `in_bridge_tileset(base+0x0F, Some(base))==true`, `in_bridge_tileset(base+0x10, Some(base))==false`,
  `in_bridge_tileset(base-1, Some(base))==false`, `in_bridge_tileset(x, None)==false`,
  `in_bridge_tileset(x, Some(-1))==false`; `sm_base_relative(base, base)==1`. (lower-inclusive,
  upper-exclusive, no-set, −1 base, the `+1` shift.)

### Slice 2 — render consumes the service for the Latin square (pure, hash-neutral)
Replace `app_instances/bridges.rs` `BRIDGE_BODY_LATIN_SQUARE` + inline jitter with calls to
`bridge_overlay_tables::bridge_body_frame` / `latin_jitter`. Keep the `DamageState→state_byte` mapping in
render.
- **AT-2 `render_frame_equals_service`:** for every `(DamageState, Axis, x, y)` in the existing
  `app_instances/bridges.rs` test matrix, `compute_bridge_body_shp_frame` output is unchanged vs the
  pre-slice baseline AND equals `bridge_body_frame(state.to_state_byte(axis), x, y)`. (regression: no
  visible frame changes; single-source proven.)

### Slice 3 — `bridge_specs.rs` + `bridge_facts.rs` + `overlay_types.rs` consume the service consts (pure)
Route `pick_destruction_overlay` through `destruction_overlay()`; replace the inline `0x4a/0x63/0xcd/0xe6`
literals in `low_bridge_overlay_damage_step_ra2` with `classify_overlay_byte`/named consts; derive
`high_bridge_stamp_for_overlay` anchor IDs and `is_bridge_overlay_index`/`is_high_bridge_index` band edges
from the service consts.
- **AT-3a `pick_destruction_overlay_unchanged`:** the existing `destruction_overlay_*_known_entries` /
  `_unused_indices_return_none` tests (`bridge_specs.rs:1304+`) still pass byte-for-byte after rerouting.
- **AT-3b `overlay_membership_unchanged`:** `is_bridge_overlay_index` and `is_high_bridge_index` produce
  the SAME bool over the full `0..=255` byte space before vs after deriving edges from the service. (full
  byte-space equality — guards against a derived-edge off-by-one.)
- **AT-3c `bridge_facts_anchor_ids_unchanged`:** `high_bridge_stamp_for_overlay` returns identical
  `(family, dir)` for `0x18,0x19,0xED,0xEE` and `None` elsewhere over `0..=255`.

### Slice 4 — `bridge_topology.rs` imports `TILESET_WINDOW` from the service (pure)
Re-point `BRIDGE_TILESET_WINDOW` to the service const; `is_bridge_tileset`/`is_wood_bridge_tileset` unchanged
behaviorally.
- **AT-4 `tileset_predicates_unchanged`:** the existing `is_bridge_tileset_distinct_from_structural_flag` /
  `is_wood_bridge_tileset_distinct_from_concrete_and_structural` tests pass unchanged. (window width single
  source.)

### Slice 5 — wire the BridgeStrength gate + Scen->Random binding into the live AoE path (STATEFUL, hash-relevant)
Add the bridge-tile damage block to the AoE path: gate on `DestroyableBridges (SpecialFlags & 0x8000)` AND
`warhead.wall`; on a bridge cell, draw `RandomRanged(1, BridgeStrength)` from the **Scenario RandomClass**
(NOT the global RNG) and apply the sequential A/B/C/D block semantics (each block its own draw; Ion bypass +
3 retries; strict `<` compare). Consume `classify_overlay_byte` + the SM classifiers from the service.
- **AT-5a `bridge_strength_gate_strict_less_than`:** with a deterministic seeded Scen RNG, a roll equal to
  `damage` does NOT damage (strict `<`); `roll < damage` damages. (boundary: equality fails.)
- **AT-5b `aoe_bridge_blocks_sequential_not_exclusive`:** a hit satisfying both an A/B SM candidate and a
  C/D direct-overlay range applies BOTH draws (two RNG consumptions), not one. (ordering + non-exclusivity.)
- **AT-5c `ion_bypass_and_retries`:** an IonCannon warhead bypasses the RNG gate and retries the A/B path up
  to 3×; C/D single-shot.
- **AT-5d `rng_instance_is_scenario_not_global`:** assert the BridgeStrength draw advances the Scenario RNG
  stream and leaves `g_GlobalRng`-equivalent stream untouched (replay-determinism / lockstep guard).

### Slice 6 — full LOW tile-variant selector (mask + two RNG draws) on the wood-bridge repair/destroy path (STATEFUL)
Implement `low_destroyed_tile`/`low_healthy_tile` modeling the gamemd inline mask decode and the
SINGLE `g_GlobalRng` variant draw (corrected 2026-06-04: `SelectDestroyedBridgeTile_Low` draws once and
reuses the value for all `%3`/`&1` forms) + the coord-delta stamp; retire the partial
`classify_low_bridge_band`/`pattern_a/b_new_index` helpers. Caller draws from `g_GlobalRng`.
- **AT-6a `low_destroyed_variant_against_trace`:** for a battery of adjacency masks × seeded `g_GlobalRng`
  states, the chosen `rel_tile_index` and `coord_delta` match a captured gamemd trace
  (decompile_function 0x579620 logic; values per mask). (mask-space coverage incl. isolated/fully-connected
  corners.)
- **AT-6b `low_variant_rng_is_global_not_scenario`:** the variant draw advances `g_GlobalRng`, NOT
  `Scen->Random` — the inverse of AT-5d; mixing the two is the lockstep DRIFT (§4.4/§4.5).

(Note: the §4.8 AoE deck-height cutover is tracked by its own in-tree TODO-cutover; it is adjacent to this
family and should be flipped in the same review window as Slice 5, since both touch the AoE bridge path's
hashed output. **CORRECTED 2026-06-04: the cutover is NOT `4→2`.** gamemd's threshold is the FULL deck
(`+4` cell-levels = `ground + 2 × DAT_0089E870` leptons); the Rust bug is the `bridge_height/2` divide in
`select_object_damage_layer`, which makes the threshold the half-deck midpoint. Parity = drop the `/2`
(compare `impact_z > level + bridge_height` with `bridge_height = 4`), NOT setting the constant to `2`. The
`bridge_topology::BRIDGE_DECK_HEIGHT_LEVELS = 2` shadow encodes the refuted `2 ×` premise and must be
re-audited — do not rely on it as the cutover target. See Verification Log entry 10.)

---

## Anchors & Evidence

| Address | Ghidra call cited | Doc cross-ref |
|---|---|---|
| `0x0081CC30` (g_LatinSquare) | read_memory 0x0081CC30 len 64; decompile_function 0x47F6A0 | BRIDGE_DISPLAY_TABLE §2.4/§5 |
| `0x007ED3D0` (CellSpread count) | read_memory 0x007ED3D0 len 64; disassemble_function 0x489280 @ 0x4895A3 | APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW; CELL_REFERENCE_POINTS |
| `0x00ABD490` (CellSpread offset) | disassemble_function 0x489280 @ 0x4895C7 | APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW |
| `0x00AA0E28` (g_BridgeSet) | read_memory 0x00AA0E28 len 64; disassemble_function 0x489280 @ 0x489ECC; decompile_function 0x587180 | BRIDGE_DISPLAY_TABLE §2.4; APPLY_DAMAGE_TO_CELL §routing |
| `0x00ABAD1C` (g_WoodBridgeSet) | read_memory 0x00ABAD1C; disassemble_function 0x489280 @ 0x48A0A8; decompile_function 0x587180 | BRIDGE_DISPLAY_TABLE §2.4 |
| `0x00A83D84` (OverlayTypeClass array) | read_memory 0x00A83D84; disassemble_function 0x489280 @ 0x48961A; decompile_function 0x47F6A0 | OVERLAY_CLASS_SYSTEM |
| `0x00ABAD30` / `0x00AA1028` (bridgehead-class bases) | disassemble_function 0x489280 @ 0x489F27 / 0x489F46; decompile_function 0x587180 | APPLY_DAMAGE_TO_CELL_OVERLAY_FIRST_ROUTING |
| `0x00ABC210` / `0x00ABC2D0` (railing tables) | read_memory 0x00ABC210 len 160 / 0x00ABC2D0 len 80 | BRIDGE_DISPLAY_TABLE §3.4.1 |
| `0x00ABDB64` / `0x00ABDDA4` (LOW coord-delta) | read_memory 0x00ABDB64 / 0x00ABDDA4 len 64; decompile_function 0x579620 | BRIDGE_DISPLAY_TABLE §2.4 |
| `0x47F6A0` (DrawOverlay_Body) | decompile_function 0x47F6A0 | BRIDGE_DISPLAY_TABLE §3.3.1 |
| `0x587180` (ApplyDamageToCell) | decompile_function 0x587180; get_function_callers 0x587180 | APPLY_DAMAGE_TO_CELL_OVERLAY_FIRST_ROUTING |
| `0x489280` (Apply_area_damage) | disassemble_function 0x489280 | APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW; BRIDGE_AOE_LAYER_DAMAGE; WEAPON_AOE_BRIDGE_DAMAGE_ENTRY |
| `0x004863D0` (tile-set membership) | decompile_function 0x004863D0 | BRIDGE_DISPLAY_TABLE §2.5 |
| `0x004865D0` (HasBridgeOverlay, misnamed) | decompile_function 0x004865D0 | BRIDGE_DISPLAY_TABLE §2.7 |
| `0x579620` (SelectDestroyedBridgeTile_Low) | decompile_function 0x579620; get_function_callers 0x579620 | BRIDGE_DISPLAY_TABLE §2.4 |
| `0x57ACF0` (SelectBridgeTileVariant_Low) | get_function_callers 0x57acf0 | BRIDGE_DISPLAY_TABLE §2.3 |
| `0x598030` (Rand_in_range → g_GlobalRng 0xABE890) | decompile_function 0x598030; disassemble_function 0x598030 @ 0x59805E | RNG instance routing |
| `0x00A8B230+0x218` (Scen->Random for BridgeStrength) | disassemble_function 0x489280 @ 0x489FEF/0x48A173/0x48A23F/0x48A299 | APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW |

---

## DRIFT Ledger

| Rust file:line | Current | gamemd-correct | Severity + trigger-frequency |
|---|---|---|---|
| `src/sim/combat/combat_aoe.rs:42, :231` | effective threshold `level + bridge_height/2 = level + 2` levels (HALF deck) | `ground + DAT_0089E864/2 = ground + 2 × per_level = ground + FULL deck (4 levels)`; `DAT_0089E864 = 4 × per_level` (writer `FADD`, NOT `FMUL`) | **MEDIUM** — fires on every AoE detonation landing on/under a high bridge with units on both layers; Rust threshold is HALF too low, routing under-bridge splash to deck list and vice versa. Hashed-state; deferred cutover. **Stage-2 corrective value `2` was WRONG — correct offset is the full deck (4 levels); fix is removing the `/2` divide. The `bridge_topology::BRIDGE_DECK_HEIGHT_LEVELS = 2` shadow is suspect (encodes the refuted `2 ×` premise).** (Corrected 2026-06-04: get_assembly_context 0x00489120; decompile_function 0x487D50.) |
| `src/sim/combat/combat_aoe.rs` (whole AoE path) | no bridge-tile damage block; no `BridgeStrength` draw | gated `DestroyableBridges & warhead.Wall`; per-block `RandomRanged(1,BridgeStrength)` from **Scen->Random**, strict `<`, sequential A/B/C/D, Ion bypass+3 retries | **MEDIUM** — fires whenever a `Wall=yes` warhead hits a bridge; bridges currently take no AoE tile damage, and the RNG instance must be Scenario (not global) or replays desync. |
| `src/sim/bridge_specs.rs:154-315` | partial LOW selector (band classify + next-index step); no adjacency-mask decode, no RNG instance, no coord-delta | full mask-driven inline arithmetic + a SINGLE `g_GlobalRng` variant draw (corrected 2026-06-04: ONE draw reused across forms, not two) + coord-delta stamp (`SelectDestroyedBridgeTile_Low`) | **LOW now / HIGH when wired** — not on a live runtime path yet; becomes player-visible (wrong connector tile) + lockstep-critical (wrong RNG instance) the moment wood-bridge repair/destroy is implemented. |
| `src/sim/bridge_specs.rs:105-106, 173-177` + `src/map/overlay_types.rs:32-47` + `src/map/bridge_facts.rs:114-122` | overlay-byte band edges + HIGH anchor IDs as inline literals in 3 files, no shared const | single named-constant source (`0x4A/0x63/0x64/0x65/0xCD/0xE6/0xE7/0xE8`, `[0x18,0x19,0xED,0xEE]`) | **MEDIUM** — fires on every bridge-damaging hit; a one-site edit silently desyncs the others (no current value drift, but a structural drift hazard). |
| `src/app_instances/bridges.rs:30, 71-84` | render holds its own `BRIDGE_BODY_LATIN_SQUARE` + jitter logic | single sim-owned `latin_jitter`/`bridge_body_frame` consumed by render | **LOW** — fires every frame a healthy bridge is visible; single-consumer today so no divergence yet; a second consumer (minimap/replay preview) would re-derive and could drift. |
| `src/sim/map/bridge_topology.rs:80` | `BRIDGE_TILESET_WINDOW = 0x10` (local) | single service const `TILESET_WINDOW = 0x10` | **LOW** — value-correct; structural single-source consolidation only. |
| `src/sim/bridge_specs.rs:419-445` | four static `DESTRUCTION_OVERLAY_*` tables (gamemd is inline arrays) | representation difference only — values proven equal across full 0..15 index space | **NOT-DRIFT (downgraded with exhaustive proof)** — listed for completeness; relocate to service, do not change values. |

---

## Verification Log (adversarial re-check, 2026-06-04)

Goal: refute the load-bearing claims against gamemd.exe. Default verdict DRIFT/UNVERIFIED unless proven this
session. Each entry: claim → verdict → evidence (Ghidra MCP call cited).

| # | Claim (doc location) | Verdict | Evidence |
|---|---|---|---|
| 1 | `g_LatinSquare @ 0x0081CC30 = {0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2}`, dword stride (§2.1, §4.1) | **VERIFIED** | read_memory 0x0081CC30 len 64 → dwords `0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2`. Rust copy `src/app_instances/bridges.rs:30` byte-identical; index `((ry&3)<<2)|(rx&3)` at `:72`; jitter gated to `Healthy{variant:0}` (state 0/9) at `:71`. |
| 2 | `0x57E7A0`/`0x57DD50` are function prologues (`81 EC CC 00 00 00`), NOT next-overlay data tables (§2, §4.3) | **VERIFIED** | read_memory 0x57E7A0 len 8 = `81 ec cc 00 00 00 b9 e8`; read_memory 0x57DD50 len 8 = identical. Both = `SUB ESP,0xCC` prologue. |
| 3 | LOW overlay range `0x4A..=0x63` → DestroyBridge_Low; HIGH `0xCD..=0xE6` → DestroyBridge_High; inclusive, signed (§4.2, §5.B) | **VERIFIED** | decompile_function 0x587180: `(0x49 < iVar1) && (iVar1 < 100)` and `(0xcc < iVar1) && (iVar1 < 0xe7)`. Also disassemble_function 0x489280 @ 0x48a217-0x48a279 (`CMP 0x4a/JL`, `CMP 0x63/JG`, `CMP 0xcd/JL`, `CMP 0xe6/JG`). |
| 4 | SM classifier `(cell+0x38 − base) + 1`; bridgehead sets `DAT_00abad30..+3` ∪ `DAT_00aa1028..+3`; neighbor `+0x44 ∈ {0x18,0x19}` (high) / `{0xed,0xee}` (low) (§5.B) | **VERIFIED** | decompile_function 0x587180 (`(*(puVar6+0x38) − g_BridgeSet)+1`, the two `..+3` set tests, `0x18/0x19` and `0xed/0xee` neighbor tests); disassemble_function 0x489280 @ 0x489ed8 (`SUB ESI,EBX; INC ESI`), 0x489f27/0x489f46, 0x48a0a8. |
| 5 | AoE gate `SpecialFlags & 0x8000` AND `warhead+0x144` (Wall) (§4.5, §5.C) | **VERIFIED** | disassemble_function 0x489280 @ 0x489eb2 `TEST CH,0x80` (after `MOV EAX,[0x00a8b230]`), @ 0x489ebb `MOV AL,[EBX+0x144]; TEST AL`. |
| 6 | BridgeStrength RNG = `Scen->Random` (`[0x00a8b230] + 0x218`), `RandomRanged(1,BridgeStrength)`, strict `<` (§4.5, §5.C) | **VERIFIED** | disassemble_function 0x489280: `LEA ECX,[EDX+0x218]; PUSH 1; PUSH [EAX+0x1740]; CALL 0x0065c7e0` at 0x489fef, 0x48a173, 0x48a23f, 0x48a299; compare `CMP EAX,[ESP+0x24]; JGE skip` → damages only if roll < damage. Rules+0x1740 = BridgeStrength. |
| 7 | IonCannon (`warhead == Rules+0xFF0`) bypasses RNG gate + 3 retries on A/B; C/D single-shot (§5.C) | **VERIFIED** | disassemble_function 0x489280 @ 0x48a229/0x48a283 `CMP EDX,[EAX+0xff0]; JZ bypass`; @ 0x48a015/0x48a199 `MOV ESI,3 ... DEC ESI` retry loop on A/B; C/D ranges (0x48a214+) have no retry counter. |
| 8 | Sequential blocks A/B/C/D, not mutually exclusive (§5.C) | **VERIFIED** | disassemble_function 0x489280: high-SM (0x489f27+), low-SM (0x48a0a5+), direct-low (0x48a214+), direct-high (0x48a26a+) fall through sequentially; no inter-block early-out (each `JZ` only skips its own block to the next). |
| 9 | AoE layer selector: strict `>`, threshold `ground + (DAT_0089E864 − sign)/2` (§4.8, §5.C) | **VERIFIED (structure)** | disassemble_function 0x489280 @ 0x4895 7a-8d: `MOV EAX,[0x0089e864]; CDQ; SUB EAX,EDX; SAR EAX,1; ADD ECX,EAX; CMP [EDI+8],ECX; JLE ground` → bridge layer iff `impact_z > ground + DAT_0089e864/2` (JLE stays ground = strict `>`). |
| 10 | `DAT_0089E864 = 2 × per_level`, half-deck "1 level (value 2)", Rust `=2` correct (§4.8, ledger) | **WRONG → corrected** | Writer at 0x00489115 is `FADD double ptr [0x007e1738]` (= +0.5 round), NOT `FMUL` (get_assembly_context 0x00489120; read_memory 0x004890f0 byte `dc 05`). So `DAT_0089E864 = round(4 × DAT_0089E870)` = `4 × per_level`. Threshold = `ground + 2 × per_level` = full deck = +4 cell-levels (GetEffectiveHeight `Level+4`, decompile_function 0x487D50). Rust does `level + bridge_height/2 = level+2` (half deck), `combat_aoe.rs:231,:239,:42`. Correct fix = full deck (4), NOT 2; the `/2` divide is the bug. Prior cross-docs `DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY` §3 and `GATE_BRIDGE_DECK_HEIGHT_RESOLUTION` §3 both mis-decoded the byte as FMUL — both are byte-wrong on this identity. |
| 11 | `SelectDestroyedBridgeTile_Low @ 0x579620`: inline mask if/else, variant forms `(uVar4%3)+9/+0xf/+0x17/+5/+0xc/+0x23`, `(uVar4&1)+0x1d`, fixed values; coord-delta `&DAT_00abdda4/+6[iVar6*4]`; tile `g_IsoTile[-4+(DAT_00aa1020+iVar6)*4]`; returns 1 unless `iVar6<1` (§5.D) | **VERIFIED** | decompile_function 0x579620 — all forms and fixed values present; coord-delta and tile-index stamps as cited; `if (iVar6 < 1) return 1`. |
| 12 | "TWO `g_GlobalRng` variant draws" in `SelectDestroyedBridgeTile_Low` (§4.4, §5.D, §6.3, §8 slice 6, ledger) | **WRONG → corrected** | decompile_function 0x579620 — exactly ONE `uVar4 = FUN_00598030()` at function entry; reused for every `%3`/`&1` form. One draw per call, not two. API/slice/ledger corrected to a single roll. (`SelectBridgeTileVariant_Low @ 0x57ACF0` healthy peer NOT re-checked — its draw count is UNCHECKED.) |
| 13 | `FUN_00598030` (Rand_in_range) reads `g_GlobalRng @ 0x00ABE890`, rejection sample; distinct from Scen->Random (§2.3, §4.4) | **VERIFIED** | disassemble_function 0x598030 @ 0x59805e `MOV ECX,0xabe890; CALL 0x0065c780`, rejection loop `CMP EAX,ESI; JA 0x59805e`. 0xABE890 ≠ Scen->Random base 0xa8b230+0x218. |
| 14 | `DAT_007ED3D0` CellSpread count first 12 = `1,9,21,37,61,89,121,161,205,253,309,369` (§2.1) | **VERIFIED** | read_memory 0x007ED3D0 len 64 → dwords `1,9,21,37(0x25),61(0x3d),89(0x59),121(0x79),161(0xa1),205(0xcd),253(0xfd),309(0x135),369(0x171)`. |
| 15 | DESTRUCTION_OVERLAY Rust table values (HIGH_NS, LOW_NS, etc.) exist as quoted, downgraded NOT-DRIFT via cross-doc 0..15 proof (§4.3) | **VERIFIED (Rust side) / UNVERIFIABLE (gamemd inline arrays this pass)** | Read `src/sim/bridge_specs.rs:419-445` — values match §4.3 quote exactly (HIGH_NS `FF,D2,D5,FF,D1,D3,D5,FF,D4,D4,E7,FF×5`; LOW_NS `FF,4F,52,FF,4E,50,52,FF,51,51,64,FF×5`; etc.). The entry-by-entry equality to gamemd's inline arrays in `ApplyBridgeDestruction_*` was NOT re-derived this session (relies on `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.2); the NOT-DRIFT downgrade rests on that cross-doc, treated as UNVERIFIED here. |
| 16 | Theater-populated bases (g_BridgeSet 0xAA0E28, g_WoodBridgeSet 0xABAD1C, DAT_00abad30, DAT_00aa1028, railing/coord-delta tables, DAT_0089E864/0089E870, g_GlobalRng state) read all-zero in static image (§2.2, §5 Open) | **VERIFIED (zero in image)** | read_memory 0x00ABE890 = 0; 0x0089e864 = 0; 0x0089e870 = 0 — all runtime/theater-filled. Absolute values remain UNCHECKED (need live-debugger post-map-load), as stage-2 flagged. |

### Net result
- **VERIFIED:** 13 (entries 1–9, 11, 13, 14, 16).
- **WRONG (corrected in place):** 2 — entry 10 (deck-height identity / §4.8 + ledger row 1: writer is `FADD` ⇒ `4 × per_level`, gamemd threshold = full deck = +4 levels, NOT the stage-2 "value 2"); entry 12 (single g_GlobalRng draw, not two — §4.4/§5.D/§6.3/§8/ledger).
- **UNVERIFIABLE / cross-doc-dependent (left as-is, flagged):** entry 15 (gamemd inline destruction-array equality — relies on a cross-doc not re-derived here); entry 16 absolute runtime base values (static-image zero, need live capture); `SelectBridgeTileVariant_Low` healthy-peer draw count (not re-decompiled).

### Recommendations invalidated for synthesis to down-weight
1. **§4.8 / DRIFT-ledger row 1 corrective value `2` is REFUTED.** Synthesis must NOT adopt
   `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 2`. The gamemd threshold is the FULL deck (`+4` cell-levels =
   `ground + 2 × DAT_0089E870` leptons); the Rust bug is the `bridge_height/2` divide making it half. The
   `sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEVELS = 2` shadow (and its `aoe_object_layer`) encode the
   refuted premise and must be re-audited before any cutover — they are NOT a "proven-correct" shadow.
2. **§6.3 API `low_destroyed_tile(mask, roll_a, roll_b)` is REFUTED** — the function takes ONE roll. The
   "two RNG draws" framing in §4.4, §5.D, §8 Slice 6, and DRIFT-ledger row 3 was corrected to a single
   `g_GlobalRng` draw. A two-draw port would consume the wrong number of RNG values and desync replays.
3. The two-RNG-instance lockstep contract itself (Scen->Random for BridgeStrength vs g_GlobalRng for LOW
   variant) is **UNAFFECTED and VERIFIED** (entries 6, 13) — only the LOW-side draw COUNT changed.
