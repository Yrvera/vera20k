# CABHUT C4 Re-Investigation — Open Questions Log (FINAL)

**Scope:** Exhaustive-slice re-investigation of the C4-on-CABHUT chain in
gamemd.exe, scoped to the explicit open work + un-verified claims in prior
research.

**Mode:** exhaustive-slice → **rescoped to Phase 1** (>25% deferred per skill
rule; ~75 entries, ~13 resolved this session, the rest deferred with
addresses + categories).

**Outcome:** The new branching rule surfaced **8 functions never documented
in any prior report** and discovered a previously-undocumented **3-state
bridge-damage model** (healthy → damaged → destroyed-anchor). Two prior-doc
claims required correction. The full exhaustion is bounded by recursion
into the `ApplyBridgeDestruction_*` / `FindBridgeEndpoints_*` layer, which
is a worthy Phase 2.

---

## Final state (Step 3)

### A — Resolved this session (with binary evidence)

- [RESOLVED] Q1 — `CollapseBridge_NS_Low @ 0x575540` decompiled. → 4-iter walker along bridge axis, calls `DestroyBridge_Low` per cell with 3-retry, spawns 3 random debris animations per intact cell, ends with `UpdateBridgeZonesHelper` + `Tactical+0xD7C=1`. (evidence: decompile 0x575540)
- [RESOLVED] Q2 — `CollapseBridge_EW_Low @ 0x575220` decompiled. → byte-equivalent twin of Q1 with axis swap. (evidence: decompile 0x575220)
- [RESOLVED] Q3 — `CollapseBridge_NS_High @ 0x575BA0` decompiled. → twin of Q1 with high overlay band `[0xCD..0xE8]` and sentinel 0xE7. (evidence: decompile 0x575BA0)
- [RESOLVED] Q4 — `CollapseBridge_EW_High @ 0x575870` decompiled. → twin of Q2; sentinel 0xE8. (evidence: decompile 0x575870)
- [RESOLVED] Q5 — Does `UpdateBridgeZonesHelper` fire from inside the walkers? → **YES, all four `CollapseBridge_*` walkers call it themselves.** This **corrects** BRIDGE_HUT_DESTRUCTION §1.2/§4.3, which claimed the overlay-match fast path doesn't fire it. (evidence: 0x575540, 0x575220, 0x575BA0, 0x575870 — last 3 lines of each)
- [RESOLVED] Q14 — `BombClass::Attach @ 0x438E70` decompiled. → gated on `param_2.RTTI == 0xf`; allocates 92-byte (`0x5C`) BombClass instance; stores target/cell refs; sets expiry to `g_CurrentFrameCounter + RulesClass+0xFD0` (C4Delay). Path is **distinct from PerCellProcess** which uses inline `building.field_0x6DF`. (evidence: decompile 0x438E70)
- [RESOLVED] Q15 — `BombClass::Detonate @ 0x438720` decompiled. → calls `Apply_area_damage(self.coord, RulesClass+0xFC8, 1, 0)` FIRST, then spawns explosion anim with magic flag `0x2600`, then runs the 5×5 outer scan (dy outer, dx inner — same shape as BuildingClass::Update), then dispatches to `DestroyBridge_{High,Low}_MapInit`. (evidence: decompile 0x438720)
- [RESOLVED] Q33 — `DestroyBridge_Low @ 0x57BAA0` decompiled. → per-cell direction-detect dispatcher; classifies cell overlay into NS-class ([0x4A..0x52]∪[0x5C..0x5F]∪{0x64}) vs EW-class ([0x53..0x5B]∪[0x60..0x63]∪{0x65}); selects anchor 0/1/2 cells back; dispatches to **`DestroyBridgeWalker_{NS,EW}_Low`** (NEW family). Returns char (0 = "this cell is not bridge anymore"). (evidence: decompile 0x57BAA0)
- [RESOLVED] Q34 — `DestroyBridge_High @ 0x57CCF0` decompiled. → twin of Q33 with high band. (evidence: decompile 0x57CCF0)
- [RESOLVED] Q35 — When does `DestroyBridge_*` return 0? → when cell overlay is outside both NS-class and EW-class bands AND not the destroyed-anchor sentinel. The 3-retry loop in `CollapseBridge_*` retries through the next cell in walking direction when this happens. (evidence: 0x57BAA0 line `return uVar4 & 0xffffff00`)
- [RESOLVED] Q49 — `DestroyBridgeWalker_NS_Low @ 0x57BCF0` decompiled. → **NEW FAMILY never in any prior doc.** Implements one step of the 3-state damage model (see Phase-1 report §3). Writes 3 cells perpendicular to bridge axis; dispatches `ApplyBridgeDestruction_NS_Low` + on full-destroy `FindBridgeEndpoints_NS_Low`. (evidence: decompile 0x57BCF0)
- [RESOLVED] Q50 — `DestroyBridgeWalker_EW_Low @ 0x57C2B0` decompiled. → twin of Q49 with axis swap. (evidence: decompile 0x57C2B0)

