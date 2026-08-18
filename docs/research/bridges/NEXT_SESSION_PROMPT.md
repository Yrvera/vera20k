# Next-session handoff prompt (bridge parity — fix phase)

Paste the block below to start the new session.

---

Continue the BRIDGE-system parity work (Rust engine vs the original). The research + design phase
is DONE; we are in the gated fix phase. Re-enable ultracode (`/effort ultracode`) and drive the work
with small, reviewed workflows — not big swarms.

START HERE (read first, do not re-derive):
- `docs/research/bridges/BRIDGE_PARITY_IMPLEMENTATION_CONTRACT.md` — THE source of truth: ~47
  adversarially-confirmed parity holes (BR-01..BR-49), each with an exact current→correct delta,
  evidence (fn@addr + verify call), an acceptance test, severity, and a disjoint-ownership fix plan (§7).
- `docs/research/bridges/BRIDGE_PARITY_ARCH_DECISIONS.md` — two policies that govern every fix:
  (A) `overlay_byte` is authoritative, the `DamageState`/`bridgehead_anchor_class` enums are derived;
  (B) the bridge dispatcher is draw-faithful (RNG count/order/stream = spec, never an outcome of a
  Rust short-circuit). Honor both. (Last session, BR-08 failed precisely because it keyed on the
  derived enum instead of the byte — Decision A caught it.)
- Per-hole proof lives in `docs/research/bridges/_parity_scan/<facet>_{findings,verdicts}.md` (32 files);
  read only when you need the full evidence behind a specific hole.

ALREADY LANDED last session (uncommitted on `dev`, all bridge tests green — 398 passed). If not yet
committed, commit them first with a clear message and tick them off in the contract:
- BR-03 outer debris gate `2_040_109_466 → 2_040_109_464`  (bridge_orchestrator.rs)
- BR-04 metallic gate `0x4000_0000 → 0x3FFF_FFFF`  (bridge_orchestrator.rs)
- BR-14 bridge-collapse kill excludes air-layer units  (bridge_orchestrator.rs::kill_ground_occupants_at)
- BR-08 repair re-activates the zone record, keyed on `effective_render_state` not `damage_state`
  (bridge_state/mod.rs::refresh_endpoint_active_flags + world_orders.rs repair wiring + faithful test)
- BR-09 body-SM collapse clears the anchor `overlay_byte = 0xFF`  (bridge_state/mod.rs)
Touched files: src/sim/world/bridge_orchestrator.rs, src/sim/bridge_state/mod.rs,
src/sim/bridge_state/tests.rs, src/sim/world/world_orders.rs.

DO NOT TOUCH — pre-existing failures, NOT ours: `movement_tests.rs::{on_bridge_fires_at_ramp_to_body_only,
multi_crossing_preserves_first_bridge_set_update, ship_high_bridge_ramp_to_body_relinks_after_on_bridge_update}`
fail at HEAD with my changes stashed (verified). They are the parallel session's in-progress on-bridge
ramp→body transition rewrite (BR-26 area). Leave them alone; if a cargo error points at files you didn't
touch, it's not yours.

CHECK AT START: is the Ghidra MCP connected? It dropped at the end of last session. These holes are
BLOCKED until it's back: BR-01/02 (dispatcher fall-through needs BR-19's routing predicate), BR-06/07
(repair RNG bits/stream — need live `disassemble_function 0x00598030`), BR-17 (Z-window lepton bounds),
BR-10/BR-15 Low-family constants, and all `[BSS]` numeric constants. Do the Ghidra-independent holes
first.

NEXT BATCH (Ghidra-independent, cheap, high-value — do these next, smallest first):
- BR-11 — remove the `if matches!(c.role, Bridgehead) { continue; }` guards in the 4 destroy walkers +
  4 cascade leaves in `walker.rs` (gamemd has no role concept; it writes all triple/cascade cells). Fires
  on most collapses (ramps left standing today). Add a test: span end-bridgehead gets the destroyed overlay.
- BR-39 — `walk_anchor_pattern` dir-6 slot 5 should be `anchor+2E` (opposite_cell + offset[2]), not
  `anchor+1E` (which dups slot 4). bridge_state/mod.rs. Tiny; mirror `bridge_facts::stamp_slots`.
- BR-16 — feed minimap `MarkTerrainDirty` from collapse: add a `radar_cells` field to
  `StateOutcome::Collapsed` and push collapsed cells into `sim.radar_terrain_dirty_cells` in the orchestrator.
- BR-13 — route bridge-collapse ground kills through the real C4Warhead death pipeline (wreck/explosion/
  smudge/eject/score) instead of the bespoke `health=0`. Bigger — combat coupling; do with care + a test.

HOW TO RUN A FIX BATCH (the shape that worked last session — it caught a real bug):
- Per batch: ONE implementer agent that OWNS a contention-free file (holes in the same file → same worker;
  bridge_orchestrator.rs and bridge_state/mod.rs are contention points — don't parallel-edit them), plus
  TWO adversarial reviewers (lens 1 = "matches the proven delta vs the binary/verdict"; lens 2 = "over-reach /
  broken-invariant / scope-creep"). Reviewers are read-only.
- DETERMINISM/RNG holes (BR-01/02/05/06/21, debris): ONE serial implementer + a `world_hash` regression
  test that pins the draw schedule — NEVER a swarm.
- cargo build/test runs as a SEPARATE foreground pass AFTER the workflow — never inside it.
- BUDGET/RATE-LIMIT: do NOT run big multi-agent scans (the 16-facet scan cost ~4.5M tokens). Throttle any
  workflow into small waves (≤3-4 concurrent first-requests) with a retry-on-rate-limit wrapper — a
  16-at-once burst tripped a server-side rate limit last session.

SEPARATE TRACK (not a mechanical fix): the CABHUT-C4 "does nothing" bug — root cause is UNIDENTIFIED
(the ally-gate hypothesis was refuted). See contract §4; needs end-to-end diagnosis plus a `/trace-action` of
SEAL-C4-on-Neutral-CABHUT → cursor → order → plant → hut-death → collapse. Don't bundle it with the fixes.
