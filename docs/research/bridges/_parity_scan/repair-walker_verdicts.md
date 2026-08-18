# Repair-Walker Parity — Adversarial Verdicts

**Facet:** repair-walker (engineer repair + variant RNG + radar/zone gating).
**Auditor stance:** adversarial skeptic; default DRIFT, downgrade only on proof.

## Session constraint (READ FIRST)

**Ghidra MCP was UNAVAILABLE this session.** `list_instances` returned
`{"instances": []}` and `connect_instance gamemd` → TCP `127.0.0.1:8089`
timed out. I could NOT live-decompile `FUN_00598030`, the four
`RepairBridgeWalker_*` bodies, `MapCoord_Add`, or `FUN_00588c60`. Per CLAUDE.md
("doc-only confidence is weaker than live Ghidra") I treat the cited gamemd
facts as **doc-corroborated, not live-reconfirmed**. Where a verdict rests on a
gamemd reading I could only cross-check against verified-Ghidra docs (not the
binary), that limitation is stated inline.

The Rust side I verified DIRECTLY by reading current source:
`src/sim/bridge_state/walker.rs` (1-425, repair path), `src/sim/rng.rs`,
`src/sim/bridge_state/mod.rs:1279-1373`, `src/sim/world/world_orders.rs:350-404`,
and caller greps across `src/`.