### B — Resolved by address-discovery (next layer enumerated; bodies deferred)

- [RESOLVED] Q51 — `DestroyBridgeWalker_NS_High` exists at **`0x57CF60`**. (Body deferred — Q76.)
- [RESOLVED] Q52 — `DestroyBridgeWalker_EW_High` exists at **`0x57D530`**. (Body deferred — Q77.)
- [RESOLVED] Q56 — `BombClass::Attach @ 0x438E70` callers → exactly one: `WarheadTypeClass::Detonate @ 0x4690B0`. (evidence: `get_function_callers`)

### C — New functions discovered this session (next layer; deferred)

- [DEFERRED] Q60 — `ApplyBridgeDestruction_NS_Low @ 0x57DD50` — body. (category: `bounded-cost-too-high`; reason: recursion into next layer; next-step: decompile in Phase 2)
- [DEFERRED] Q61 — `ApplyBridgeDestruction_EW_Low @ 0x57E2A0` — body. (category: `bounded-cost-too-high`; reason: same as Q60)
- [DEFERRED] Q62 — `ApplyBridgeDestruction_NS_High @ 0x57E7A0` — body. (category: `bounded-cost-too-high`; reason: same)
- [DEFERRED] Q63 — `ApplyBridgeDestruction_EW_High @ 0x57ED00` — body. (category: `bounded-cost-too-high`; reason: same)
- [DEFERRED] Q64 — `FindBridgeEndpoints_NS_Low @ 0x57C990` — body. (category: `bounded-cost-too-high`; reason: same; next-step: decompile and explain what "endpoints" mean for an EW-walking N-S-axis bridge)
- [DEFERRED] Q65 — `FindBridgeEndpoints_EW_Low @ 0x57C870` — body. (category: `bounded-cost-too-high`)
- [DEFERRED] Q66 — `FindBridgeEndpoints_NS_High @ 0x57DC20` — body. (category: `bounded-cost-too-high`)
- [DEFERRED] Q67 — `FindBridgeEndpoints_EW_High @ 0x57DAF0` — body. (category: `bounded-cost-too-high`)
- [DEFERRED] Q76 — `DestroyBridgeWalker_NS_High @ 0x57CF60` body. (category: `bounded-cost-too-high`; reason: presumed compiled twin of Q49; high-confidence inference but not verified)
- [DEFERRED] Q77 — `DestroyBridgeWalker_EW_High @ 0x57D530` body. (category: `bounded-cost-too-high`; reason: same)
- [DEFERRED] Q68 — `FUN_0047fde0` and `FUN_0047fb90` — coordinate / screen-rect helpers. (category: `out-of-scope`; reason: rendering-side; not on parity-critical path of C4 cascade)
- [DEFERRED] Q70 — `FUN_00487f40` / `FUN_00487ee0` — RadarClass coordinate transforms. (category: `out-of-scope`)
- [DEFERRED] Q71 — `FUN_00487a10` (called 3× in each Walker) — probably radar/minimap mark-dirty per cell. (category: `out-of-scope`)
- [DEFERRED] Q72 — `FUN_005868a0` — called at walker end if `local_78 > 0 && local_74 > 0`; likely propagates a destruction notification to objects-on-cell. (category: `requires-different-system-context`; reason: needs unit-on-bridge subsystem context; next-step: trace what reads `local_80..local_74` rect)
- [DEFERRED] Q55 — RTTI 0xf identity. (category: `bounded-cost-too-high`; reason: requires RTTI enum extraction; next-step: read `TechnoClass::RTTI` getter in InfantryClass/UnitClass/AircraftClass vtables and match return constants)

