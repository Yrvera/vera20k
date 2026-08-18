# Repair-Walker Parity Findings (engineer repair + variant RNG + radar/zone gating)

**Facet:** repair-walker. DETERMINISM-SENSITIVE.
**Rust source audited (current):** `src/sim/bridge_state/walker.rs` lines 16, 65-425 (repair entry, 4 walkers, `apply_repair_to_strip_cell`, `repair_transition`, `repair_variant_offset`, `next_rejection_sampled_u8`); `src/sim/bridge_state/mod.rs` `body_cell_repair_state` (1279-1373); `src/sim/rng.rs` (`next_u32`, `next_range_u32_inclusive`); live wiring `src/sim/world/world_orders.rs:374-400`.
**gamemd live-decompiled this session:** `RepairBridge_High@0x0057f440`, `RepairBridge_Low@0x0057f200`, `RepairBridgeWalker_NS_High@0x005800d0`, `RepairBridgeWalker_EW_High@0x00580600`, RNG helper `FUN_00598030@0x00598030` (disasm + decomp), `Random__Next@0x0065c780`, `Math__ftol@0x007c5f00`, scale const `[0x007ed898]=2^-32`, walker loop guard `FUN_00580b70@0x00580b70`, RMG seeder `FUN_00598960@0x00598960`, `MarkBridgesForRepair_Low@0x00578e60` callers.

---

### D1: Healthy-variant RNG uses modulo (low bits); gamemd uses multiply-high (high bits) — DESYNC + wrong tile

- **Rust now:** `repair_variant_offset` (walker.rs:412-414) → `next_rejection_sampled_u8(rng, 3)` (walker.rs:416-425), which returns `draw % 4` (the **low 2 bits** of one `next_u32()`). `accepted = (u32::MAX+1)/4*4 = 2^32`, so the reject branch is dead — exactly one draw, value = `draw & 3`.
- **gamemd:** `FUN_00598030(0,3)` (disasm `0x00598030`): `span = max-min+1 = 4`; loop body = `Random__Next()` once, then `Math__ftol(r * span * 2^-32 + min)`; reject `while (result > max)`. `[0x007ed898]` = `0x3DF0000000100000` = `2^-32`. So `result = floor(r * 4 / 2^32)` = the **high 2 bits** of one draw. Math__ftol@0x007c5f00 = `ROUND(ST0)` (truncate toward zero; product is in `[0,4)` so floor). One draw in the common case; rejection only at the impossible exact-`2^32` boundary.
- **Fixture (seed-pinned, from `rng.rs` test `test_gamemd_raw_sequence_seed_one`):**
  - 1st draw `0x78B76ED5` = 2025459925. Binary: `floor(2025459925*4/2^32)=floor(1.886)=1`. Rust: `0xD5 & 3 = 1`. (coincide)
  - 2nd draw `0x275D74AE` = 660956334. Binary: `floor(660956334*4/2^32)=floor(0.6156)=0`. Rust: `0xAE & 3 = 2`. **DIVERGENT: binary variant 0, Rust variant 2.**
  - The same selected value is written to all 3 strip cells in both (binary: `iVar3=FUN_00598030()+0xcd` then `this_01/local_c4/local_d0 ->Overlay = iVar3`; Rust: `base+variant` to triple). Draw COUNT matches (1 each), but VALUE differs whenever high-2-bits != low-2-bits (~75% of draws).
- **Player sees:** A repaired bridge span shows a different healthy-tile variant than gamemd, AND every bridge repair pulls the shared RNG to a different downstream state. Because the value is identical only ~1/4 of the time, this both diverges the visible tile and (since the draw lands on a synchronized stream) can desync subsequent RNG consumers / world_hash. Fires every engineer bridge repair.
- **Severity:** HIGH (determinism + visible).
- **Confidence:** PROVEN-DRIFT (algebra: `floor(r*4/2^32)` = bits 31..30 of `r`; `r % 4` = bits 1..0 of `r`; not equal in general — fixture above).
- **Verify-call:** `disassemble_function 0x00598030` (FMUL `[0x007ed898]`, CALL `Random__Next`, CALL `Math__ftol`, `CMP EAX,ESI / JA`); `read_memory 0x007ed898 8` → `2^-32`; `decompile_function 0x007c5f00` (ROUND).

---

### D2: Dead/parallel second repair impl `body_cell_repair_state` uses a THIRD RNG algorithm (mask-based) — diverges from both the live path and gamemd

