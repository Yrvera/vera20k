# Bridge Walker-Variant RNG (`mapgen_rng`) Implementation Plan — Faithful Shape

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Route the bridge **repair** walker's "healthy" overlay-variant pick off the
gameplay RNG and onto a dedicated, never-seeded third stream (`mapgen_rng`) that mirrors
gamemd's unseeded `g_MapGenRng` — so fixed-map repairs yield variant 0 deterministically
and never advance the scenario/main cursors.

**Architecture:** Adds a third `SimRng` field to `Simulation` beside `scenario_rng` /
`main_rng`, constructed in a genuine all-zero state (not seeded), folded into the world
hash and the snapshot layout (version bump), and consumed only by the repair-variant pick.
Pure `sim/`; no render/ui/audio/net dependency.

**Contract:** `docs/contracts/2026-05-29-bridge-walker-variant-mapgen-rng-implementation-contract.md`
(Shape decision LOCKED to the **Faithful model** — option 2.)

---

## Grounding Summary

- **Binary (all VERIFIED this session via Ghidra MCP):** `FUN_00598030` hardcodes the RNG
  instance with `MOV ECX,0x00ABE890` (`g_MapGenRng`) at `0x0059805E` immediately before
  `CALL 0x0065C780` (`Random__Next`); it is a **float-scaled** rejection draw
  (`FMUL` normalizer `0x007ED898`, `ftol` `0x007C5F00`, `JA` reject `> high`) — *not* the
  integer helper `RandomRanged 0x0065C7E0`. `g_MapGenRng` has exactly **one** write xref
  (`0x0059899B` in `FUN_00598960`, the random-map generator) and is **never seeded at game
  start**. Live callers include all four `RepairBridgeWalker_{NS,EW}_{Low,High}`
  (`0x0057F6A0 / 0x0057FBC0 / 0x005800D0 / 0x00580600`). ⇒ On a fixed-map skirmish the
  instance is all-zero BSS, `Random__Next` returns 0 forever, so `FUN_00598030` → `low`
  ⇒ every repaired main-deck cell becomes **variant 0** (the base).
- **Current Rust (VERIFIED via Read):** the repair walker draws a random `0..3`
  (`next_rejection_sampled_u8`, `walker.rs:416-425`) from `scenario_rng`. The stream is
  passed at the production call site **`src/sim/world/world_orders.rs:381`**
  (`bs.repair_bridge_from_engineer_scan(&scan, &mut self.scenario_rng, terrain)`) — a direct
  field borrow, **not** via `bridge_rng()` and **not** in `bridge_orchestrator.rs` (those
  rng uses are bridge collapse/debris). The whole walker chain is generic over
  `rng: &mut SimRng`, so re-routing is a **one-line call-site change**.
- **Repo pattern this mirrors:** the same-day two-stream split (`scenario_rng` + `main_rng`,
  commit `795fdd4`) is the exact precedent — struct field + dual construction in
  `with_seed`, fold in `world_hash.rs`, `SNAPSHOT_VERSION` bump, `rng_routing_tests.rs`.
  We extend that pattern by one stream.
- **Zero-state construction:** `SimRng::new(0)` is **NOT** all-zero — `reseed` mixes
  `entry_index` through `INIT_TABLE_1/2` (`rng.rs:73-93`), producing non-zero words even for
  seed 0. The faithful shape therefore needs a new `SimRng::zeroed()` that bypasses `reseed`.
- **INI keys:** none — this is RNG plumbing; no INI constant drives it.
- **Still unknown after grounding:** exact random-map seed + draw-consumption order and the
  `FUN_00598030` float-scale math — **deferred** (Blocked row; only matters for random maps,
  not a current target).

## Key Technical Decisions

- **Add a third stream `mapgen_rng: SimRng`, zero-state, never seeded.** — Mirrors gamemd's
  unseeded `g_MapGenRng` exactly; fixed-map repairs draw 0 ⇒ variant 0. **Confidence:** high.
  **Source:** Ghidra `disassemble_function 0x00598030` + `get_xrefs_to 0x00ABE890` (sole write
  in map-gen); repo pattern `src/sim/world/mod.rs:288-302`.
- **New `SimRng::zeroed()` constructor (all fields zero), do NOT reuse `SimRng::new(0)`.** —
  `new(0)` runs `reseed`, which yields non-zero state. **Confidence:** high. **Source:**
  `src/sim/rng.rs:73-93` (read this session).