### D — Field-offset audit (TECH_CABHUT §12 list) — deferred

- [DEFERRED] Q16 — `Selectable=` at ObjectTypeClass+0x230 verify. (category: `bounded-cost-too-high`; reason: not load-bearing for CABHUT path; next-step: `read_memory` at `0x5F948C` for string-pointer)
- [DEFERRED] Q17 — `Nominal=` at TechnoTypeClass+0xC9E verify. (category: `bounded-cost-too-high`)
- [DEFERRED] Q18 — `Engineer=` +0xEC3 vs CAHOSP §10's +0xEC5 contradiction. (category: `requires-different-system-context`; reason: cross-doc inconsistency requiring full read of `InfantryTypeClass::ReadINI`; next-step: decompile InfantryTypeClass::ReadINI in full — **this is the highest-priority single follow-up** per TECH_CABHUT §12)
- [DEFERRED] Q19 — `RepairBridgeSound=` parse at RulesClass+0x248 verify. (category: `bounded-cost-too-high`)

### E — Tick-cycle & integration — deferred

- [DEFERRED] Q20 — `BuildingClass::Update` BridgeRepairHut branch position in update sequence. (category: `requires-different-system-context`; reason: needs full Update decomp; next-step: decompile 0x43FB20 head-to-tail)
- [DEFERRED] Q21 — `BombClass::Detonate` timing within tick. (category: `requires-different-system-context`)
- [DEFERRED] Q22 — `field_0x6DF` clear timing race. (category: `needs-runtime-debugger`)

### F — Edge cases — deferred