- **Rust now:** `mod.rs::body_cell_repair_state` (1335) draws `rng.next_range_u32(4)` → `next_range_u32_inclusive(0,3)` (rng.rs:131-162). span=3, `mask = u32::MAX >> 3.leading_zeros() = u32::MAX>>30 = 3`; returns `draw & 3` if `<=3` (always, since masked to `0..=3`) → effectively `draw & 3`, ONE draw. It is reached only from `repair_tests.rs`/`mod` unit tests; the live engineer path (`world_orders.rs:381`) calls `repair_bridge_from_engineer_scan` (walker.rs) instead.
- **gamemd:** single repair entry per direction (`RepairBridge_{Low,High}` → 4 walkers), RNG always `FUN_00598030` (multiply-high). There is no second repair function family.
- **Fixture:** `body_cell_repair_state` and `repair_bridge_from_engineer_scan` happen to agree on the RNG result here (both `draw & 3` for span 4 / span-3-mask), so they agree with each other but BOTH disagree with gamemd's high-bits (D1). The risk is dual: (a) two divergent-from-gamemd impls of the same behavior; (b) `body_cell_repair_state` iterates spans/slots and draws per damaged main-deck cell, a structurally different draw schedule than the walker's per-strip-iteration draw.
- **Player sees:** Nothing today (dead on live path). If `body_cell_repair_state` is ever wired in, it would both desync vs gamemd and vs the walker path. Flagged so it is not mistaken for the source of truth.
- **Severity:** LOW (currently dead path; would be HIGH if wired). Triggers only in tests now.
- **Confidence:** PROVEN-DRIFT (vs gamemd RNG, same algebra as D1) / UNCHECKED whether it is intentionally retained.
- **Verify-call:** Grep shows `body_cell_repair_state` callers are tests only; `world_orders.rs:381` is the sole live engineer caller and uses the walker path.

---

### D3: Healthy-variant repair draws on `scenario_rng`; gamemd draws on `g_MapGenRng` (RMG stream), not the main gameplay RNG