- **Keep `next_rejection_sampled_u8` (integer `%4`) unchanged; do NOT port the float-scale
  draw.** — On zero-state both math paths yield 0, so the fixed-map observable is identical;
  the float math only matters for the Blocked random-map case. **Confidence:** high.
  **Source:** contract "Required Rust Changes" + Ghidra draw-math verification.
- **Re-route only the call site at `world_orders.rs:381` (scenario → mapgen); leave the
  walker chain and `bridge_rng()` untouched.** — The walker is stream-agnostic; `bridge_rng()`
  serves collapse/debris which correctly stay on `scenario_rng`. **Confidence:** high.
  **Source:** grounding read of `world_orders.rs:376-384`, `mod.rs:531`, `bridge_orchestrator.rs`.
- **`mapgen_rng` declared after `main_rng`; folded into the hash after `main_rng`; snapshot
  `SNAPSHOT_VERSION` 12 → 13.** — Fixed, documented order is the hash/wire contract.
  **Confidence:** high. **Source:** `world_hash.rs:36-46`, `snapshot.rs:15-19`.
- **`zeroed()` sets `index_b: 0` (literal zero BSS), not `RNG_INDEX_B_SEED`.** — Output is 0
  for any index values when every state word is 0, so this is observably irrelevant; choosing
  all-zero is the most faithful reading of "unseeded BSS" and is internal-only.
  **Confidence:** high (output-invariant proven by `next_u32` body, `rng.rs:106-126`).

## Open Questions

### Resolved During Planning

- *Field names — `scenario_rng`/`main_rng` or `rng`/`scen_rng`?* → `scenario_rng` / `main_rng`
  (the two-stream split landed with these names; `mod.rs:288-302`).
- *Is the repair-walker RNG passed via `bridge_rng()`?* → No; direct field at
  `world_orders.rs:381` (borrow-checker forbids the method there).
- *Does `SimRng::new(0)` give zero-state?* → No; needs `zeroed()` (`rng.rs:73-93`).
- *Current `SNAPSHOT_VERSION`?* → 12 (`snapshot.rs:19`).

### Deferred to Implementation

- *Does any existing test pin a **literal** `state_hash()` value?* — Folding `mapgen_rng`
  changes the hash. The save/load round-trip test compares hash-to-self (safe), but the full
  sim test sweep in Task 9 will surface any literal-pin test that must be regenerated.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/rng.rs` | Add `zeroed()` zero-state constructor + unit test |
| Modify | `src/sim/world/mod.rs` | Add `mapgen_rng` field; construct zeroed in `with_seed`; reset in `reseed_both` |
| Modify | `src/sim/world/world_hash.rs` | Fold `mapgen_rng` into the state hash after `main_rng` |
| Modify | `src/sim/snapshot.rs` | Bump `SNAPSHOT_VERSION` 12 → 13 |
| Modify | `src/sim/world/world_orders.rs` | Re-route repair walker from `scenario_rng` to `mapgen_rng` (one line + comment) |
| Modify | `src/sim/world/mod.rs` (`bridge_rng` comment) | Drop now-stale "repair" word from the collapse/debris accessor comment |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` | Tighten overlay assertion to exact base; assert scenario/main untouched |
| Modify | `src/sim/world/rng_routing_tests.rs` | Add routing-isolation + three-stream round-trip tests |

## Interface Changes

- **`SimRng::zeroed() -> Self`** (new public associated fn, `src/sim/rng.rs`). No existing
  caller; only `with_seed` / `reseed_both` will call it. Non-breaking (additive).
- **`Simulation.mapgen_rng: SimRng`** (new `pub(crate)` field). Additive; changes the bincode
  positional layout (handled by the `SNAPSHOT_VERSION` bump) and the world-hash contract
  (handled by the fixed-order fold). No public API removed or renamed.

## Sim Checklist

- [x] No `fixed`-point math added — RNG is `u32`; no `f32`/`f64` introduced.
- [x] New state (`mapgen_rng`) folded into the deterministic state hash (`world_hash.rs`).
- [x] No dependency on render/ui/sidebar/audio/net — pure `sim/`.
- [x] Tick ordering unchanged — repair still runs in its existing phase; only the *stream*
      it borrows changes.
- [x] `BTreeMap` iteration order irrelevant to this change.

## Risk Areas

- **State-hash output changes for every game.** Folding `mapgen_rng` (a third `SimRng`'s
  fields) into `state_hash()` changes the hash even before any reroute. Any test/fixture that
  pins a **literal** hash value will mismatch and must be regenerated. Task 9's full sim sweep
  catches these. (Self-comparing round-trip tests are unaffected.)