Cross-checked gamemd docs (all `[ghidra/verified]`):
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.4–12.7 (FUN_00598030 = Random__Next+Math__ftol; radar 3-cell set; zones gate)
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md` §3.4 (per-walker radar dirty cell set, with per-walker addresses)
- `docs/research/bridges/05-.../REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md` (caller verification: NS_High/EW_High called only from RepairBridge_High)

---

## D1 — Healthy-variant RNG: Rust low bits vs gamemd high bits

**VERDICT = REAL** (gamemd side doc-corroborated, NOT live-reconfirmed this session).

Rust verified directly: `repair_variant_offset` (walker.rs:412-414) →
`next_rejection_sampled_u8(rng, 3)` (walker.rs:416-425). With
`max_inclusive=3`: `span=4`, `accepted = (2^32)/4*4 = 2^32`, so the reject
branch is dead and it returns `(draw % 4)` = **low 2 bits** of one `next_u32()`.
Finder's read of the Rust is correct.

gamemd side: `FUN_00598030` is documented (verified-Ghidra doc §12.5) as
`Random__Next() + Math__ftol()` in a rejection loop with limit 3 — i.e.
`floor(r * 4 * 2^-32)` = **high 2 bits**. That mechanism is multiply-high, not
modulo. Algebra confirms bits 31..30 ≠ bits 1..0 in general (finder's seed-1
fixture: 2nd draw `0x275D74AE` → high-bits 0 vs low-bits 2).

Corrected delta: **Rust `draw % 4` (low 2 bits) → gamemd `floor(draw*4/2^32)`
(high 2 bits) via Random__Next + Math__ftol.** Same 1-draw count (both consume
exactly one `next_u32` in the common case); the *value* differs ~75% of draws,
diverging both the visible healthy-tile variant and downstream RNG-consumer
state on every engineer bridge repair.

Skeptic caveat: I could not live-disassemble `0x00598030` to re-confirm the
FMUL `[0x007ed898]` / ftol / high-bit extraction this session. The verdict
rests on a verified-Ghidra doc plus the finder's algebra, both of which agree.
If a live re-decode of 0x00598030 ever shows a `& 3` / low-bit path, demote to
REFUTED — but nothing in the doc corpus suggests that.

---

## D2 — Dead parallel impl `body_cell_repair_state` uses a third (mask-based) RNG algo

**VERDICT = REAL** (low severity — dead on the live path; confirmed tests-only).

Rust verified directly: `mod.rs::body_cell_repair_state` (1279-1373) draws
`rng.next_range_u32(4)` (mod.rs:1335) → `next_range_u32_inclusive(0,3)`
(rng.rs:139-170). For span 3, `mask = u32::MAX >> 30 = 3`, sample `& 3` is
always `<= 3` → returns `draw & 3` (low bits), ONE draw. This is a THIRD
selection shape (masked-rejection), distinct from both the live walker's
`draw % 4` and gamemd's multiply-high.

Caller grep across `src/` (verified): every `body_cell_repair_state` call site
is in `src/sim/bridge_state/repair_tests.rs` (lines 132,150,168,194,214,229,
243,244,267,338). NO live sim caller. The sole live engineer caller,
`world_orders.rs:381`, calls `repair_bridge_from_engineer_scan` (walker path).
So D2 is correctly characterized as dead-on-live, tests-only.

Nuance worth keeping: for span 4 (`next_range_u32(4)` → inclusive 0..=3) the
mask path here happens to land on the SAME low-bits as the walker's `draw % 4`
(both low 2 bits, one draw) — so D2 and D1's Rust paths agree with each other
but BOTH disagree with gamemd. Not the source of truth; flag retained.

---

## D3 — Repair RNG stream identity: `scenario_rng` vs gamemd `g_MapGenRng`

**VERDICT = UNCERTAIN** (stream-identity claim could not be live-reconfirmed;
bit-identity to `g_MapGenRng` is admittedly UNCHECKED even by the finder).

Rust verified directly: `world_orders.rs:381` passes `&mut self.scenario_rng`
(comment: "scenario stream. Direct field (NOT bridge_rng())"). That part is
real.

gamemd side: finder claims `FUN_00598030` calls `Random__Next` with
`ECX = 0x00abe890 = g_MapGenRng` (the RMG stream), separate from
`g_MainRng@0x00886b88`, seeded only by the random-map generator. I could NOT
live-disassemble `0x00598030` to confirm the `MOV ECX, 0xabe890` this session,
and no verified-Ghidra doc in the corpus pins the *stream global* used by
FUN_00598030 (the docs describe it only as "the game's seeded RNG /
Random__Next", without nailing the `0xabe890` ECX). The finder itself marks the
Rust↔g_MapGenRng bit-identity UNCHECKED.

This is therefore the weakest-confirmed item: the *direction* of the claim (a
stream/seed mismatch could make D1's fix still wrong) is plausible and
determinism-relevant, but neither the `g_MapGenRng` binding nor the Rust
`scenario_rng`-vs-`g_MapGenRng` start-state equivalence is proven here. Marking
UNCERTAIN, not REAL: needs a live `disassemble_function 0x00598030` to confirm
the ECX global, and a cross-system audit of `scenario_rng` seeding/advance
schedule (out of this facet's files) before it can be raised to REAL.

---

## D4 — Radar MarkTerrainDirty cell set on final-collapse repair

**VERDICT = REFUTED** (cell sets are the SAME 3 cells; finder's "possible
1-cell delta" is resolved by the verified-Ghidra radar doc).

Rust verified directly: `apply_repair_to_strip_cell` (walker.rs:365-367) — when
`prior_overlay ∈ {0x64,0x65,0xE7,0xE8}`, pushes `touched` = the three written
strip cells. For an NS walker the strip is `ns_triple(x,sy)` = `(x,sy)`,
`(x,sy-1)`, `(x,sy+1)`. The walker iterates X with Y fixed, so the "main cell"
is `(x,sy)` and the perpendicular neighbors are the y±1 cells in the same
triple.

gamemd side (verified-Ghidra doc, with per-walker addresses):
`BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md` §3.4 states each walker marks
"main cell and its `y-1`/`y+1` neighbors" (NS) or `x-1`/`x+1` (EW), and
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.7 confirms "main cell + both
perpendicular neighbors (3 total)", with §12.7 explicitly noting
`FUN_00588c60` **negates** the coord so the two perp cells are `+offset` and
`−offset` (i.e. both y-1 AND y+1, not the south neighbor twice). That is exactly
Rust's `ns_triple` / `ew_triple`. The finder's worry that the two helper offsets
might both be `{0,+1}` (south twice → off-by-one) is contradicted: the verified
decode shows one helper negates. The marked 3-cell set is identical to Rust's
`touched`.

Caveat: this rests on the verified radar doc, not a live re-decode of the
`MapCoord_Add`/`FUN_00588c60` sign bytes this session. But the doc explicitly
addresses the exact sign question the finder left open and resolves it in the
"same 3 cells" direction. Downgrading UNCHECKED → REFUTED on that basis. (If a
live byte-decode ever shows both helpers emit `+1`, re-open as REAL — not
indicated by any current doc.)

Secondary note (not a delta): Rust also pushes `touched` for the LOW finals
`0x64`/`0x65`, matching the doc's Low NS/EW gates. The prior-overlay-is-final
gate matches on both sides.

---

## NEW disparities the finder may have missed

- **MISS (none material in-facet).** The repair-walker Rust path (walker.rs
  65-425) matches the verified docs on: the 4-family `repair_transition` table,
  the per-strip single draw written to all 3 cells, no-draw on Fixed/NoChange,
  `zones_dirty` only on the RandomHealthy (case-0) arm, the prior-final radar
  gate, axis classification, loop-guard ranges, and the pre-walk back-scan. I
  found no additional value/ordering drift in this facet beyond D1–D4.

- **MISS (process, low-confidence): trace docs are stale on the Rust RNG algo.**
  `ENGINEER_HIGH_BRIDGE_REPAIR_MUTATION_TRACE.md` and the LOW counterpart still
  describe the Rust side as "xorshift64* modulo." Current Rust
  (`rng.rs::next_u32`) is a 250-word XOR-lag stream, not xorshift64*. This does
  NOT change the D1 verdict (the bit-selection drift stands either way) but
  those trace docs should be re-verified before being cited as the Rust
  baseline. Flagged so a future patch pass doesn't trust the stale Rust
  description.

- **MISS (deferred, out of this facet's mutation scope): radar cells are
  dropped, not propagated.** Per `BRIDGE_PRESENTATION_RADAR_DIRTY` §6, Rust
  collects `outcome.radar_cells` correctly (walker.rs:365) but the consumer side
  historically dropped them (no minimap propagation). Current
  `world_orders.rs:393` does call `self.mark_radar_terrain_dirty_cells(...)`, so
  this may now be wired — but whether `mark_radar_terrain_dirty_cells` actually
  reaches the minimap terrain buffer (vs. a no-op sink) was not verified this
  session and is the real player-visible question behind D4's "minimap refresh"
  framing. Not in this facet's mutation files; flagged for the radar/minimap
  facet.