- **Rust now:** `world_orders.rs:381` passes `&mut self.scenario_rng` to `repair_bridge_from_engineer_scan` (comment: "scenario stream. Direct field (NOT bridge_rng())").
- **gamemd:** `FUN_00598030` calls `Random__Next` with `ECX = 0x00abe890` = `g_MapGenRng` (label confirmed). `g_MapGenRng` is the Random Map Generator stream, seeded once in `FUN_00598960` ("RMG: Init random map") via `Random__Seed(*(param_1+0x74))` and NOT re-seeded per match; it is a SEPARATE stream from `g_MainRng@0x00886b88` (the main gameplay RNG). After map gen it simply keeps advancing.
- **Fixture:** N/A numerically here — this is a "which stream / what is its starting state" question. For lockstep, what matters is that the Rust `scenario_rng` start-state and advance schedule must equal `g_MapGenRng`'s post-mapgen state on every client. Stock (non-RMG) maps never run the RMG seeder, so on a stock skirmish `g_MapGenRng` is at its default-constructed / last-touched state when the first bridge repair fires — the Rust `scenario_rng` must mirror exactly that, not the main combat stream.
- **Player sees:** No direct visible artifact, but a mismatched starting state or advance schedule between `scenario_rng` and gamemd's `g_MapGenRng` makes the repaired-tile variant (D1) wrong even after D1's algorithm is fixed, and risks lockstep divergence if other systems also touch `scenario_rng` in a different order than gamemd touches `g_MapGenRng`. Every bridge repair.
- **Severity:** MED (determinism foundation; gates whether D1's fix actually matches).
- **Confidence:** LIKELY-DRIFT (stream identity confirmed = `g_MapGenRng`; whether Rust `scenario_rng` is bit-identical to `g_MapGenRng` across init + all interleaved consumers is UNCHECKED and outside this facet's file set).
- **Verify-call:** `disassemble_function 0x00598030` (`MOV ECX,0xabe890` before `CALL 0x0065c780`); `list_globals Rng` → `g_MainRng@0x886b88`, `g_MapGenRng@0xabe890`; `decompile_function 0x00598960` (seeder is RMG).

---

### D4: Radar `MarkTerrainDirty` marks a fixed 3-cell PERPENDICULAR strip at the final-collapse cell; Rust pushes the entire repaired strip (`touched`) when prior overlay was a final byte

- **Rust now:** `apply_repair_to_strip_cell` (walker.rs:365-367): `if matches!(prior_overlay, 0x64|0x65|0xE7|0xE8) { outcome.radar_cells.extend(touched.iter().copied()); }` — pushes the three actually-written cells (`touched`), which for an interior cell are (x,y),(x,y-1),(x,y+1) for NS.
- **gamemd:** in `RepairBridgeWalker_NS_High`, the radar branch fires `if (iVar9 == 0xe7)` (prior overlay == 0xE7) and calls `RadarClass__MarkTerrainDirty` on exactly three coords built from the CURRENT walker cell `&local_fc` and two fixed offsets: `local_d4=0,local_d2=1` via `MapCoord_Add` and `local_cc=0,local_ca=1` via `FUN_00588c60` (the two perpendicular ±1 neighbors). For EW_High the offsets are `{1,0}` (`local_d0=1,local_ce=0`). So the marked cells are (walker cell) + the two perpendicular-axis neighbors — the SAME three cells as `touched` only when the walker cell coincides with the strip center.
- **Fixture:** NS repair of a final-collapse cell at (10,10) with strip = {(10,9),(10,10),(10,11)}. gamemd marks `&local_fc` = the walker iteration cell (X=10 column, Y=`sStack_fa`), plus offsets `(0,+1)` and `(0,+1)`-via-FUN_00588c60. The two helper offsets in the NS body are `{dx=0,dy=1}` for both, i.e. the south neighbor twice via two different coordinate helpers — need byte-exact decode of `MapCoord_Add` vs `FUN_00588c60` sign to confirm one is north. Rust marks the precomputed triple. Likely the SAME 3 cells but the construction differs; the offset-helper sign (`MapCoord_Add` +1 vs `FUN_00588c60` which the destruction walker uses for the OPPOSITE neighbor) is not byte-verified here.
- **Player sees:** Minimap dirty-rect on a bridge-repair-from-final-collapse: if the marked cell set differs by one cell, a 1-cell minimap stale/extra-refresh region. Only when a fully-destroyed (0xE7/0xE8/0x64/0x65) span is engineer-repaired.
- **Severity:** LOW (minimap-only; rare trigger — only repairs of a *fully collapsed* segment).
- **Confidence:** UNCHECKED (the two offset helpers `MapCoord_Add` and `FUN_00588c60` were not byte-decoded for sign; cannot prove the 3-cell sets are identical or off-by-one).
- **Verify-call:** `decompile_function 0x005800d0` (NS_High radar branch under `iVar9==0xe7`), `0x00580600` (EW_High under `iVar8==0xe8`). Follow-up: decode `MapCoord_Add` and `FUN_00588c60` offsets.

---

## PARITY-CONFIRMED (checked, matches gamemd)

- **`repair_transition` overlay table (all 4 families).** walker.rs:383-410 exactly matches the switch in `RepairBridgeWalker_NS_High`/`EW_High` and the LUT decode in the verified doc:
  - HighNs: `0xD1..=0xD5|0xE7 → RandomHealthy{0xCD}`, `0xDF|0xE0 → Fixed(0xDF)`, `0xE1|0xE2 → Fixed(0xE1)`, else NoChange. (binary switch `case 0xd1..0xd5,0xe7 → +0x598030+0xcd`; `0xdf,0xe0 → 0xdf`; `0xe1,0xe2 → 0xe1`; default no-op.)
  - HighEw: `0xDA..=0xDE|0xE8 → {0xD6}`, `0xE3|0xE4 → 0xE3`, `0xE5|0xE6 → 0xE5`. (binary `case 0xda..0xde,0xe8 → +0xd6`; `0xe3,0xe4 → 0xe3`; `0xe5,0xe6 → 0xe5`.)
  - LowNs `{0x4A}`/`0x5C`/`0x5E`, LowEw `{0x53}`/`0x60`/`0x62` — mirror structure (Low dispatcher `RepairBridge_Low@0x0057f200` confirms the NS set `[0x4a..0x52]∪[0x5c..0x5f]∪{0x64}` and EW set `[0x53..0x5b]∪[0x60..0x63]∪{0x65}`).
- **Same RNG value written to all 3 strip cells, one draw per strip iteration.** Binary computes `iVar3 = FUN_00598030()+base` once, writes to `this_01/local_c4/local_d0`. Rust computes `variant` once, writes `base+variant` to the triple. Draw count per case-0 strip = 1 on both.
- **No-change / fixed transitions consume NO RNG draw.** Binary only calls `FUN_00598030` in the case-0 (damaged) branch; Fixed/NoChange paths draw nothing. Rust draws only in the `RandomHealthy` arm. Draw schedule matches.
- **zones_dirty set only on a case-0 (damaged→healthy RNG) repair.** Binary sets `bVar1=true` only in the RNG branch and calls `UpdateBridgeZonesHelper` post-loop iff `bVar1`. Rust sets `outcome.zones_dirty = true` only inside `RepairTransition::RandomHealthy` (walker.rs:341). Match. (Fixed half-damaged normalizations do NOT dirty zones on either side.)
- **Radar dirty gated on prior overlay being the FINAL-destroyed byte.** Binary gate `iVar9==0xe7` (NS) / `iVar8==0xe8` (EW); only the final single-collapse byte triggers `MarkTerrainDirty`. Rust gate `prior_overlay ∈ {0x64,0x65,0xE7,0xE8}` (final bytes for Low NS/EW and High NS/EW). The prior-overlay-is-final gate matches (cell-set off-by-one open in D4).
- **Walker axis classification by overlay byte.** `is_ns_walker_overlay_high/low` and `is_ew_*` (walker.rs:597-619) match the `RepairBridge_High/Low` dispatcher's range tests exactly (NS = `[0xCD..0xD5]∪[0xDF..0xE2]∪{0xE7}`, EW = `[0xD6..0xDE]∪[0xE3..0xE6]∪{0xE8}`; low shifted analog).
- **Walker loop guard range.** `FUN_00580b70` returns 1 iff `0xCD≤+0x44≤0xE8`; `FUN_00580b20` low iff `0x4A≤+0x44≤0x65`. Rust `is_high_repair_overlay`=`0xCD..=0xE8`, `is_low_repair_overlay`=`0x4A..=0x65`. Exact match.
- **Walker iterates ACROSS the span (NS walker iterates X with Y fixed; EW walker iterates Y with X fixed), 3-wide perpendicular strip.** Binary NS_High: index `sStack_fa(Y)*0x200 + local_fc(X)`, `local_fc += 1`, strip at Y±1. Rust `repair_bridge_walker_ns_high` iterates `x`, strip `ns_triple(x,sy)`=(x,sy-1/sy/sy+1). Match. EW analog matches.
- **Pre-walk back-scan to leftmost in-range start cell.** Binary pre-pass `do{ local_fc-=1; ... }while(in range)` then `+1`. Rust back-walks `while cell(x-1) in range { x-=1 }`. Equivalent start cell. (At x=0 / map edge, binary hits the `DAT_00abdc50` out-of-range sentinel → break; Rust guards `x>0` → same result.)
- **`MarkBridgesForRepair_Low@0x00578e60` is map-init, not engineer.** Callers are `FUN_00598960` (RMG) and `FUN_005a1e10` (map-gen). Correctly NOT part of the live engineer repair path; not a Rust gap (map-gen / RMG scope).
- **Outer Low-vs-High selection is per-entry in gamemd, not a unified scan.** gamemd reaches `RepairBridge_Low` vs `RepairBridge_High` via the separate `ProcessBridgeDestruction_{Low,High}` parents; there is no single 5x5 "pick low if any low" scan inside the repair functions. Rust `repair_bridge_from_engineer_scan` does its own low-first scan (walker.rs:65-81). This is a structural difference in WHERE the low/high choice is made but produces the same target walker for a given damaged span; flagged for awareness, not raised as a value-drift (the engineer-entry parent dispatch is a different facet).

## UNCHECKED

- **D4 radar cell-set identity** — `MapCoord_Add` and `FUN_00588c60` offset signs not byte-decoded; cannot confirm whether gamemd marks the exact same 3 cells as Rust's `touched` or differs by one cell.
- **D3 stream bit-identity** — confirmed gamemd uses `g_MapGenRng`; NOT verified that Rust `scenario_rng`'s seed + interleaved advance schedule equals `g_MapGenRng` across map load and all other consumers (cross-system, out of this facet's files).
- **`OverlayTypeClass[+0x2a9]` indirect `+0x11E` reset** — the verified doc's open question (does `RecalcAttributes` reset damage-state `+0x11E` to 0 for these overlay bytes) was not re-verified; Rust models damage_state separately via `apply_damaged_variant_flood_fill`, so the indirect mechanism may or may not be needed for output parity.
- **`FUN_005868a0` / `FUN_00487a10` post-loop helpers** — region relayer + draw/dirty helpers; not decoded. No bridge-state writes per the verified doc, so assumed non-load-bearing for sim state, but their render/relayer effects on objects sitting on the repaired span are unverified.
- **EVA_BridgeRepaired radar event type 14 + EVA gating** — per `BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md` (ECX=0xE before `CreateRadarEvent`, AL gates EVA), this lives in `InfantryClass::PerCellProcess` (engineer-entry), not in the repair-walker bodies; Rust models it at `world_orders.rs:356-372` (`RadarEventType::BridgeRepaired` push gates `eva_allowed`). Not re-decoded this session; appears present in Rust, left UNCHECKED for exact event-type-value (14) and dedup parity.