- **Snapshot version 12 → 13 rejects old v12 saves.** Intended — a v12 blob has no third
  stream and would mis-deserialize. The existing version guard (`snapshot.rs:110-119,123-132`)
  enforces it.
- **Existing repair test under-constrains the variant.** `engineer_enters_cabhut_repairs_bridge`
  asserts an overlay *range* (`0xCD..=0xD0`) that currently passes with a random variant. After
  the fix the variant is always 0, so the range still passes but no longer proves variant 0 —
  Task 7 tightens it to `== 0xCD` so a regression to random variants would fail.
- **Bincode field position.** `mapgen_rng` must be added in a fixed, documented position
  (after `main_rng`); the version bump makes the layout change explicit and safe.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `mapgen_rng` zero-state ⇒ `next_u32()==0` forever | gamemd's unseeded `g_MapGenRng` returns 0; any non-zero state yields a wrong variant on fixed maps. Fires on every fixed-map bridge repair. | Unit test draws >250 times, all 0; Ghidra `disassemble_function 0x00598030` (zero-BSS → `low`) |
| Task 5 | Repair-variant draws `mapgen_rng`, **never** `scenario_rng`/`main_rng` | gamemd repair does not consume the gameplay stream; the current cursor-corruption is a **HIGH** lockstep/replay bug (first fixed-map repair desyncs every later scenario draw). | Snapshot scenario/main state before+after repair, assert unchanged (Task 8) |
| Task 5/7 | Fixed-map repaired overlay = base + 0 (`0x4E→0x4A`, `0x57→0x53`, `0xD1→0xCD`, `0xDA→0xD6`) | Player-visible "uniform vs varied deck tiles" must match gamemd's uniform variant-0 result on fixed maps. | Integration test asserts exact base byte after repair (Task 7) |
| Task 3 | World-hash fold order (`scenario`, `main`, `mapgen`) fixed forever | Reordering changes `state_hash()` for all clients ⇒ false desyncs. | Code review + Task 8 hash-sensitivity test |
| **Deferred (Blocked)** | `FUN_00598030` float-scale draw + random-map seed/consumption order | Exact variant parity on **random maps** needs this; out of scope (random maps not a target). NOT reproduced here. | `/re-investigate FUN_00598960` if/when random maps land |

---

## Tasks

### Task 1: Add `SimRng::zeroed()` zero-state constructor

**Why:** Foundation — the faithful shape needs a genuinely all-zero `SimRng` (gamemd's
unseeded `g_MapGenRng`). `SimRng::new(0)` runs `reseed` and is non-zero, so a dedicated
constructor is required before the field can be added.

**Files:**
- Modify: `src/sim/rng.rs` (add fn after `new`, ~`rng.rs:34`; add test in the existing
  `#[cfg(test)] mod tests`, ~`rng.rs:173`)

**Pattern:** Mirrors the field initialization in `new` (`rng.rs:23-33`) but skips `reseed`.

**Step 1: Add the constructor** (insert immediately after the closing `}` of `new`, ~line 34):
```rust
    /// Create an RNG in the all-zero "unseeded" state.
    ///
    /// Mirrors gamemd's `g_MapGenRng`, a `RandomClass` instance that lives in
    /// zero-initialized BSS and is never seeded on a non-random map. With every
    /// state word zero, `next_u32` returns `state[a] ^ state[b] == 0` on every
    /// draw regardless of the index positions, so the stream yields 0 forever
    /// until (and unless) it is explicitly reseeded. Do NOT reuse `new(0)`: that
    /// runs `reseed`, which mixes a non-zero table into the state.
    pub fn zeroed() -> Self {
        Self {
            disabled: 0,
            index_a: 0,
            index_b: 0,
            state: vec![0; RNG_TABLE_LEN],
        }
    }
```

**Step 2: Add the test** (inside `#[cfg(test)] mod tests`):
```rust
    #[test]
    fn zeroed_stream_returns_zero_forever() {
        let mut rng = SimRng::zeroed();
        // Draw past one full table wrap (RNG_TABLE_LEN = 250) to cover the
        // index advance/wrap path, not just the first few draws.
        for _ in 0..600 {
            assert_eq!(rng.next_u32(), 0, "unseeded zero-state must draw 0");
        }
        // A fresh zeroed stream must be byte-identical to another (deterministic).
        assert_eq!(SimRng::zeroed().state(), SimRng::zeroed().state());
    }
```