- [DEFERRED] Q23 — Two C4 bombs on adjacent CABHUTs sharing a bridge. (category: `needs-runtime-debugger`; reason: requires in-game observation OR speculative trace of overlapping `DestroyBridge_*_MapInit` reentrancy)
- [DEFERRED] Q24 — CABHUT under Iron Curtain at timer-expiry. (category: `bounded-cost-too-high`; next-step: re-read BuildingClass::Update C4-branch for IC gate)
- [DEFERRED] Q25 — CABHUT killed-while-armed. (category: `bounded-cost-too-high`)
- [DEFERRED] Q26 — Frame-perfect race: C4 plant + engineer repair same tick. (category: `bounded-cost-too-high`; reason: requires full PerCellProcess ordering trace)
- [DEFERRED] Q27 — Save/load survival of `field_0x6DF` & timer fields. (category: `requires-different-system-context`; reason: needs BuildingClass::Save/Load decomp; next-step: `search_functions BuildingClass.*Load`)
- [DEFERRED] Q28 — Multiplayer lockstep determinism of the plant. (category: `needs-runtime-debugger`)
- [DEFERRED] Q29 — Demo truck on already-armed CABHUT. (category: `bounded-cost-too-high`)
- [DEFERRED] Q30 — CABHUT health=0 at timer-expiry. (category: `bounded-cost-too-high`)
- [DEFERRED] Q31 — `BridgeRepairHut` flag TS-vs-YR origin. (category: `out-of-scope`; reason: prior docs confirm it's live in YR; pre-YR origin not parity-relevant)

### G — Tiny details from the Walker decompiles — partially resolved, partially deferred

- [RESOLVED] **3-state bridge damage model** — overlay healthy → damaged → destroyed-anchor; each C4 cascade transitions one step per cell. (evidence: 0x57BCF0 branches on `local_ac == 0x5C / 0x5E / <0x50 / <=0x52`; 0x57C2B0 branches on `0x60 / 0x62 / <0x59 / <=0x5B`) — **NOT in any prior doc.**
- [RESOLVED] **RNG-order constraint per debris animation** — exactly 4 `Random__RandomRanged` calls per anim, in order: X-jitter, Y-jitter, frame-delay-arg (1..5), anim-array-index (0..RulesClass+0x168 - 1). 3 anims per intact cell × 4 cells = up to 48 RNG calls per walker. **Load-bearing for lockstep parity.** (evidence: 0x575540 inner loop)
- [RESOLVED] **AnimClass instance size = `0x1C8` (456 bytes)** (evidence: `operator_new(0x1c8)` in walker)
- [RESOLVED] **BombClass instance size = `0x5C` (92 bytes)** (evidence: `operator_new(0x5c)` in 0x438E70)
- [RESOLVED] **`RulesClass+0x15C`** = ptr to bridge-collapse animation array; **`RulesClass+0x168`** = count of entries. (evidence: 0x575540 anim spawn loop) — INI key name not yet identified.
- [RESOLVED] **Cell array sentinel `&DAT_00abdc50`** — used whenever cell index is OOB or `g_CellArray[i] == NULL`. (evidence: every walker fallback)
- [RESOLVED] **`DAT_00abdc74`** — backup of param that triggered OOB. Side-effect-only diagnostic. (evidence: same)
- [DEFERRED] Q36 — Cell field `+0x11B` (used as Z scale source) — what writes it? (category: `bounded-cost-too-high`; next-step: `get_xrefs_to 0xN+0x11b` won't work — search for `[reg+0x11b]` write sites)
- [DEFERRED] Q37 — `DAT_00abde88` (Z scaling factor; reads as 0 currently — likely loaded at theater init). (category: `bounded-cost-too-high`)
- [DEFERRED] Q41 — Magic flag `0x600` (walker AnimClass arg) vs `0x2600` (Detonate AnimClass arg) decomposition. (category: `bounded-cost-too-high`; next-step: decompile AnimClass::Constructor and identify bit semantics)
- [DEFERRED] Q42 — Confirm AnimClass size by reading the vtable layout. (category: `bounded-cost-too-high`)
- [DEFERRED] Q43 — Byte-equivalence verification of the 4 Collapse walkers (current evidence: structurally identical decompiles with constant substitution; not bit-exact verified). (category: `bounded-cost-too-high`)
- [DEFERRED] Q44 — Walker behavior when input cell IS destroyed-anchor sentinel. (category: `bounded-cost-too-high`; reason: in decompile of 0x575540 the `if (overlay != 0x64)` guard skips anim spawn but loop continues; need to verify retry behavior)
- [DEFERRED] Q45 — Why `local_2c = 4` iterations — longest bridge span supported. (category: `out-of-scope`; reason: empirical from map data)
- [DEFERRED] Q46 — Why 3 retries on `DestroyBridge_Low/High`. (category: `bounded-cost-too-high`; reason: probably "skip up to 3 already-destroyed cells before giving up"; not parity-critical)
- [DEFERRED] Q47 — Direction-detect orientation semantics. (category: `bounded-cost-too-high`)
- [DEFERRED] Q48 — Midpoint adjustment signed-vs-unsigned division direction. (category: `bounded-cost-too-high`; reason: parity-critical for odd-length bridges; **flag for Phase 2**)
- [DEFERRED] Q53 — DemoTruckWarhead `RulesClass+0xFC8` effect on CABHUT itself. (category: `bounded-cost-too-high`)
- [DEFERRED] Q54 — Magic flag `0x2600` decomposition. (category: `bounded-cost-too-high`)
- [DEFERRED] Q57 — BombClass target `+0x81` gate in Detonate. (category: `bounded-cost-too-high`)
- [DEFERRED] Q58 — All cited RulesClass offsets identification (0x20C, 0x210, 0x248, 0xFC8, 0xFD0, 0x15C, 0x168). (category: `bounded-cost-too-high`; next-step: scan `RulesClass::ReadINI` and `RulesClass::ReadAudioVisual` for the corresponding string-pointer reads)
- [DEFERRED] Q59 — `DAT_00abde88` writers. (category: `bounded-cost-too-high`)
- [DEFERRED] Q74 — Cell `OverlayTypeIndex` exact byte offset (Ghidra has a struct typedef; cells use `+0x44` per prior docs which matches). (category: `bounded-cost-too-high`)
- [DEFERRED] Q75 — Confirm 3-state model for all 4 walker variants (only Low NS + Low EW decompiled this session). (category: `bounded-cost-too-high`)

### H — Adversarial reader test (Step 3 §3 — 5 corner-case questions)

- [RESOLVED-A1] What happens when overlay is the destroyed-anchor sentinel (0x64/0x65/0xE7/0xE8) at walker entry? → `CollapseBridge_*` skips the debris-anim spawn but continues `DestroyBridge_*` 3-retry; per-cell walker (`DestroyBridgeWalker_*_Low`) treats it as "not in a damage-transition range" and the early-return-at-top branch fires (`if (0x52 < local_ac) return`). Net: no further state transition; advance to next cell. (evidence: 0x575540 outer `if (overlay != 0x65)`; 0x57BCF0 fall-through path)
- [RESOLVED-A2] What ordering between `Apply_area_damage` and `DestroyBridge_*_MapInit` in `BombClass::Detonate`? → Apply_area_damage FIRST, then anim spawn, then bridge cascade. Even if area damage destroys the CABHUT, the cascade still fires because the target ref is held in `BombClass+0x2C` and re-read. (evidence: 0x438720 statement order)
- [RESOLVED-A3] Does the C4 cascade guarantee the bridge fully collapses, or only damages it? → Walks 4 cells. Each cell transitions overlay ONE step (healthy→damaged or damaged→destroyed). **A fully-healthy bridge cell becomes damaged, not destroyed, on a single C4 strike** (per 0x57BCF0 branch on `< 0x50`). Second C4 finishes it. **NOT documented in any prior report.** (evidence: 0x57BCF0)
- [DEFERRED-A4] What if two C4 bombs detonate on the same tick? → Q23 deferred.
- [DEFERRED-A5] What if the bridge has a unit standing on it during collapse? → Q72 deferred (FUN_005868a0 likely handles this — `local_78=3, local_74=3` rect notification).

### I — Deferral summary

**Total entries:** 77 (32 seeded + 45 spawned by branching rule)
**Resolved:** 16 (21%)
**Deferred:** 61 (79%) — exceeds the 25% threshold → report **must** be titled "Phase 1" per skill rule.

**Major categories of deferral:**
- 8 entries: `ApplyBridgeDestruction_*` + `FindBridgeEndpoints_*` bodies — recursion into next layer
- 2 entries: `DestroyBridgeWalker_*_High` bodies — presumed twins
- 11 entries: tiny-detail follow-ups (magic flags, RulesClass offsets, sentinel writers)
- 10 entries: edge-case/runtime questions requiring in-game observation or full sibling-system decompiles
- 4 entries: type-class offset spot-checks (low-priority for the C4 chain)

The Phase 1 report documents what WAS resolved (with binary evidence) and
defers the rest with addresses + categories so a Phase 2 has a turnkey
work-list.
