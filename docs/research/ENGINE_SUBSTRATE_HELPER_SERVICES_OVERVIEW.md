# Engine Substrate Helper Services — Cross-Cutting Program Overview

**Status:** SYNTHESIS / INDEX (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. Parity bar = indistinguishable-from-gamemd on player-observable output.

This indexes seven per-family substrate-service studies into one program view. It does **not** restate
their detail — read the deep docs for contracts, addresses, and slice plans. Its job: per-family readiness,
master-TODO mapping, **shared primitives + single ownership**, **cross-family seams**, a **program-wide
dependency-ordered slice sequence**, the consolidated **blocking research gates**, and **unresolved reviewer
conflicts**. The seven studies and their adversarial verdicts:

> **Pass 2 (2026-06-04): verify-and-expand complete.** All four shared cross-cutting gates are now RESOLVED/VERIFIED
> (see §3.1). Every family ran a live verify+expand pass; all per-family P0s are closed or downgraded to non-blocking.
> **No BLOCKING research gate remains** — authoritative work is unblocked. See §3.1 (shared resolutions), §6 (residual
> non-blocking gates with exact next queries), and §8 (Pass 2 expansion highlights).
>
> **Slice-tail follow-up (2026-06-04): six per-slice gates that were blocking the deferred cell-validation and bridge
> slice tails are now CLOSED with standalone resolution docs — see §6c.** Cell #1 (`IsRectInPlayfield` corner formula),
> Cell #3 (Track-over-Clear passability), Cell #4 (FNPC ring shape), Bridge A1 (deck Z-init — **corrects the prior
> "round(src×4)" claim to `2×per_level`**), Bridge A2 (OnBridge occupancy), Bridge A4 (CheckBridgeTraversal + `+0x144` +
> `+0x1B0`). The affected plan tasks are flipped to READY in their implementation plans; only the non-blocking dummy
> `DAT_00ABDC50` field-values gate stays open.