**Step 3: Verify**
Run: `cargo test -p <sim-crate> zeroed_stream_returns_zero_forever -- --nocapture`
(use the crate that owns `src/sim/`; if unsure, `cargo test zeroed_stream_returns_zero_forever`).
Expected: PASS.

**Step 4: Commit** — `sim/rng: add SimRng::zeroed() unseeded zero-state constructor`

---

### Task 2: Add the `mapgen_rng` field to `Simulation` and construct it zero-state

**Why:** Introduce the stream the repair-variant pick will consume. Must be built zero-state
in every constructor/reseed path so a fresh or reseeded sim matches gamemd's unseeded
instance.

**Files:**
- Modify: `src/sim/world/mod.rs` — field decl (~`mod.rs:300`, after `main_rng`),
  `with_seed` init (~`mod.rs:471`), `reseed_both` (~`mod.rs:545-549`)

**Pattern:** Mirrors the `scenario_rng` / `main_rng` declarations and their `with_seed` /
`reseed_both` construction (the two-stream precedent).

**Step 1: Declare the field.** Insert immediately after the `main_rng` field
(after `pub(crate) main_rng: SimRng,`, ~`mod.rs:300`, before the `seed` field):
```rust
    /// Map-generator RNG — gamemd `g_MapGenRng` (0x00ABE890). On a non-random map
    /// this `RandomClass` is never seeded, so it stays all-zero and returns 0 on
    /// every draw. The bridge **repair** walker-variant pick consumes this stream;
    /// on a fixed map it therefore always yields variant 0 (the base overlay), and
    /// the scenario/main cursors are never advanced by a repair. Constructed via
    /// `SimRng::zeroed()` (NOT seeded). MUST be serialized + hashed like the other
    /// two streams. Seeding this for random maps is a deferred (Blocked) follow-up.
    pub(crate) mapgen_rng: SimRng,
```

**Step 2: Initialize in `with_seed`.** In the struct literal inside `with_seed`
(~`mod.rs:471`, immediately after `main_rng: SimRng::new(seed),`):
```rust
            main_rng: SimRng::new(seed),
            mapgen_rng: SimRng::zeroed(),
```

**Step 3: Reset in `reseed_both`.** In `reseed_both` (`mod.rs:545-549`), add the zero-state
reset so a reseeded sim is identical to a fresh one (keep it unseeded — do NOT seed it):
```rust
    pub(crate) fn reseed_both(&mut self, seed: u64) {
        self.scenario_rng = SimRng::new(seed);
        self.main_rng = SimRng::new(seed);
        // mapgen_rng mirrors gamemd's unseeded g_MapGenRng — reset to zero-state,
        // never seeded from the gameplay seed.
        self.mapgen_rng = SimRng::zeroed();
        self.seed = seed;
    }
```

**Step 4: Verify it compiles**
Run: `cargo check`
Expected: compiles. (Any errors in files you did not touch may be a parallel session — do
not fix them; confirm your edited files are clean.)

**Step 5: Commit** — `sim/world: add zero-state mapgen_rng stream to Simulation`

---

### Task 3: Fold `mapgen_rng` into the world state hash

**Why:** A divergence in `mapgen_rng` must be visible to desync detection; the fold order is
part of the hash contract and must be fixed.

**Files:**
- Modify: `src/sim/world/world_hash.rs` (~line 44, after the `main_rng` fold)

**Pattern:** Mirrors the existing `scenario_rng` / `main_rng` folds (fixed-order, append-only).

**Step 1: Insert the fold.** Immediately after `self.main_rng.hash_state(&mut hasher);` and
before `self.next_stable_entity_id.hash(&mut hasher);`:
```rust
        self.scenario_rng.hash_state(&mut hasher);
        self.main_rng.hash_state(&mut hasher);
        // mapgen_rng (gamemd g_MapGenRng): appended AFTER the two gameplay streams.
        // This order is part of the hash contract and must never change.
        self.mapgen_rng.hash_state(&mut hasher);
        self.next_stable_entity_id.hash(&mut hasher);
```

**Step 2: Verify it compiles**
Run: `cargo check`
Expected: compiles.

**Step 3: Commit** — `sim/world: fold mapgen_rng into state hash (fixed order, after main_rng)`

---

### Task 4: Bump `SNAPSHOT_VERSION` 12 → 13

**Why:** Adding `mapgen_rng` changes the positional bincode layout of `Simulation`; old v12
saves must be rejected, not mis-deserialized.

**Files:**
- Modify: `src/sim/snapshot.rs:15-19`

**Pattern:** Mirrors the 11 → 12 bump done for the two-stream split.

