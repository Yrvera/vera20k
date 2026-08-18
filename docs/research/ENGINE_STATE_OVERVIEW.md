# Engine State Overview — where the RA2/YR Rust engine stands

**Date:** 2026-05-29
**Scope:** High-level maturity read across 14 core systems, foundational-gap ranking, and the one system to build next. Read-only audit; no code changed.
**Method:** One parallel reader per system — each read the live Rust source *and* cross-checked `docs/research/` via the research-index MCP — then a synthesis pass. Ratings judged against the project's 100%-parity bar (indistinguishable from gamemd.exe on player-observable output, default-to-DRIFT). Skipped AI and tunnels per standing project rules.

---

## TL;DR

The engine has **broad coverage and almost no depth-to-parity yet.** Every core system except **power** is **PARTIAL** — wired up, works for the common case, but with documented player-visible drift from gamemd. The tick pipeline (`World::advance_tick`) is fully assembled: commands → ground move → air/special move → vision → power → superweapons → combat → retaliation → production/docks/ore → AI/defeat/anims → state-hash. The hard part remaining is **exactness, not breadth.**

**The one thing to build next (REVISED after the substrate audit below): seal one determinism leak first, then combat.** A second-pass audit of the core engine *substrate* — the layer the gameplay systems sit on — found an active determinism hazard that preempts feature work: a **float (`f32 atan2`) feeds the state-hashed `entity.facing` field on the live, every-tick movement path** (verified: `fixed_math.rs:289` → `movement_step.rs:118` → `world_hash.rs:388`). That can desync replay/lockstep across compilers or machines, and it makes verifying combat-parity meaningless because targeting, turret aim, drive-track selection, and which-armor-face-is-hit all read `facing`. The fix already exists in-repo and is unused on this path (`int_atan2_bam` integer LUT, `homing_movement.rs:208`) — it's hours, not a project.

**Revised sequence:** (1) route facing through the integer LUT [~1 session]; (2) fix the occupancy foundation-cell rebuild-on-load and add an intern-order/content-hash guard [determinism seals for save/load + future MP]; (3) **then** complete the combat damage-resolution pipeline as originally recommended. Combat is still the right *feature* move and the gap the parity bar names most explicitly ("damage numbers and armor-vs-warhead multipliers to the last decimal") — it just shouldn't be perfected on top of an untrustworthy facing value.

**The biggest raw foundational hole remains multiplayer lockstep transport** (the `net/` layer is a 44-line dead stub) — large, partly-unscoped, no live MP to test against. The determinism *primitives* it needs are mostly verified, but the substrate audit found three latent cross-peer desync landmines it will trip the moment networked MP ships (intern-order hash coupling, command tie-break key, particle RNG draw-count). See the substrate section and "The honest tension" below.

---

## Maturity table