| Family | Deep doc | Review verdict |
|---|---|---|
| Damage helpers | `DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: ftol order + vet offsets + receiver-gate order VERIFIED; P0 CLOSED) |
| Pathfinding helpers | `PATHFINDING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: heuristic doc-was-wrong/Rust-right; corridor-cost is the real work) |
| Cell validation | `CELL_VALIDATION_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: FNPC=frame-counter not RNG; P6 unblocked; new C22 save-order DRIFT) |
| Bridge helpers | `BRIDGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: Z-init=**2×per_level** [corrected from round(src×4) — see §6c], `+0x144`=Wall not Bridge, tileset/structural co-used) |
| Target scoring | `TARGET_SCORING_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: +0x400=IsOccupied not Sensor; 5-coeff score algebra VERIFIED) |
| Drawing helpers | `DRAWING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: only Layer-2 sorted; sprites painter-order; FIFO tie-break VERIFIED) |
| INI parsing helpers | `INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | GREEN (P2: ReadDouble=f32-narrow VERIFIED; full accessor set promoted to VERIFIED) |

Precedent format: `FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`. Master roadmap:
`docs/plans/2026-05-29-core-engine-substrate-todo.md` (8 systems; mission/radio + shell/object substrate
slices 0–3 already shipped). #1 invariant: `sim/` never depends on `render/ui/audio/net`.

---

## 1. Per-family verdict: readiness + biggest player-visible gap

| Family | Readiness (implemented vs missing) | Single biggest player-visible parity gap |
|---|---|---|
| **Damage** | Apply path exists but is a single-multiply shortcut; armor index, CellSpread table, Verses-gate, building-state gate present. ftol order VERIFIED (`ftol(ftol(lerp)*Verses)`); vet offsets PINNED. Missing: three-`ftol` order, MaxDamage cap, **receiver country-armor is a DIVIDE** (FDIVR) + per-unit ArmorMult (`+0x158`)/FirepowerMult (`+0x160`) — entirely absent in Rust (new DRIFT), ordered immunity gates, heal path, overkill clamp. | **ProneDamage trap** — Rust *applies* a prone modifier gamemd never reads in YR (now VERIFIED dead by exhaustive `+0xF8` byte sweep), dealing 50–70% wrong damage on every prone-infantry hit. |
| **Pathfinding** | A* spine is solid; cell-A* heuristic is genuinely Euclidean and **Rust MATCHES** (P2: doc was wrong, Rust right). Missing: corridor/zone cost model (Rust's centroid-Manhattan injects a distance pull the binary's uniform-cost Dijkstra+zone-base+slope lacks), soft codes 4/5/6 (Rust hard-blocks), bridge-peer-path marker (`cell+0x140 & 0x40000` is a dynamically XOR-toggled bridge marker, not static cliff terrain). | **Corridor-cost model wrong** — the corridor tier is uniform-cost Dijkstra + zone-base + ftol(slope); Rust's centroid-Manhattan picks different routes. Runner-up: a unit ringed by stationary allies returns no-path (codes 4/5/6 should expand at cost, not hard-block). |
| **Cell validation** | Ingredients scattered across `PathGrid`/`OccupancyGrid`/ad-hoc spawn checks; zone matrix + occupancy list-order verified-correct. FNPC selection now VERIFIED = deterministic per-tick frame counter (no RNG). Missing: rect passability/occupancy pair, reservation surface, `Find_Nearby_Passable_Cell`, `Get_CellClass` dummy fallback. | **Wrong nearby-cell choice** — every "place legal nearby" (exit, scatter, chrono-return, paradrop, slave deploy, crate, AI site — **40 callers**, not ~3) uses a coarse walkable grid + non-gamemd ring instead of the diamond-ring FNPC with frame-counter selection. New: `OccupancyGrid::rebuild` orders by entity ID on load; gamemd preserves insertion order → divergence after any savegame (C22). |
| **Bridge** | Predicates exist and are mostly formula-correct but scattered across 4 cell views; occupancy rebuild-layer correct. Tileset `IsBridge`/`IsWoodBridge` (zone/path-snap layer) and structural `0x100` flag (movement/AoE/occupancy) are now VERIFIED **co-used** — modeling only `0x100` is a real gap. Missing: `CheckBridgeTraversal` legality gate in binary shape; separate tileset predicates. | **Traversal legality not modeled** — `compute_bridge_transition` does the render/occupancy layer flip but not the diff-{0,1,4}/bridgehead/parent-fallback gate, so bridge entry/exit pathing is approximated. Now-confirmed DRIFT #6: the service must expose tileset predicates (incl. new `is_wood_bridge_tileset`) separately from the structural flag. |
| **Target scoring** | No score concept at all; ranks `(dist_sq, threat_class, stable_id)` nearest-first. Missing: the whole threat-score pipeline (now VERIFIED = FIVE coefficient doubles per branch + base const `100000.0`, ranking turns on small deltas), ring scan + early return, cadence timer, OpportunityFire passive path, retaliation target-switch (`ShouldRetaliate` is a 2nd score consumer), cloak/bridge/enemy-house gates. | **Wrong target choice** — Rust picks the nearest, gamemd picks the highest integer-threat-score with ring-perimeter scan-order tie-break (not stable-id); visibly different in nearly every fight with ≥2 candidates. |
| **Drawing** | App-layer instance builders + hand-ordered ~40 draw calls; turret pass + sparkles position-correct; AdjustForZ math correct. Missing/drift: Y-sort folds Z in, FLH screen-only, invented SHP-over-VXL tie-break, several DrawExtras decorations. | **Y-sort key drift (D8)** — gamemd sorts Layer-2 by lepton X+Y with elevation excluded; Rust folds `z·HEIGHT_STEP` into the key → draw-order flips on any ramp/hill/bridge, every match. |
| **INI parsing** | `IniFile` raw store + per-`IniSection` typed reads exist; foundation enum + merge order correct. ReadDouble now VERIFIED to narrow through **f32** at parse (`sscanf("%f")` then widen), so generic scalars carry only ~7 sig digits; Verses bypasses ReadDouble (full f64/strtod). Missing: hex (`$xx`/`xxh`) parsing (zero in `rules/`), first-char T/Y/1 bool, `%`-anywhere ×0.01, shared enum-by-name, value-transform accessors (ReadSpeed/ReadRange/ReadColorRGB). | **`PercentAtMax` (and any `%`-suffixed double via `get_f32`)** reads with no `%`→×0.01 → 100× wrong. (`Verses` is NOT affected — full-f64 hand-rolled loop.) New: generic scalars must round-trip through f32 before `×0.01`/`SimFixed` to match gamemd's precision-narrowing; Rust parses straight to f64 today. |

---

## 2. Mapping to master-TODO items

| Master-TODO item | Families that serve it |
|---|---|
| **#5 — combat/projectile/warhead pipeline** | **Damage helpers** (the damage-application math + classifier; explicitly item #5). Target-scoring's Verses-gate touches the warhead edge but is item #6. |
| **#6 — target acquisition / order cadence** | **Target scoring** (score + ring scan + tie-break + cadence; explicitly item #6). Depends on #1's live-object vector for the scan-order tie-break. |
| **#7 — map/cell substrate** | **Cell validation** (item #7's *first slice*; extends `CELLCLASS_SUBSTRATE_FIRST_MIGRATION_SLICE`), **Pathfinding helpers** (consume the cell substrate; the edge-cost/zone/slope/bridge-peer layer), **Bridge helpers** (a bridge-topology read service over the cell substrate). All three live in or over #7. |
| **load-time / `rules/` layer** | **INI parsing helpers** (the load-time data substrate *below* `sim/` that feeds every type parser; not a tick system). |
| **render layer (above `sim/`)** | **Drawing helpers** (render-side draw-order/offset/remap service; consumes a sim snapshot, never feeds back; golden-frame harness replaces the hash step). |
| cross-cutting deps | **#1 native tick spine** supplies the live-object scan order target-scoring (#6) needs for tie-break; **#2 two-RNG-stream** is the gate for cell-validation FNPC frame-counter-vs-RNG selection; **#8 save/load/hash** governs every SNAPSHOT_VERSION bump. |

---

## 3. Shared primitives — single ownership (so two studies don't both claim one)

These primitives are consumed by ≥2 families. Ownership is assigned to **one** owner; the others **borrow**.

| Shared primitive | Owner | Borrowed by | Note |
|---|---|---|---|
| **CellClass validators** (`check_passability_rect` / `check_occupancy_rect` / per-cell passability / `Find_Nearby_Passable_Cell` / `get_cellclass_fallback`) | **Cell validation** | Pathfinding (`Can_Enter_Cell` 0–7 code is the *input* to its edge cost; A* blocked-destination fallback calls the search), Bridge (occupancy reads), production/spawn/scatter | The single most-shared primitive. Pathfinding owns the *graph search*; cell-validation owns the *cell-legality predicate*. Do not duplicate the predicate in A*. |
| **Cell flags + `OnBridge` layer + occupancy lists** (`+0x140` bits, `+0xE4`/`+0xE8`, `+0x124`/`+0x128`, `+0x122` blocker refcount) | **Map/cell substrate (#7)** as the backing store; **Bridge helpers** owns the *bit semantics + layer-select rules* | Cell-validation (occupancy reads `+0xDC`/`+0x44`/`+0x4C`/`+0x11C` + lists), Pathfinding (`+0x122` off-marker exception, `+0xE4`/`+0xE8` code-2 walk), Damage (AoE impact-Z layer select), Drawing (draw-offset) | `+0x122` is the **blocker refcount**, NOT fog (corrected across pathfinding+cell-validation). `BridgeFlags` bit-value constants are owned once by the bridge service; `bridge_facts.rs` routes its reads through them. |
| **Fixed-point math** (`SimFixed`, single `f64`→`SimFixed` pinned conversion, `ftol` truncate-toward-zero) | **`util/fixed_math`** (existing) | Damage (double-`ftol` Verses/falloff), Target scoring (score `ftol` before integer rank), Pathfinding (slope-cost `ftol`), INI (the one `(double)(float)x ×0.01`→`SimFixed` path) | All four families depend on the *same* `ftol`/conversion path. `ftol` truncation **order** is an open gate in 3 of them (see §6). INI owns the only parse→fixed boundary; everyone else converts already-`SimFixed` data. |
| **CCINI typed-accessor surface** (`read_int`/`read_bool`/`read_double`/`read_string`/`enum_by_name`) | **INI parsing** | Every type parser (`warhead_type`, `object_type`, `weapon_type`, `foundation`, …) feeding damage/target-scoring/cell/pathfinding rules | Owned once in `rules/ini_value.rs` + `ini_enum.rs`. Damage's Verses-precision and target-scoring's coefficient parsing both depend on `read_double` precision being pinned (gate S0). |
| **`g_DirectionOffsets` / 8-dir neighbor tables** | **Map/cell substrate (#7)** / `util/direction` | Pathfinding (neighbor lookup, parent reconstruct), Bridge (`CheckBridgeTraversal` parent-`0` reconstruct via `(dir-4)&7`) | Both families read it. NOTE: `0x0089F688` reads all-zero in the cold image (runtime-init) in BOTH studies — re-resolve before it is load-bearing; the operative pathfinding table `0x007e3774` is verified. |
| **`RecalcZoneType` → reduced ZoneType `+0x4C` + zone-passability matrix** | **Pathfinding** (the cascade that writes the value) / shared matrix table | Cell validation (the `required_zone_id` comparison reads the column + matrix) | The matrix (`MOVEMENT_ZONE_PASSABILITY[13][8]`) is byte-verified and shared; pathfinding owns the *cascade*, cell-validation owns the *comparison*. |
| **Fog-of-war / shroud** (object-discovery vs TS darkening) | n/a — **explicitly NOT a shared primitive** | — | Flagged in 5 docs: TS "previously-seen darkening" is OFF by stock-YR default. Acquisition discovery uses object bytes `+0x41A/+0x41B`; cells use shroud-only. No family designs darkening in. |

---

## 3.1 Shared primitive verification (Pass 2 — all four cross-cutting gates RESOLVED)

The four primitives every math/coordinate family depends on were each verified live this pass. Each carries
the answer, its VERIFIED/UNCHECKED status, and which family doc holds the full evidence.

| Shared primitive | Status | Resolution (Rust-native contract) | Detail doc |
|---|---|---|---|
| **`Math__ftol` truncation/rounding order** | **VERIFIED** | `Math__ftol` @ `0x007c5f00` is MSVC `_ftol2`, control-word-driven; CW @ `0x00822d80` = `0x0E7F` → RC bits = **round-toward-zero (truncate)**. == C `(int)` cast == `cvttsd2si`. Per-consumer order pinned (damage three-ftol at result boundaries; threat-score single distance ftol; pathfinding two bridge-anchor ftols; CCINI ReadDouble has none — truncation happens at the downstream consumer). **Rust: model as truncate-toward-zero (`.to_num()`/`.int()`, NEVER `.round()`); keep the multiply chain in the wider fixed type and truncate ONLY at the same sub-step boundaries gamemd does — not per-multiply, not once-at-end.** | Damage + Pathfinding + Target-scoring + INI (`ReadRange`) docs |
| **CCINI `ReadDouble` → SimFixed precision boundary** | **VERIFIED** | ReadDouble @ `0x005283D0` does `sscanf(str,"%f")` (4-byte float, fmt @ `0x00825bd8`) then `(double)(float)v`; `%`-anywhere → `×0.01` in double. So **generic scalars carry only f32 mantissa (~7 sig digits)**. **Verses bypasses ReadDouble entirely** (`0x0075d590`: hand-rolled `strtod`/full-f64 when no `%`, `atoi(token)*0.01` when `%`) → `double[11]` at WarheadType+0xA0. **Rust: generic consumers parse `s.parse::<f32>()` → widen → `×0.01` in f64 → SimFixed (do NOT parse straight to f64/SimFixed); Verses parse full f64 per token, keep without re-narrowing to f32.** | INI + Damage + Target-scoring docs |
| **`g_DirectionOffsets` cold-zero re-resolve** | **VERIFIED (FLH + cell table); Table-2 numerics UNCHECKED** | The "cold-zero" is real BSS lazy-init but splits into THREE tables with different roles: (1) **cell-offset table `0x0089F688`** (runtime-filled by `0x0049F2F0`; 8× `{dx,dy}` **cell units**, 0=N…7=NW CW) — Rust `util/direction.rs DIRECTION_DELTAS` is byte-identical. (2) **lepton-delta tables `0x0089F6D8/0x6DC`** (written by `SubCellDirOffset_Init 0x0049F3B3`; consumed by locomotors only) — numeric contents UNCHECKED (BSS-zero statically), irrelevant to FLH. (3) **FLH/turret/firing uses NO table** — `GetFLH 0x006F3AD0` rotates by a continuous angle; const `0x007E4408` = **-PI/16** = the 32-way facing quantization; Rust `flh_transform.rs` matches the quantization formula + angle constant. | Bridge + Pathfinding + Drawing docs |
| **CCINI typed-accessor surface** (precision feeds damage/scoring/rules) | **VERIFIED** | Folded into the ReadDouble row above. The full DOC-ONLY accessor set (ReadMovementZone, ReadMinMax, ReadCLSID, Read3Int, ReadPoint/Size, ReadRect, ReadLayer, ReadSpeedType, ReadSoundList, ReadAction, ReadGeneral, ReadColorRGB, ReadSpeed, ReadRange) was promoted to VERIFIED live; strtrim thresholds, hex/`%`/bool semantics re-verified. | INI doc |

**Net effect:** all three §6 "shared / cross-cutting" blocking gates are CLOSED. The only residual shared UNCHECKED
items are non-blocking (Table-2 lepton numerics — not read by FLH; last-ULP x87 `float10` in threat-score ranking;
Verses adversarial-decimal strtod bit-parity) — each with an exact next query logged in §6.

---

## 4. Cross-family seams / collision risks

| Seam | Risk | Resolution |
|---|---|---|
| **Pathfinding ↔ Cell validation** | Both could implement the cell-legality predicate; A* edge cost is indexed by the `Can_Enter_Cell` 0–7 code. | Cell-validation **owns** the predicate (produces the code); pathfinding **consumes** it for edge cost. The `Can_Enter_Cell` seam is named in both §6.0 (pathfinding) and §6.6 (cell-validation). Land cell-validation's `check_passability_rect` first; pathfinding routes through it. |
| **Bridge ↔ Cell validation** | Both read cell flags / occupancy lists / `OnBridge`; bridge has its own `CheckBridgeTraversal`, cell-validation has `check_cell_passability` (which selects `+0x124`/`+0x128` by bridge flag + height). | Bridge owns **traversal legality + layer-select rules**; cell-validation owns the **rect passability/occupancy predicate** and consumes the bridge bit-semantics constants. The `+0x124`/`+0x128` occupation-byte selection is shared logic — define it once in the bridge service, call it from cell-validation's per-cell check. |
| **Damage ↔ Bridge (AoE layer)** | Both touch the AoE impact-Z layer selector (`impact_z > ground_z + BridgeHeight/2`): damage's `Apply_area_damage` picks ground vs deck list; bridge's `aoe_object_layer` owns the threshold. | Bridge **owns** `aoe_object_layer` (the C12 threshold). Damage's AoE distribution **calls** it once per detonation, then runs the per-target kernel. Both blocked on the same `DAT_0089E864` (BridgeHeight) init gate. |
| **Drawing's independence from `sim/`** | Drawing is the only render-layer family; risk is a `sim/`→`render/` back-edge or a blitter port. | Drawing is purely **downstream** — consumes a frozen sim snapshot, never feeds deterministic state; render offset for bridge lives behind a `render/`-facing trait so `sim/` has no `render/` dep. No blitter port (GPU depth pipeline stays). Golden-frame harness, not SNAPSHOT_VERSION, gates it. |
| **Target scoring ↔ tick spine (#1)** | The equal-score tie-break must match the native live-object scan order; Rust's `entities.values()` is stable-id order, which gamemd is NOT. | Target-scoring's scanner takes a `live_order: &[u64]` supplied by #1's live-object vector. **Blocked on #1** for replay-correct tie-break; surface, don't hardcode stable-id. |
| **All four math families ↔ `ftol` order** | Damage, target-scoring, pathfinding-slope, and drawing-pip-offsets each need the exact `ftol` truncation **order**, which the decompiler collapses. | **RESOLVED (Pass 2, §3.1):** `Math__ftol` = `_ftol2` truncate-toward-zero (CW `0x0E7F`); per-consumer ftol boundaries pinned per family. Reproduce as truncate-toward-zero at the same sub-step boundaries. No longer blocks any of the four. |

---

## 5. Recommended program-wide slice sequence (dependency-ordered)

Each family keeps its own P0…Pn internal slices (in its deep doc). This is the **cross-family landing order** —
what must exist before what. Tagged **[hash]** where it changes hashed sim state (needs shadow-first + SNAPSHOT_VERSION bump);
**[render]** = golden-frame harness, not hashed; **[load]** = load-time, indirect hash via stats.

**Pass 2 note:** the ordering is unchanged — the dependency DAG didn't reorder — but every slice's research gate is now
CLOSED, so each can start on engineering schedule rather than waiting on a Ghidra answer. The two precision contracts
(ftol truncate-toward-zero; ReadDouble f32-narrow / Verses f64) are now FIXED inputs, not open gates.

1. **INI typed-accessor service** [load] — `read_int`/`read_bool`/`read_double`/`enum_by_name` + value-transform accessors (ReadSpeed/ReadRange/ReadColorRGB) + the corpus-equivalence harness. **First**: damage Verses-precision and target-scoring coefficient parsing both depend on it. **S0 now VERIFIED** (ReadDouble = f32-narrow; Verses = f64) — the contract is pinned, so percent/precision consumers can flip; the remaining work is the Rust f32→`×0.01`→SimFixed conversion + bit-identity test, not Ghidra. *Shadow = assert == old accessor on full stock corpus.*
2. **Cell-validation facade** [hash at the FNPC flip only] — `CellValidator` borrow-only facade, `get_cellclass_fallback`, `check_passability_rect`, `check_occupancy_rect`. **Second**: it is master-TODO #7's first slice and the shared predicate pathfinding + bridge + spawn all consume. P1–P5 are hash-neutral (read-only); the FNPC authority flip bumps SNAPSHOT_VERSION but is now **unblocked** — FNPC selection is VERIFIED = deterministic per-tick frame counter `0x00A8ED84` (no RNG), so the #2 two-RNG concern is a non-issue for FNPC. **Slice-tail gates now CLOSED (§6c):** FNPC ring shape/order (T4/T7 — 3 Rust reconciles), `IsRectInPlayfield` corner formula (T3.5 — inclusive 4-corner diamond, contradicts Rust rect-bounds), Track-over-Clear passability (T2/T6). **New scope: C22 save/load occupancy-order DRIFT** (`OccupancyGrid::rebuild` orders by entity ID; gamemd serializes insertion order verbatim) — must land in this facade.
3. **Bridge-topology service** [mostly hash-neutral] — `BridgeFlags` + **seven** predicates (added `is_wood_bridge_tileset`) + `aoe_object_layer` + the binary-shaped `check_bridge_traversal`. **Third**: defines the bit-semantics constants and layer-select rules cell-validation's per-cell check and damage's AoE both call. **Slice-tail gates now CLOSED (§6c):** deck Z-init VERIFIED = `2×per_level` leptons (≈208) — **CORRECTS the earlier `round(src×4)` claim**; the deck-height const is the resolved int `2×per_level`, NOT `4` levels (reconcile the Rust `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS=4` against this at P4 cutover); A2 occupancy two-layer + Clear-vs-Mark asymmetry verified (P5); A4 `CheckBridgeTraversal` table + `+0x1B0` slot + `+0x144`=Wall verified (P3). **DRIFT #6 confirmed**: must expose tileset predicates (zone/path-snap layer) separately from the structural `0x100` flag. Predicate consolidation is hash-neutral; occupancy/transition re-order (P5) touches hash.
4. **Pathfinding-support service** [hash at edge-cost/corridor flips] — `edge_g_cost` (one owner), soft codes 4/5/6, the bridge-peer-path marker subsystem (`cell+0x140 & 0x40000` is XOR-toggled, gated by `cell+0x124`; H4/H5/R6 are one coupled bridge subsystem), marker gate authoritative + retire `expand_corridor`. **Fourth**: consumes #2's predicate and #3's bridge flags. **P3 reframed**: the cell-A* heuristic is NOT a fix (Rust Euclidean MATCHES) — the real work is swapping the corridor's centroid-Manhattan for the binary's uniform-cost Dijkstra + zone-base + `ftol(slope)`. Slope-context writer (`Foot+0x21c`) + search-ctx `+0x01` bridge flag still UNCHECKED (non-blocking; queries logged).
5. **Damage math service** [hash] — kernel (three-`ftol` + MaxDamage; ftol order VERIFIED), **retire ProneDamage first** (now VERIFIED dead — sharpest correctness fix, ship early), receiver gates (order VERIFIED), overkill clamp + state classify, Verses→`SimFixed` (S0 pinned), **country-armor DIVIDE + per-unit ArmorMult `+0x158`/FirepowerMult `+0x160`** (new DRIFT, entirely missing), AoE layer-select via #3. Authoritative flip bumps SNAPSHOT_VERSION. **P0 CLOSED.**
6. **Target-scoring service** [hash] — score terms (FIVE coefficients per branch, base `100000.0` — algebra VERIFIED), candidate gate, ring-perimeter scanner + strictly-greater selection + early return, `ShouldRetaliate` retaliation target-switch, cadence timer. **After** cell-validation/pathfinding (shares the cell substrate); equal-score tie-break is ring-perimeter scan order (NOT stable-id), still **gated on tick-spine #1** for replay-correct cell-occupant insertion order.
7. **Drawing-helper service** [render] — Y-sort key fix (X+Y, Z excluded, with per-class bias D8a), **sort ONLY Layer-2** (layers 0/1/3/4 append unsorted; Air-layer aircraft/missiles/jumpjet draw in submission order — D8b), FIFO equal-key tie-break, sprite-vs-sprite occlusion is painter-order only (z-buffer occludes sprites against terrain/cliffs only, never reorders two sprites), FLH world-coord path, decoration placement, remap result table. **Last / parallel**: purely downstream of `sim/`; ftol/insert-tie-break gates CLOSED. Golden-frame harness, no SNAPSHOT_VERSION bump.

Rationale: data (1) → cell substrate (2) → bridge semantics over it (3) → pathfinding consuming both (4) →
damage math (5) → target choice over the substrate (6) → render of the result (7). 1–4 unblock 5–6; 7 is
independent. Every hashed flip follows shadow → invert → drop asserts → authoritative → SNAPSHOT_VERSION bump → parity harness.

---

## 6. Research gates — Pass 2 status

**No BLOCKING gate remains.** All seven families' P0s are CLOSED or downgraded to non-blocking. Authoritative
slice work is unblocked. Section 6a lists what Pass 2 RESOLVED (with the answer + which family doc holds the
evidence); 6b lists the residual non-blocking items, each with its exact next query.

### 6a. RESOLVED in Pass 2

**Shared / cross-cutting (all CLOSED — see §3.1 for full contracts):**
- **`Math__ftol` truncation order** → VERIFIED truncate-toward-zero (CW `0x0E7F`, `_ftol2`); per-consumer boundaries pinned. *Docs: Damage / Pathfinding / Target-scoring / INI.*
- **`read_double` precision** → VERIFIED f32-narrow at parse; Verses bypasses ReadDouble (full f64). *Doc: INI (corroborated in Damage / Target-scoring).*
- **`g_DirectionOffsets` cold-zero** → VERIFIED three distinct tables; cell table = `util/direction.rs` byte-match; FLH uses NO table (continuous angle, const `0x007E4408` = -PI/16). *Docs: Bridge / Pathfinding / Drawing.*

**Cell validation:** FNPC selection = deterministic per-tick frame counter `0x00A8ED84` (NO RNG) — P6 unblocked. RTTI-0x24 = TerrainClass. Save/load cell-list order serialized verbatim (`+0xE4`/`+0xE8` + `+0x30` swizzle), zone `+0x4C` re-derived. *Doc: CELL_VALIDATION.*

**Damage:** ProneDamage-DEAD VERIFIED by exhaustive `+0xF8` byte sweep. three-`ftol` order VERIFIED (`ftol(ftol(lerp)*Verses)`). VeteranArmor = Rules+0x688 (FDIV), VeteranCombat = Rules+0x670 (FMUL); ability bytes pinned (defender ARMOR `type+0x29d/0x2af`, attacker FIREPOWER `type+0x29e/0x2b0`). Receiver immunity gate order + Fire_At attacker order VERIFIED. MaxDamage = Rules+0x16C8 per-target. *Doc: DAMAGE.*

**Bridge:** Z-init `DAT_0089E864`/`00B1D0AC`/`00AC13BC` = **`2 × per_level`** (the `×4 then ×0.5` idiom = `×2`; const `0x007E1738` = 0.5) — **CORRECTED from the earlier "round(src×4)" claim** per `GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`; three copies of one deck-height constant (≈208 leptons = 2 levels). The `+4` is a SEPARATE Level-unit pathfinding seed, not the coord-Z. Warhead `+0x144` = **`Wall=`** (not Bridge); `+0x145`=WallAbsoluteDestroyer, `+0x148`=Tiberium. Tileset `IsBridge`/`IsWoodBridge` vs structural `0x100` = VERIFIED **co-used** (DRIFT #6 real). vtable `+0x1B0` = `CheckBridgeTraversal 0x004D9C60` in the Foot/Unit/Infantry vtables (not CellClass, not Aircraft/Building). OnBridge occupancy = two layers (list-by-`+0x8C`, bit-by-Z) + Clear-vs-Mark `0x100` asymmetry (`GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`). *Doc: BRIDGE + the three slice-tail gate-resolution docs (§6c).*

**Target scoring:** `+0x400` = **IsOccupied** (not Is_Sensor); BuildingClass override @ 0x00458DD0. Score = FIVE coefficient doubles per branch (Rules+0x1068.. / Type+0x2C8..), base const `100000.0`. ftol order VERIFIED. Scan-order tie source = ring-perimeter square walk × intra-cell occupant list × array index (NOT stable-id). `Rules+0xF48` = OccupyWeaponRange. *Doc: TARGET_SCORING.*

**Drawing:** ONLY Layer-2 is depth-sorted (`Submit_Object` passes `sorted=(layer==2)`); equal-key tie-break = stable FIFO (`0x00551A90`); 2nd DrawExtras pass replays buffer order. `0xC0` remap-blitter decision flags pinned (`0x00490B90`). Building `-0x80` = half-cell leptons on X and Y, Z untouched, feeds sort. Normal opaque SHP never touches `g_ZBuffer` → sprite-vs-sprite is painter-order only. *Doc: DRAWING.*

**INI:** ReadDouble (S0 binary side) fully pinned. Full DOC-ONLY accessor set promoted to VERIFIED live (ReadMovementZone/MinMax/CLSID/Read3Int/Point/Size/Rect/Layer/SpeedType/SoundList/Action/General/ColorRGB/Speed/Range). 128-char buffer = per-accessor caps (smallest = 32). hex/`%`/bool semantics re-verified. *Doc: INI_PARSING.*

### 6b. Still OPEN (all non-blocking) — with exact next query

- **Threat-score last-ULP ranking** (Target-scoring): x87 `float10` intermediates can differ in the last ULP vs an f64 port for the returned comparison score. *Next: `emulate_function 0x0070CD10` on two near-equal candidates, diff ordering vs f64 reimpl — only if a ranking tie ever breaks observably.*
- **Verses adversarial-decimal bit-parity** (INI): `FUN_007d151e` (strtod) vs Rust `parse::<f64>()` for boundary strings. *Next: boundary-spanning bit-identical test on tokens like `0.1`, `33.33` — only if Verses bit-parity is required.* Also: ApplyWarheadDamage's internal Verses-as-stored-double-vs-narrow read not decompiled — *next: `decompile_function` on the damage-calc reading `WarheadType+0xA0+armor*8`.*
- **MaxDamage-vs-INI enumeration** (Damage): whether any stock-YR path exceeds Rules+0x16C8. *Next: enumerate INI Damage/Warhead values against the cap; D8 `param_2[0x382]==5` field name still a paraphrase.*
- **Slope-context writer + bridge-aware flag** (Pathfinding): `Foot+0x21c` slope-context block writer; search-ctx `+0x01` bridge flag lifecycle. *Next: `get_field_access_context` on `Foot+0x21c` / ZoneMap allocator; trace `+0x01` set/clear at A* entry/exit. Also: corridor min-heap insertion-order tie equivalence (H13) and `UpdateHierarchicalEdges` retry loop unmodeled.*
- **Lepton-delta Table-2 numerics** (shared): BSS-zero statically; not read by FLH. *Next: `decompile_function 0x0049F3B3` — only if locomotor sub-cell deltas are wanted.*
- **Drawing bridge-body `0xC0` intensity-remap pixel result** vs GPU substitute — a design choice + golden-image test, not a binary fact.
- ~~**Cell `IsRectInPlayfield 0x00578390`** 4-corner formula~~ → **RESOLVED** in `GATE_PLAYFIELD_RECT_RESOLUTION_GHIDRA_REPORT.md` (see §6c). Residual: dummy `DAT_00ABDC50` runtime-init field values stays open (non-blocking; gates cell-validation T1 only if a human-path caller reads OOB-dummy fields). *Next: `read_memory 0x00ABDC50` at runtime ONLY if such a caller is found.*
- **Target-scoring equal-score replay parity** + **Cell C22 save-order** both resolve only after master-TODO #1's native live-object vector lands (Rust cell-occupant insertion order must match gamemd `cell+0xE8` chaining).

### 6c. RESOLVED by the slice-tail gate-resolution docs (2026-06-04)

These six per-slice gates that were blocking the deferred cell-validation and bridge slice tails are now CLOSED with binary evidence; each has a standalone resolution doc. The affected plan tasks are flipped to READY in their plans. (Gate names use the cell-validation / bridge plans' local numbering.)

| Gate | Resolution doc | Verdict | One-line resolved fact | Unblocks |
|---|---|---|---|---|
| **Cell #1 — `IsRectInPlayfield` corner formula** | `GATE_PLAYFIELD_RECT_RESOLUTION_GHIDRA_REPORT.md` | CLOSED (only bound-field human-names YELLOW) | Four corners NW/NE/SW/SE with **inclusive** `x+w-1`/`y+h-1`, each judged by `Is_Cell_In_Playfield 0x00578460` as an **isometric diamond** (sum `sx+sy` in half-open band `(low,high]`, strict `(sx-sy)<RIGHT`, `(sy-sx)<LEFT`) against MapClass bound fields — **NOT** a `0<=x<512` rect test. 0-size rect evaluates corners at decremented `(x-1,y-1)` coords (not a no-op). | cell-validation **T3.5** + the un-`#[ignore]`'d 0-size-rect test. CONTRADICTS current Rust rect-bounds. |
| **Cell #3 — SpeedType/passability (Track over Clear)** | `GATE_SPEEDTYPE_MATRIX_RESOLUTION_GHIDRA_REPORT.md` | CLOSED-PASS (one structural DRIFT flagged) | `(Clear, Track)` PASSES — land-speed table `0x0089EA40[LandType*9+SpeedType]`, `[Clear(0)] Track(1)=1.0`; `Can_Enter_Cell 0x0073F0A0` impassable only on `==0.0`. | cell-validation **T0 #3 / T2 / T6**. DRIFT: `passability.rs::is_passable_for_speed_type` uses the wrong 13×8 zone matrix + drops per-terrain multipliers — separate speed/cost follow-up. |
| **Cell #4 — FNPC ring shape & ordering** | `GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` | CLOSED | `Find_Nearby_Passable_Cell 0x0056DC20` scans concentric DIAMOND rings r=0..min(Speed+Sight,32)-1, per-ring N/S apex rows then W/E columns, cap 24, per-ring "direct-found→finish ring→stop" early-out; pick = `g_FrameCounter [0x00a8ed84] % pool.len()` (NOT RNG) or nearest sqrt; direct/indirect = `FUN_006d6410` height-projection identity. | cell-validation **T7/T8** authority flip. CONTRADICTS Rust on 3 points: row-major vs 4-segment order; cardinal-axis vs height-identity `direct`; missing per-ring early-out. Label trap: real counter is `0x00a8ed84`, not `0x00887324`. |
| **Bridge A1 — deck height / GetGroundHeight Z-init** | `GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md` | CLOSED | Coordinate-Z deck = `GetGroundHeight(Coord) + DAT_00AC13BC`, where `DAT_00AC13BC = 2 × per_level` leptons (`×4 then ×0.5`=`×2`), nominally 208. **CONTRADICTS** the prior "round(src×4)" framing; the `+4` is a SEPARATE Level-unit pathfinding seed, never the coord-Z. | bridge **P4** (deck-height const), tightens **P0b** domain. CONTRADICTS the plan's `BRIDGE_DECK_HEIGHT=4`/`BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS=4` (binary full-deck = 2 levels). |
| **Bridge A2 — OnBridge occupancy representation** | `GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md` | CLOSED (a/b/c) | Two independent per-cell layers: object-LIST (`+0xE4`/`+0xE8`) selected by occupant `+0x8C` OnBridge byte, occupancy-BIT (`+0x124`/`+0x128`) selected independently by Z-height; crossing = remove-old-OnBridge → write-new-OnBridge (`dst.Level==src.Level-4 && Flags&0x100`) → add-new-OnBridge; Clear clears by Z alone (no `0x100` re-check). | bridge **P5** (HASH-RELEVANT) and consequently **P6** (DropIn relayer). |
| **Bridge A4 — CheckBridgeTraversal + warhead `+0x144` + vtable `+0x1B0`** | `GATE_BRIDGE_TRAVERSAL_RESOLUTION_GHIDRA_REPORT.md` | CLOSED (all three) | `CheckBridgeTraversal 0x004D9C60` = ground-unit traversal validator in vtable slot `+0x1B0` (Foot/Unit/Infantry, NOT Aircraft/Building); §1.3 decision table verified (`(dir-4)&7` parent reconstruct, dir==-1 candidate seed, directed `*height==-1` parent seed + bridgehead-required, diff∈{0,1,4}, `bridge_entered` set only on ascend E4b); warhead `+0x144 = Wall=` (bool, default false). | bridge **P3** (traversal gate; relocates an already-correct Rust gate) + the `+0x144`/`+0x1B0` inventory entries. |