**Step 1: Edit the constant + comment.**
```rust
/// Bump this when the snapshot binary format changes in a breaking way.
// Bumped 12 -> 13 for the third RNG stream: `mapgen_rng` (zero-state g_MapGenRng
// mirror) is now a field of `Simulation`, changing the positional bincode layout.
// Old v12 blobs (two streams only) must be rejected, not mis-deserialized.
const SNAPSHOT_VERSION: u32 = 13;
```
(Serialization itself is automatic — `Simulation` derives `Serialize`/`Deserialize` and
`SimRng` does too; no per-field serialize line exists or is needed.)

**Step 2: Verify it compiles**
Run: `cargo check`
Expected: compiles.

**Step 3: Commit** — `sim/snapshot: bump SNAPSHOT_VERSION 12 -> 13 for mapgen_rng`

---

### Task 5: Re-route the repair walker from `scenario_rng` to `mapgen_rng`

**Why:** The actual behavior fix — the repair-variant pick must draw the map-gen stream
(zero-state ⇒ variant 0) and leave the scenario/main cursors untouched. The walker chain is
already generic over `&mut SimRng`, so only the production call site changes.

**Files:**
- Modify: `src/sim/world/world_orders.rs:376-384` (the borrow passed into
  `repair_bridge_from_engineer_scan`)
- Modify: `src/sim/world/mod.rs:531` (drop the now-stale "repair" word from the `bridge_rng`
  doc-comment — `bridge_rng` serves collapse/debris only after this change)

**Pattern:** Mirrors the existing direct-field borrow at this call site (`&mut self.scenario_rng`
is taken directly, not via an accessor, because `bs`/`terrain` hold live disjoint borrows of
`self`). No `mapgen_rng()` accessor is added — it would be unusable here for the same
borrow-conflict reason and would be dead code.

**Step 1: Change the stream passed at `world_orders.rs:381`.**
```rust
            let outcome = if let (Some(bs), Some(terrain)) =
                (self.bridge_state.as_mut(), self.resolved_terrain.as_ref())
            {
                // bridge repair walker-variant pick — gamemd draws g_MapGenRng, not the
                // scenario stream. Direct field (NOT bridge_rng(); `bs`/`terrain` hold live
                // disjoint borrows). On a fixed map mapgen_rng is zero-state => variant 0,
                // and the scenario/main cursors are left untouched.
                bs.repair_bridge_from_engineer_scan(&scan, &mut self.mapgen_rng, terrain)
            } else {
                crate::sim::bridge_state::RepairOutcome::default()
            };
```

**Step 2: Fix the `bridge_rng` doc-comment at `mod.rs:531`** so it no longer claims to serve
repair (it now serves only collapse/debris/explosion):
```rust
    pub(crate) fn bridge_rng(&mut self) -> &mut SimRng { &mut self.scenario_rng } // bridge collapse/debris/explosion
```

**Step 3: Verify it compiles**
Run: `cargo check`
Expected: compiles.

**Step 4: Commit** — `sim/world: route bridge repair-variant pick to mapgen_rng (g_MapGenRng)`

---

### Task 6: Unit-test the walker variant pick is stream-agnostic and zero on zero-state