| # | System | Rating | One-line evidence |
|---|--------|--------|-------------------|
| 1 | map + INI load | **PARTIAL** | ~200 unit keys + 63 weapon fields parsed, and bounded existing-section map values reach the shared merged rules source; map-side type/registry allocation and native stateful multi-pass reread semantics remain absent or unverified, and `IniFile` silently drops `#`/`$Include` lines (`ini_parser.rs:197`). |
| 2 | render | **PARTIAL** | No object shadows (shadow SHP half discarded, `sprite_atlas.rs:727`), no laser/tesla/beam visuals, cloak/EMP/IC tint shaders are stubs with no producer, warp alpha hardcoded `1.0` (`units.rs:248`). |
| 3 | movement | **PARTIAL** | Hull rotation uses ms-based `rot_to_facing_delta` not binary-frame `FacingClass`; straight steps bypass DriveTrack consumption — subcell/facing not bit-identical (`movement_step.rs:234`, AMCV trace FAIL #4). |
| 4 | pathfinding | **PARTIAL** | A* core + cost multipliers verified, but corridor finder is a centroid-Manhattan Dijkstra *approximation* (own `//!` admits it) and incremental zone refresh falls back to full rebuild. |
| 5 | combat | **PARTIAL** | `raw_damage = base*verses/100` then a bare `saturating_sub` (`combat/mod.rs:1948,2172`) — omits FirePower, veterancy combat/armor mults, min-1 floor, MaxDamage cap, and AoE double-truncation. **← next target** |
| 6 | production / build | **PARTIAL** | Full cost deducted at enqueue + fully refunded on cancel (`production_queue.rs:217`); gamemd deducts/refunds per-step over 54 discrete steps. No mid-build no-funds stall. |
| 7 | power | **NEAR-PARITY** | Continuous PowerRatio build-speed formula matches gamemd exactly (`production_tech.rs:457`, verified vs report). One match-fires bug: building-up structures excluded from totals. |
| 8 | radar + shroud | **PARTIAL** | LOS/spysat/gap core is solid, but minimap pings fire for RevealOnFire weapons only (`world/mod.rs:1846`) vs ~25 native event producers; `RevealByHeight` disabled (`mod.rs:1641`). |
| 9 | economy / harvest | **PARTIAL** | Full harvest cycle works, but country/difficulty `IncomeMult` is never applied to deposits and `HarvestedCredits` score stat is untracked (no `income_mult` anywhere in `src`). |
| 10 | superweapons | **PARTIAL** | 7 SWs apply real effects (Lightning Storm full-fidelity), but **Nuke, Chronosphere, ChronoWarp, Psychic Dominator, SpyPlane are unimplemented no-ops** (`world_commands.rs:1247`) — and stay stuck "ready." |
| 11 | audio | **PARTIAL** | Parsers (.aud/.csf/.bag) + tick-synced SFX solid, but channel eviction is FIFO not priority-based (`sfx.rs:429`), no per-VocClass `Limit`, no `Control` flags, no stereo pan. |
| 12 | UI / sidebar | **PARTIAL** | No in-game tooltips at all, no new-buildable cameo pulse, power-bar slide timing is an admitted placeholder (`power_bar_anim.rs`, `SLIDE_TICKS_PER_STEP=9` "unverified"). |
| 13 | lockstep determinism | **PARTIAL** | RNG value-parity + two-stream routing + state-hash verified, but `net/` is a 44-line dead `LockstepScheduler` — no transport, no MP seed handshake, no command-gate barrier; per-tick draw-order only Medium confidence. |
| 14 | save / load | **PARTIAL** | Bincode round-trip works, but live particle systems are `#[serde(skip)]` yet hashed + deal damage (`mod.rs:448`), `map_hash` hardcoded to 0 (guard is dead code), can't load from cold start. |

Rating scale: **MISSING** (no code) · **STUB** (placeholder/returns default) · **PARTIAL** (common case works, documented gaps) · **NEAR-PARITY** (matches gamemd output, only minor/edge drift). Per project rule, ratings default to the worse tier unless equivalence is proven. All 14 confidences: **HIGH**.

---

## Foundational gaps (ranked by impact)

Systems that are both *shaky* and *load-bearing* — many other systems sit on them, so their drift compounds everywhere.

1. **Combat damage resolution** — `base*verses/100` then bare `saturating_sub` omits 5+ documented pipeline stages (FirePower, VeteranCombat ~1.1×, VeteranArmor ÷1.5, min-1 floor, MaxDamage cap, AoE `ftol(ftol(dmg*falloff)*verses)` double-truncation). Every engagement deals/takes wrong damage to the decimal; death timing drives lifecycle, retaliation, defeat detection, and the state hash. Until exact, **no combat-adjacent system can be verified against gamemd.**

2. **Lockstep determinism (transport + draw-order)** — RNG/hash primitives are verified, but `net/` is a dead 44-line stub: no sockets, no seed handshake, no command-gate barrier. **MP literally cannot run**, so the spectator/replay goal — the entire reason the sim is fixed-point — is unrealized. Separately, per-tick cross-consumer draw-order is only Medium confidence, so counts can silently desync against gamemd even with a perfect RNG algorithm.

3. **Map + INI load (native map reread completeness)** — current Rust applies bounded existing-section map values after the base/YR/mode layers and retains that same merged source for `RuleSet` and overlay consumers. It still excludes map registry-list allocation/new type creation, and native stateful multi-pass per-type `ReadINI` side effects are unverified. Maps that add types or depend on reread side effects can therefore still produce wrong constants or registries downstream.

4. **Movement (hull rotation + drive-track cadence)** — hull turns use ms-based delta not binary-frame `FacingClass`; straight steps skip DriveTrack consumption, so subcell leptons and per-frame facing aren't bit-identical to `Process_Drive_Track`. Feeds vision, turret aim, docking, occupancy, and the hash every tick → a lockstep desync source and a positioning drift for combat targeting.

5. **Pathfinding (hierarchical corridor + incremental zone refresh)** — corridor finder is a centroid-Manhattan approximation, not gamemd's 3-level hierarchical zone Dijkstra; incremental refresh falls back to full rebuild. Equal-cost route choice around obstacles/collapsed bridges can diverge — deterministic-but-non-parity, cascading into movement/docking/combat positioning.

*Honorable mention (foundational core is solid, drift is at the edges):* **radar+shroud** vision LOS feeds combat targeting and is widely depended on; the core is solid, but `RevealByHeight` is currently disabled (cliffs don't block sight) and reveal isn't Z-centered.

---

## Substrate audit (core engine services)

Second pass: the 14 systems above are gameplay-facing. They all sit on a substrate of core engine services that the first audit treated as a given. Auditing the substrate directly changed the verdict — so it's here, before the recommendation.

**Headline:** the substrate is roughly **one notch *below* the gameplay layer in true readiness, despite identical PARTIAL labels.** Single-build, same-machine determinism is genuinely solid — `BTreeMap` everywhere (no `HashMap` in any sim iteration path), fixed-point sim, RNG core pinned bit-exact to gamemd seed=1, render strictly read-only on sim. But the failures cluster in two dangerous spots the gameplay audit couldn't see: **a float leaking into a state-hashed field**, and several **"live path vs rebuild path" mismatches** (occupancy foundations, intern-order-coupled hash) that diverge only after save/load or once real MP exists. The foundation is well-built but has unsealed cracks that make the gameplay-parity layer above it *unverifiable for lockstep* until closed.

| Substrate service | Rating | Determinism verdict | One-line evidence |
|---|---|---|---|
| entity store + GameEntity | **PARTIAL** | clean | All-`BTreeMap`, sorted-id iteration, serialized monotonic id allocator — but `GameEntity` is a ~70-field god-struct and the per-tick owner-index rebuild has zero callers (scale, not correctness). |
| string interning | **NEAR-PARITY** | latent hazard | Internally clean, but `world_hash` hashes the raw `InternedId` integer (`world_hash.rs:390`) → cross-peer hash equality depends on identical intern *order*, enforced only by load-pipeline convention, tested by nothing. |
| **fixed-point + lepton math** | **PARTIAL** | **ACTIVE hazard** | `facing_from_delta_int` uses `f32 atan2` (`fixed_math.rs:289`), output written to state-hashed `entity.facing` — `as i32` truncation at a bucket boundary can flip one facing unit across platforms. **← fix first** |
| RNG substrate | **PARTIAL** | parity hazard | Core LFG pinned bit-exact to gamemd, but particle consumers use rejection-sampling `next_range_u32` where gamemd does raw `Random__Next()%N` (RED in `PARTICLE_RNG_CLASSIFICATION`) — extra draws shift the shared stream vs a real gamemd client. |
| command & input transport | **PARTIAL** | cross-peer hazard | Same-tick commands tie-broken by `owner:InternedId` value (`mod.rs:1317`), but the verified contract wants HouseClass registration-index order; `net/lockstep.rs` is a dead stub. Single-machine safe, MP-latent. |
| asset/VFS (.mix + merge) | **PARTIAL** | clean (load-time only) | Archive precedence is a hardcoded fixed order approximating gamemd's last-opened-wins head insertion; **loose-file shadowing is missing entirely** (gamemd probes disk before MIX). All peers agree but can drift from retail. |
| format parsers (.shp/.vxl/.pal/.tmp/.hva) | **NEAR-PARITY** | clean (render-only) | VXL/HVA verified bit-correct vs binary; SHP format byte treated as enum not bitfield and frame rect read unsigned not signed (stock content decodes fine; malformed/negative-offset SHPs mis-route). |
| spatial substrate (occupancy + grids) | **PARTIAL** | **save/load hazard** | `OccupancyGrid::rebuild` registers only the origin cell per entity (`occupancy.rs`), but live spawn expands full building footprints — after any load, multi-cell buildings become walkable ghost cells → divergent crush/scatter RNG → desync. Grid is excluded from the hash, so the detector stays blind. |
| tick/frame loop & timing | **PARTIAL** | replay-clean, pace-drift | Determinism solid for same-build replay, but a **dual-clock model** (45 Hz sim ticks vs 15 Hz `binary_frame`) means turret lag / ROF / ore spread / gate timing run at a cadence off gamemd's single frame clock — pervasive *observable* pace drift, not a desync. |
| GPU/render plumbing | **PARTIAL** | clean (read-only on sim) | Surface-loss and device-loss are logged but never recovered (`app.rs:1900`) → wedged/black render survivable-but-stuck; `Limits::default()` self-caps below the target AMD GPU for the 20k-unit budget. Visual/availability only — never corrupts sim. |

### Critical substrate gaps (ranked)

1. **Float in the facing path (ACTIVE, single-build).** `facing_from_delta → facing_from_delta_int → f32 atan2 → as-i32 truncation`, written to hashed `entity.facing`. The "1 ULP → same bucket" comment is asserted, never proven; per the project's default-to-DRIFT rule an unproven float-in-hash is a hazard. The *only* finding that can desync even single-player replay across compilers/arch. **Verified this session** (4 reads: `fixed_math.rs:289`, `movement_step.rs:118`, `world_hash.rs:388`, `homing_movement.rs:208`). Fix primitive already in-repo, unused.
2. **Occupancy foundation-cell rebuild on load (save/load → MP).** Reloaded clients treat building footprint cells as passable; next cell-entry over a footprint cell resolves differently and consumes different RNG → desync. The debug-assert net is explicitly calibrated to *tolerate* this. Same ghost-cell bug class already proven for the live MCV-undeploy path.
3. **Intern-order ↔ state-hash coupling (latent total-desync for MP).** Hash trusts the integer ID, not the string; any future runtime-interned string or divergent house-roster order silently diverges every peer with no string-level diagnostic. A content-derived hash (or debug cross-check) makes it self-diagnosing.
4. **Command cross-peer tie-break key (MP-latent).** Wrong sort field for same-tick multi-owner commands; plus no transport, no execute-frame restamping, unsorted AI dispatch.
5. **Particle RNG draw-count + stream routing (parity vs gamemd).** Two Rust instances stay in sync with each other, but draw-count divergence accumulates into a hard desync against a real gamemd client/replay over a match.
6. **Scale blockers (20k-unit ceiling, not correctness).** God-struct entity, per-tick full-store scans, O(N) state-hash + path-grid rebuilds every tick, `Limits::default()` GPU cap. Defer until determinism is sealed.

---

## The ONE system to build next

### → First: eliminate the f32 facing leak (then occupancy + intern-hash, then combat)

The substrate audit preempts the combat recommendation with **one small, surgical determinism fix**: route `facing_from_delta_int` / `_u16` through the existing integer `int_atan2_bam` LUT (`homing_movement.rs:208`) and assert bit-stable u8/u16 bucketing across the delta space. This is the single place a non-determinism source (float) is wired directly into a state-hashed field on the live every-tick path. It's ~one session, the correct primitive already exists unused in the repo, and it makes every facing/turret/drive-track value trustworthy — which combat targeting depends on.

Then, as the immediate follow-on (both are determinism seals, not features): fix the **occupancy foundation-cell rebuild** (#2) and add the **intern-order/content-hash guard** (#3). Both silently make save/load — and any future MP combat verification — unsafe.

**Only then resume combat** — the remaining substrate gaps (command tie-break, particle RNG, scale) are either MP-only-latent or scale-only and don't block verifying single-engine combat parity.

### → Then: Complete the combat damage-resolution pipeline

Port the full gamemd damage math as ordered fixed-point stages, replacing today's `base*verses/100` + bare `saturating_sub`:

```
base
  → × FirePower (country multiplier)
  → × VeteranCombat (attacker veterancy, ~1.1× elite)
  → × verses/armor multiplier
  → ÷ VeteranArmor (defender veterancy, /1.5)
  → clamp ≥ 1        (min-1-damage floor)
  → clamp ≤ Rules.MaxDamage (default 10000)
```
…all in `I32F16` with gamemd's exact truncation order, plus the AoE two-truncation form and the `ReceiveDamage` immunity / `AffectsAllies` gates.

**Why this one wins:**
- **Foundational + shaky + parity-named.** Combat is genuinely load-bearing (death/damage drives lifecycle, retaliation, AI defeat, and the determinism hash) and it is exactly what the parity bar calls out: "damage numbers and armor-vs-warhead multipliers to the last decimal." It is verifiably wrong in code I read, not a hypothetical.
- **Highest leverage-per-effort.** It's fully documented (`DAMAGE_MATH_GHIDRA_REPORT.md` §11, `WARHEADTYPECLASS_REINVESTIGATION` §2.4, §3) — buildable now with **no new reverse-engineering** and a bounded, well-scoped surface.
- **Unblocks verification everywhere else.** Until damage is exact, every combat-touching system (retaliation, defeat, veterancy, AI, balance) stays unverifiable against gamemd, and any fix elsewhere risks masking or compounding this drift.

**First steps:**
1. Encode the ordered pipeline above in `src/sim/combat/mod.rs`, replacing the `raw_damage` at line 1948 and the bare `saturating_sub` at line 2172.
2. Fix AoE rounding in `combat_aoe.rs` to gamemd's `ftol(ftol(damage*falloff)*verses)` two-truncation form (not the single `base*verses*falloff/10000` expression); verify off-by-1 splash on boundary inputs.
3. Add `ReceiveDamage` immunity gates before health subtraction: Radiation/PsychicDamage/Poison armor immunities and `AffectsAllies=no` allied-skip, wired into the damage-event application loop (~`mod.rs:2161`).
4. Add fixed-point unit tests pinning exact outputs against documented gamemd values across **boundary cases** (high-armor vs weak weapon → 1 not 0; veteran/elite attacker *and* defender; MaxDamage cap; AoE falloff truncation) — bit-verified, not happy-path.

### The honest tension

If you weigh "the engine can't do its headline feature at all," **lockstep transport (#2) is the bigger hole** — multiplayer is a hard requirement and currently impossible. The reason it isn't the pick: it's a large, partly-unscoped build (sockets, peer protocol, seed handshake, MaxAhead command barrier) with no live MP to test against yet, and it depends on per-tick draw-order being proven exact first — which in turn depends on combat (a per-tick RNG consumer) being correct. Combat is the smaller, fully-scoped, higher-certainty move that also de-risks lockstep. **Recommend combat now; lockstep transport as the next major milestone after the per-tick consumer order is locked down.** Your call.

### Runner-ups considered

- **Lockstep transport** — most foundational overall; large/unscoped, no live MP to test, lower leverage-per-effort right now.
- **Map+INI native reread completeness** — bounded existing-section values now merge, but map-side type allocation and native stateful reread side effects remain unresolved → narrower trigger frequency than unconditional combat drift.
- **Movement hull/drive-track cadence** — real lockstep + positioning drift, but improving it is partly wasted until combat (which consumes positions for targeting) is itself exact.
- **Pathfinding hierarchical corridor** — route divergence is real but lower-frequency than per-shot damage and depends on movement consuming its output first.
- **Superweapons (Nuke/Chrono/Dominator unimplemented)** — high player-visibility (three iconic SWs are no-ops that stick "ready"), but a leaf system: nothing depends on it, so it's a feature build, not a foundation fix.
- **Power (building-up exclusion)** — NEAR-PARITY with one match-fires bug; a single localized fix, not a foundation-wide blocker.
- **Production credit timing** — wrong upfront full-cost deduction skews economy pacing, but self-contained to the queue.

---

## Reading the table: what this tells us

- **The engine is past scaffolding and into the long parity tail.** 13 of 14 systems run; the remaining work is closing documented output drift, not building from scratch. That's the expensive phase the project's rigor rules exist for.
- **Research is ahead of implementation in most systems** — combat, movement, pathfinding, power, production, superweapons, audio, radar all have verified Ghidra docs the code hasn't fully caught up to. The bottleneck is implementation fidelity, not knowledge. (Exceptions: no consolidated rules-parser key-coverage doc, and no doc of what the Rust snapshot must serialize field-by-field.)
- **The two highest-leverage moves both live in the per-tick deterministic core** (combat damage, then lockstep draw-order/transport) — fix those and a large fraction of downstream parity becomes verifiable rather than assumed.