**Note:** every CLOSED gate here flips (or feeds) hashed sim state when implemented — the READY tasks must follow shadow → invert → drop asserts → authoritative → `SNAPSHOT_VERSION` bump → parity harness. Cell-validation **T7** and bridge **P5** (and possibly **P6**) are the hash-flipping tasks; bridge **P3/P4** are bit-identical relocations contingent on P0b's `cell.level`-vs-`GetGroundHeight` equality.

---

## 7. Reviewer conflicts — Pass 2 dispositions

Nearly all Pass-1 conflicts are now resolved (see §6a for the verified answers). Residual carry-forward only:

- **Damage:** all Pass-1 conflicts RESOLVED. New finds (country-armor DIVIDE, per-unit `+0x158`/`+0x160` mults, 2nd kernel call site, area-damage 18 consumers, D18 yellow=integer `Strength>>1`) are now contract items, not conflicts. Residual: MaxDamage-vs-INI enumeration; D8 field-name paraphrase (both non-blocking).
- **Pathfinding:** heuristic conflict RESOLVED in the **opposite** direction — the doc was wrong, Rust's Euclidean is right; the work is the corridor-cost swap. Codes 4/5/6 SOFT confirmed. Residual UNCHECKED (both non-blocking, queries in §6b): slope-context writer `Foot+0x21c`; search-ctx `+0x01` bridge flag; corridor min-heap tie equivalence (H13); `UpdateHierarchicalEdges` retry loop.
- **Cell validation:** FNPC source RESOLVED (frame counter, no RNG); save/load order, RTTI-0x24, all RESOLVED. **`IsRectInPlayfield` corner formula now RESOLVED** (§6c — inclusive 4-corner isometric diamond, contradicts Rust rect-bounds); FNPC ring shape/order RESOLVED (§6c, 3 reconciles for the Rust shadow). Residual: dummy field values (non-blocking).
- **Bridge:** all four Pass-2 gates RESOLVED (Z-init, `+0x144`=Wall, tileset/structural co-used, vtable `+0x1B0`). **Slice-tail gates A1/A2/A4 now have standalone resolution docs (§6c):** A1 deck-Z = `2×per_level` leptons (CONTRADICTS "round(src×4)" and the plan's `=4`-levels const); A2 two-layer occupancy + Clear-vs-Mark asymmetry verified (P5 ready); A4 CheckBridgeTraversal decision table + `+0x1B0` slot + `+0x144`=Wall verified (P3 ready). Residual: `BridgeStrength=1500` INI re-read (trivial); A1's `cell.level`-vs-`GetGroundHeight` ramp equality (P0b); C18/render shadow-DX before P7.
- **Target scoring:** all four P0s RESOLVED (ftol, +0x400=IsOccupied, scan-order=ring-perimeter, 5-coeff algebra); `0x00445F00` resolved to BuildingClass +0x3C4. Residual: last-ULP ranking parity (non-blocking, gated on #1).
- **Drawing:** ftol, Layer-2 FIFO tie-break, `-0x80` shift all RESOLVED. D8a per-class YSort bias is a confirmed DRIFT carried into the contract. Residual: bridge-body `0xC0` intensity-remap pixel result = a design choice + golden-image test, not a binary fact.
- **INI:** S0 binary side RESOLVED (f32-narrow); full accessor set promoted to VERIFIED. Residual: the Rust f32→`×0.01`→SimFixed conversion bit-identity is an unwritten engineering test (not a Ghidra gate); P5 corpus buffer-truncation (smallest cap = 32) UNCHECKED.

---

## 8. Pass 2 expansion highlights — most material NEW items

Beyond gate closures, the verify+expand pass surfaced these (each in its family doc):

- **Damage — country-armor is a DIVIDE, plus per-unit multipliers:** receiver armor folds in `TechnoClass+0x158` (ArmorMultiplier) as an FDIVR; attacker FirePower folds `TechnoClass+0x160` (FirepowerMultiplier) — **both entirely missing from Rust (new DRIFT)**. Also: a 2nd kernel call site (Psychedelic/MC path, NULL warhead → 0 HP); `Apply_area_damage` has **18 consumers** (superweapons/anims/terrain/per-cell), owns bridge/wall/overlay/tiberium destruction + rocking.
- **Cell validation — FNPC has 40 callers**, not ~3 (rally, scatter, chrono warp, paradrop, slave deploy, crate, start positions, AI convoy). New **C22 DRIFT**: load-time occupancy rebuild orders by entity ID vs gamemd's verbatim insertion order — observable after any savegame.
- **Pathfinding — `cell+0x140 & 0x40000` is a dynamic XOR-toggled bridge-peer marker** (set by `UpdateBridgePassability 0x0042acf0`, gated by `cell+0x124`), NOT static cliff terrain; H4/H5/R6 are one coupled bridge subsystem. New top entry `Find_Path 0x004d3920` (all three locomotors); corridor uses a binary min-heap (not FIFO).
- **Bridge — `IsWoodBridge 0x00486770`** is a second, never-listed tileset predicate; `IsLowBridgeCell` feeds cursor hit-test (`What_Action_OnCell`); tileset `IsBridge` feeds `UnitClass::TurretAI`.
- **Target scoring — `ShouldRetaliate 0x007087C0` is a second direct score consumer** (retaliation keeps current target if it out-scores the attacker → new contract C25); base score const `100000.0` dominates, so ranking turns on small coefficient deltas; distance is cells (scanner) vs leptons (explicit-coord) — new DRIFT note.
- **Drawing — normal opaque SHP sprites never write `g_ZBuffer`** → sprite-vs-sprite occlusion is painter's-order only (z-tested remap is bridge-body/terrain-adjacent); Air-layer objects (aircraft, in-flight missiles, jumpjet) draw in submission order, not depth-sorted (D8b).
- **INI — three value-transform accessors uncatalogued in Pass 1:** `ReadColorRGB 0x00474B50` ([u8;3]), `ReadSpeed 0x00474810` (`(v<<8)/100` trunc→clamp255), `ReadRange 0x00474620` (the INI instance of the project-wide ftol-truncate gate).

---

*End of overview. This is a navigational index over seven deep studies. The program lands data → cell substrate →
bridge semantics → pathfinding → damage → target choice → render; INI (1) and cell-validation (2) come first because
the other five build on them. Pass 2 closed all four shared gates and every family P0 — no BLOCKING research gate remains;
residual items (§6b) are non-blocking with logged next queries. No TS-legacy path is designed into any service (ProneDamage
now VERIFIED dead, VeinholeMonster, tube/subterranean, fog darkening, FoggedObject, BounceClass, the bridge death-list, the
CRC-collision shadow all flagged DEAD/DORMANT). No `sim/`→`render/` dependency, no blitter port.*