**Why:** Prove at the unit level that `repair_variant_offset` returns 0 for a zero-state
stream (acceptance test 1, variant axis) and a deterministic value for a seeded stream
(proves the walker is stream-agnostic — it draws whatever stream it's handed).

**Files:**
- Modify: `src/sim/bridge_state/walker.rs` — add to the existing `#[cfg(test)] mod tests`
  (if none exists in this file, add one; the repair fixtures live in
  `src/sim/bridge_state/repair_tests.rs` and `tests.rs` — match their import style)

**Pattern:** Mirrors the rng-state assertions in `bridge_state/repair_tests.rs:191-202`.

**Step 1: Add the unit test.**
```rust
#[cfg(test)]
mod mapgen_variant_tests {
    use super::*;
    use crate::sim::rng::SimRng;

    #[test]
    fn repair_variant_is_zero_on_zero_state_stream() {
        let mut rng = SimRng::zeroed();
        // Zero-state stream draws 0 => variant 0 (gamemd fixed-map result).
        assert_eq!(BridgeRuntimeState::repair_variant_offset(&mut rng), 0);
        // And it must not have advanced into anything non-zero (still zero-state).
        assert_eq!(rng.state(), SimRng::zeroed().state());
    }

    #[test]
    fn repair_variant_consumes_whatever_stream_it_is_handed() {
        // The walker is stream-agnostic: a seeded stream advances and yields a
        // variant in [0, REPAIR_VARIANT_LIMIT_INCLUSIVE].
        let mut rng = SimRng::new(0x1234_5678);
        let before = rng.state();
        let variant = BridgeRuntimeState::repair_variant_offset(&mut rng);
        assert!(variant <= REPAIR_VARIANT_LIMIT_INCLUSIVE);
        assert_ne!(rng.state(), before, "seeded stream must advance");
    }
}
```
(If `repair_variant_offset` / `REPAIR_VARIANT_LIMIT_INCLUSIVE` are private to
`BridgeRuntimeState`, this in-file test module can still reach them via `super::*`. Adjust the
type path if the impl type is named differently — confirm against `walker.rs:412`.)

**Step 2: Verify**
Run: `cargo test repair_variant_is_zero_on_zero_state_stream`
and `cargo test repair_variant_consumes_whatever_stream`
Expected: PASS.

**Step 3: Commit** — `sim/bridge: unit-test repair-variant pick (zero-state => variant 0)`

---

### Task 7: Tighten the end-to-end repair test to the exact variant-0 overlay

**Why:** Acceptance test 1 (variant axis) end-to-end: a real engineer→CABHUT repair on a
fixed map must produce exactly the base overlay byte (variant 0), not just "somewhere in the
healthy range." Also prove the scenario/main cursors are untouched (acceptance test 2 +
parity-critical item).

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs` — the
  `engineer_enters_cabhut_repairs_bridge` test (~line 503, assertions ~524-536)

**Pattern:** Mirrors the existing overlay-band assertion; tightens the range to an equality
and adds the rng-unchanged snapshot pattern from `rng_routing_tests.rs:108-138`.

**Step 1: Snapshot the gameplay streams before stepping.** Just before the engineer
`step()`/run that triggers the repair (find the line that advances the sim), capture:
```rust
        let scenario_before = sim.scenario_rng.state();
        let main_before = sim.main_rng.state();
```

**Step 2: Tighten the per-cell overlay assertion** (replace the `(0xCD..=0xD0).contains`
range check for the high-bridge case with an exact base-byte equality; the contract base for
NS-High repaired-from-damaged is `0xCD`):
```rust
        let bs = sim.bridge_state.as_ref().unwrap();
        for &(rx, ry) in ENGINEER_REPAIR_STRIP_CELLS {
            let cell = bs.cell(rx, ry).unwrap();
            assert_eq!(
                cell.overlay_byte, 0xCD,
                "cell ({rx},{ry}) overlay={:#04X} must be repaired to base+0 (variant 0) \
                 on a fixed map (mapgen_rng zero-state)",
                cell.overlay_byte
            );
            assert!(matches!(cell.damage_state, DamageState::Destroyed));
            assert!(bs.is_bridge_walkable(rx, ry));
        }
```
(If `ENGINEER_REPAIR_STRIP_CELLS` covers a low-bridge fixture instead, use `0x4A`; confirm the
fixture's band against `walker.rs:383-408` and the contract's
`0x4E→0x4A / 0x57→0x53 / 0xD1→0xCD / 0xDA→0xD6` map. If the fixture mixes bands, assert each
cell against its own base.)

**Step 3: Assert the gameplay streams were untouched** (after the repair):
```rust
        assert_eq!(
            sim.scenario_rng.state(), scenario_before,
            "bridge repair must NOT advance the scenario stream"
        );
        assert_eq!(
            sim.main_rng.state(), main_before,
            "bridge repair must NOT advance the main stream"
        );
```

**Step 4: Verify**
Run: `cargo test engineer_enters_cabhut_repairs_bridge -- --nocapture`
Expected: PASS (overlay is exactly the base; both gameplay streams unchanged).

**Step 5: Commit** — `sim/world: tighten bridge-repair test to variant-0 overlay + rng-unchanged`

---

### Task 8: Add Simulation-level routing-isolation and three-stream tests

**Why:** Acceptance tests 3, 4, 5 — prove the repair consumes only `mapgen_rng`, that a
zero-state stream yields variant 0 while a forced non-zero stream advances and varies, that
two fresh sims are bit-identical, and that all three streams round-trip through a snapshot
with the new version guard.

**Files:**
- Modify: `src/sim/world/rng_routing_tests.rs` — add tests beside the existing
  two-stream ones (`:108-138` isolation pattern, `:226` round-trip pattern)

**Pattern:** Mirrors `snapshot_round_trip_persists_both_streams` (`rng_routing_tests.rs:226`)
and the stream-isolation `state()`-before/after pattern (`:108-138`).

**Step 1: Routing isolation (acceptance test 3).** Use the same engineer→CABHUT repair
fixture helper this module/`world_orders_bridge_repair_tests.rs` uses to build a destroyed
fixed-map bridge and trigger a repair (factor a shared helper if needed):
```rust
    #[test]
    fn repair_variant_advances_only_mapgen_stream() {
        let mut sim = /* destroyed-bridge + engineer fixture, fixed map */;
        let scenario_before = sim.scenario_rng.state();
        let main_before = sim.main_rng.state();
        let mapgen_before = sim.mapgen_rng.state();

        /* trigger the engineer repair (step the sim until the walker runs) */;

        assert_eq!(sim.scenario_rng.state(), scenario_before, "scenario untouched");
        assert_eq!(sim.main_rng.state(), main_before, "main untouched");
        // Zero-state mapgen draws 0 (rejection-sampled accept on first draw), so it
        // DOES advance index_a/index_b even though every drawn word is 0; assert it
        // moved (proves the repair routed here), and the variant was 0 (Task 7 / overlay).
        assert_ne!(sim.mapgen_rng.state(), mapgen_before, "repair drew mapgen stream");
    }
```
> Note: a zero-state `SimRng` still *advances its index fields* on a draw (the words stay 0
> but `index_a`/`index_b` increment), so `state()` changes after a draw. That is the correct
> proof the repair routed to `mapgen_rng`. The *variant* being 0 is proven by Task 7's overlay
> equality. If you instead want to prove "value 0", assert the overlay byte, not the rng state.

**Step 2: Forced non-zero mapgen varies the variant (acceptance test 3, routing direction).**
```rust
    #[test]
    fn forced_nonzero_mapgen_varies_repair_variant() {
        let mut sim = /* same destroyed-bridge fixture */;
        sim.mapgen_rng = crate::sim::rng::SimRng::new(0xDEAD_BEEF); // force non-zero
        let scenario_before = sim.scenario_rng.state();
        /* trigger the repair */;
        // The repaired overlay byte should be base + variant where variant != 0 is
        // possible; the point is ONLY mapgen was consumed:
        assert_eq!(sim.scenario_rng.state(), scenario_before, "scenario still untouched");
        // (Optional) assert the overlay is in base..=base+REPAIR_VARIANT_LIMIT_INCLUSIVE.
    }
```

**Step 3: Lockstep determinism (acceptance test 4).**
```rust
    #[test]
    fn two_fresh_sims_repair_identically() {
        let mut a = /* fixture, seed S */;
        let mut b = /* identical fixture, seed S */;
        /* trigger the same repair span on both */;
        assert_eq!(a.state_hash(), b.state_hash(), "identical sims must hash-match");
        assert_eq!(a.scenario_rng.state(), b.scenario_rng.state());
        assert_eq!(a.main_rng.state(), b.main_rng.state());
        assert_eq!(a.mapgen_rng.state(), b.mapgen_rng.state());
    }
```

**Step 4: Three-stream snapshot round-trip + version guard (acceptance test 5).** Extend the
existing two-stream round-trip:
```rust
    #[test]
    fn snapshot_round_trip_persists_all_three_streams() {
        let mut sim = Simulation::new();
        /* trigger a repair so mapgen_rng has advanced past zero-state */;
        let scenario_before = sim.scenario_rng.state();
        let main_before = sim.main_rng.state();
        let mapgen_before = sim.mapgen_rng.state();

        let bytes = GameSnapshot::save(&sim, 0, 0, "rng_test", 0);
        let restored = GameSnapshot::load(&bytes).expect("snapshot load").sim;

        assert_eq!(restored.scenario_rng.state(), scenario_before);
        assert_eq!(restored.main_rng.state(), main_before);
        assert_eq!(restored.mapgen_rng.state(), mapgen_before, "mapgen stream must round-trip");
    }
```
(The existing `version_mismatch_is_rejected` test at `snapshot.rs:204` is version-agnostic —
it corrupts byte 0 — so it continues to prove the 13-guard rejects a wrong version with no
edit needed.)

**Step 5: Verify**
Run: `cargo test --lib rng_routing -- --nocapture`
Expected: PASS for all four new tests.

**Step 6: Commit** — `sim/world: add mapgen_rng routing-isolation, lockstep, and round-trip tests`

---

### Task 9: Full sim regression sweep (catch hash-fixture breakage)

**Why:** Folding `mapgen_rng` into `state_hash()` changes the hash; any test pinning a literal
hash value (or a snapshot recorded at v12) must be regenerated. Surface them now, not in a
later session.

**Files:** none (verification only).

**Step 1: Run the sim test suite.**
Run: `cargo test -p <sim-crate>` (or `cargo test --lib`).
Expected: all pass. If a test fails because it pins a **literal** `state_hash()` value or
loads a stored v12 snapshot, that is the expected, intended break — regenerate the pinned
value / re-record the fixture under v13 (do NOT revert the fold or the version bump). If a
test fails in a file you did not touch and is unrelated to RNG/hash/snapshot, treat it as a
possible parallel session and confirm before acting.

**Step 2: Run the bridge + snapshot + rng modules explicitly to confirm green.**
Run: `cargo test bridge_repair`, `cargo test snapshot`, `cargo test rng_routing`.
Expected: PASS.

**Step 3: Commit** any regenerated fixtures — `sim: regenerate state-hash fixtures for mapgen_rng fold`
(skip if none changed).

---

### Task 10: Verification against gamemd.exe behavior

**Why:** Confirm the implemented result matches the original engine's observable behavior.

**Verify:**
- **Fixed-map variant is 0.** gamemd: `g_MapGenRng` is unseeded BSS ⇒ `FUN_00598030` → `low`
  ⇒ repaired main-deck cell = base (`0x4A`/`0x53`/`0xCD`/`0xD6`). Ours: `mapgen_rng` zero-state
  ⇒ variant 0 ⇒ same base. Confirmed by Task 6 (unit) + Task 7 (end-to-end overlay equality).
- **Repair consumes no gameplay RNG.** gamemd draws `g_MapGenRng`, not `Scen->Random`/
  `g_MainRng`. Ours: scenario/main states unchanged across a repair. Confirmed by Tasks 7 & 8.
- **Lockstep-safe.** Two fresh fixed-map clients repair identically and their gameplay
  cursors stay aligned. Confirmed by Task 8.
- **Out of scope (do not implement):** random-map exact variant parity (`FUN_00598960` seed +
  `FUN_00598030` float-scale draw). Deferred (Blocked row).

**Step:** Re-read this checklist against the green test output from Task 9; if every row is
backed by a passing test, the fixed-map fix is done. No code change in this task.

---

## Sources & References

- **Contract:** `docs/contracts/2026-05-29-bridge-walker-variant-mapgen-rng-implementation-contract.md`
- **Ghidra (verified this session):**
  - `disassemble_function 0x00598030` — `MOV ECX,0x00ABE890` @ `0x0059805E`; `CALL 0x0065C780`
    (`Random__Next`) @ `0x00598063`; float-scale (`FMUL` normalizer `0x007ED898`, `ftol`
    `0x007C5F00`, `JA` reject `> high`). NOT `RandomRanged 0x0065C7E0`.
  - `get_xrefs_to 0x00ABE890` — sole WRITE `0x0059899B` in `FUN_00598960` (random-map gen);
    all other refs DATA reads. Never seeded at game start.
  - `get_function_callers 0x00598030` — live `RepairBridgeWalker_{NS_Low 0x0057F6A0,
    EW_Low 0x0057FBC0, NS_High 0x005800D0, EW_High 0x00580600}`.
- **Research docs:** `RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §4.3#3/§9;
  `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §12.4/§12.5/§12.9 (overlay bands + bases);
  `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md` (the precedent two-stream split).
  Doc follow-up: correct `RNG_SYSTEM_GHIDRA_REPORT.md` §3.3 ("g_MapGenRng never consumed during
  a normal tick" — bridge repair consumes it) — separate doc task, not in this plan.
- **Rust touchpoints (verified this session):** `src/sim/rng.rs:14-34,73-126`;
  `src/sim/world/mod.rs:265-266,288-302,463-522,531,545-549`;
  `src/sim/world/world_hash.rs:36-46`; `src/sim/snapshot.rs:15-19,67-132,153,204`;
  `src/sim/world/world_orders.rs:376-384`; `src/sim/bridge_state/walker.rs:65-322,339-343,412-425`;
  `src/sim/bridge_state/repair_tests.rs:160,188`;
  `src/sim/world/world_orders_bridge_repair_tests.rs:503,524-536`;
  `src/sim/world/rng_routing_tests.rs:108-138,226`.
- **Prior commits:** `795fdd4` (two-stream RNG split — the pattern this extends); `c7ae5bd`
  (HEAD).
