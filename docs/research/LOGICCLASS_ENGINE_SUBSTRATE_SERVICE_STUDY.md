# LogicClass as an Engine Substrate Service — Study & Replacement-Boundary Design

**Date:** 2026-05-29
**Mode:** study/design only — no Rust written. Authority order binary → Ghidra → docs; every
load-bearing native claim re-verified live this session (citations inline).
**Bar:** active in a standard **local skirmish** (`g_GameMode == 0` campaign-local or `== 5`
skirmish/LAN). MP-only / SpecialFlags / TS-legacy behavior is flagged DORMANT.
**Builds on (does not re-decide):** `docs/plans/2026-05-28-logicclass-object-lifecycle-spine-design.md`
and `docs/plans/2026-05-28-logicclass-scheduler-live-pass-contract.md`. Where those decided
something, this study cites the decision rather than re-opening it.

---

## 0. Executive summary

"LogicClass" is **not one thing** — in gamemd it is a `DynamicVectorClass<ObjectClass*>`
**singleton** (`0x0087F778`) that plays two substrate roles, plus a per-frame **driver
function** that is named after it:

1. **Active-object registry** — the membership set of objects that receive per-tick AI, with
   tail-append register / compacting remove / per-object membership bit.
2. **Per-tick scheduler** — `LogicClass::PerTickUpdate @ 0x0055AFB0` runs a **fixed ladder** of
   global subsystems (tiberium, bombs, teams, lasers, lightning, radiation, EMP, …) and **one
   ordered live-vector object-AI pass** where every registered object's *entire* per-frame
   update happens in a single `vtable+0x5C` call, in insertion order, with the count re-read
   each iteration.

Two things commonly confused with it are explicitly **separate**: the keyboard/command
dispatcher `Process_Command @ 0x0055DEE0` (historically mislabeled `LogicClass::AI`), and the
z-sorted **draw** list `LayerClass` / `g_DisplayLayers @ 0x008A0360` (same base class, different
instance and purpose).

**State of the Rust port:** the *storage + scheduler primitive* is already built and faithful
(`LogicVector`, `register/unregister_live_object`, `for_each_live_object`, save/load, hash). The
**gap** is usage and ordering: `advance_tick` is a ~22-pass phased pipeline that iterates the
`EntityStore` in **stable-id order**, and `for_each_live_object` is called from **zero**
production phases (test-only). The native single ordered object pass and the native global-rung
macro-order are not reproduced; several global rungs are missing entirely; ore growth/spread runs
late instead of early.

**Headline verdict:** the substrate boundary is the right design and is half-built. The remaining
work is (a) a Reveal/Conceal lifecycle chokepoint, (b) incremental migration of object phases from
stable-id order to logic-vector order, (c) global-rung macro-reorder (ore-before-objects is the
biggest), (d) scaffolding the missing global rungs, and — deferred, large — (e) the true single
interleaved object walk that closes the inter-phase interleaving DRIFT.

---

## 1. Verified active-YR responsibilities

All verified via `decompile_function 0x0055AFB0`, `get_function_callers 0x0055AFB0` (sole caller
`Main_Tick @ 0x0055D360`), `decompile_function 0x0055D360`.

| # | Responsibility | Evidence | Active in skirmish |
|---|---|---|---|
| R1 | **Active-object membership registry** — own the set/order of objects that get per-tick AI | singleton vector `+0x04` items / `+0x10` count; register `0x0055BAA0`, remove `0x0055BAE0` | Yes |
| R2 | **Per-tick scheduler / driver** — run the fixed global-subsystem ladder + the object-AI pass once per frame | `PerTickUpdate @ 0x0055AFB0`, called once from `Main_Tick @ 0x0055DC9E` | Yes |
| R3 | **Single ordered object-AI fan-out** — one `vtable+0x5C` call per registered object = that object's whole-frame update, insertion order, live count reload | loop `0x0055B608..0x0055B619` | Yes |
| R4 | **Same-tick membership semantics** — objects appended mid-pass run the same pass; self-removal compacts and skips the shifted successor (no index repair) | count re-read `0x0055B613`; remover left-shift `0x0055BB11` | Yes |
| R5 | **Save/load of the order** — the vector is serialized verbatim and restored in order with pointer swizzle | Save `FUN_00551B20` (in `0x0067D300 @ 0x0067D435`, `ECX=0x0087F778`); Load `FUN_00551B90` (in `0x0067E730 @ 0x0067E8D2`) | Yes |

**Substrate framing:** R1+R5 are the *registry* service; R2+R3+R4 are the *scheduler* service.
The driver also hosts a stack of **non-object global subsystem rungs** (§2, §3) that are part of
the same per-frame function but operate on their own arrays, not the object vector.

What LogicClass is **not**:
- **Not** the input/command dispatcher. `Process_Command @ 0x0055DEE0` takes `ECX = &keycode`
  (a stack local, `LEA ECX,[ESP+0x38]` at the call site `0x0055D8B4`), dispatches hotkeys via a
  CommandClass handler table, and is unrelated to the singleton. (verified via
  `decompile_function 0x0055DEE0`, byte-confirmed call site `read_memory 0x0055D8B9`.) The old
  `LogicClass::AI` label on this function is wrong.
- **Not** the draw list. `LayerClass`/`g_DisplayLayers @ 0x008A0360` is five `DynamicVectorClass`
  instances (z-bands) walked by `Tactical_ObjectRenderingLoop @ 0x006D8DB0`; only band 2 (Ground)
  is Y-sorted. Same base class, different instance, render-only. (verified `read_memory 0x008A0360`
  distinct BSS region; `LAYER_CLASS_GHIDRA_REPORT.md`.)

---

## 2. Surface inventory

### 2.1 Singleton storage layout (`0x0087F778`, 24 bytes)

`DynamicVectorClass<ObjectClass*>` base. Offsets proven from the code that reads them
(`decompile_function` on `0x0055AFB0`, `0x005519B0`, `0x0040CE50`, `0x0040CC70`, `0x0040CDC0`;
static `read_memory 0x0087F778` is all-zero because it is runtime-constructed).

| Off | Field | Evidence |
|-----|-------|----------|
| +0x00 | vtable ptr (`= 0x007E18FC`) | Insert calls `[*this+8]` Resize slot; ctor `0x0040CBAC` installs it |
| +0x04 | `ObjectClass** Items` | PerTickUpdate `*(this+4)`; remover `[ESI+4]` |
| +0x08 | `int Capacity` | Insert grow-check `this[2]`; Resize `0x0040CE50` writes it |
| +0x0C | `IsAllocated` (owns-array; byte at +0x0D) | Insert/Clear gate `FUN_007c8b3d(array)` on byte 0x0D |
| +0x10 | `int ActiveCount` | PerTickUpdate loop bound `*(this+0x10)`; remover `[ESI+0x10]` |
| +0x14 | `int GrowthStep` | Insert `this[5]`, auto-grow increment |

### 2.2 Vtable (`0x007E18FC`) — inherited DynamicVector with 2 overrides

`read_memory 0x007E18FC len=96` (LE-DWORD). Terminates NULL at **+0x4C**. **The LogicClass vtable
has no +0x5C slot** — the `+0x5C` AI dispatch in PerTickUpdate is on the **element objects'**
vtables, never on the container.

| Slot | Addr | Function | Origin |
|------|------|----------|--------|
| +0x00 | 0x0040CC20 | deleting destructor | DynamicVector |
| +0x08 | 0x0040CE50 | **Resize/SetCapacity** | DynamicVector |
| +0x0C | 0x0040CC70 | **Clear** | DynamicVector |
| +0x10 | 0x0040CF00 | **InWhichPosition / index-of** (used by remover) | VectorClass |
| **+0x1C** | **0x0055BAA0** | **Add (override)** — register w/ `+0x98` guard | **LogicClass** |
| **+0x28** | **0x0055B880** | remove-from-sentry-vector variant (over `DAT_008b40cc/d8`) | **LogicClass** |
| +0x3C | 0x0040CDC0 | Clear variant | DynamicVector |
| +0x4C | 0x00000000 | NULL terminator | — |

(`+0x28` operates on a *different* sentry vector `DAT_008b40cc/d8`, not the element array — do not
conflate; its exact LogicClass-slot semantics are UNCHECKED.)

### 2.3 Membership / lifecycle helpers

| Symbol | Addr | Behavior | Verified |
|---|---|---|---|
| Register (Add) | 0x0055BAA0 | idempotent `+0x98` guard → `DynamicVector__Insert(tail, sorted=0)` → set `+0x98` on success | `decompile_function 0x0055BAA0` |
| Remove | 0x0055BAE0 | `__thiscall(this=vector, [esp+4]=object)`; gate `+0x98`; index-of `vtable+0x10`; order-preserving **left-shift compaction** (count `[4]`, array `[1]`); decrement count; clear `+0x98` even if absent; no tail-zero, no index repair | `disassemble_function 0x0055BAE0` |
| Insert | 0x005519B0 | tail-append at old count; auto-grow via Resize slot when `count>=capacity && GrowthStep>0`; returns low-byte bool; `sorted!=0` routes to `SortedInsert 0x00551A90` (= the **LayerClass** path, never used by LogicClass) | `decompile_function 0x005519B0` |
| Save | 0x00551B20 | write count + each element ptr in array order | `decompile_function 0x00551B20` |
| Load | 0x00551B90 | read count, tail-append in saved order, swizzle each slot `FUN_006cf240` | `decompile_function 0x00551B90` |

**Membership flag `ObjectClass+0x98`** = "currently in the Logic vector." Distinct from InLimbo
`+0x81`, IsAlive `+0x90`, IsMarked, UniqueID `+0x10`. Not serialized (`ObjectClass::Save 0x005F6250`
saves `+0x81` not `+0x98`); membership truth after load = restored vector contents.

### 2.4 Add / remove triggers (object lifecycle)

| Trigger | Addr | Effect | Notes |
|---|---|---|---|
| `ObjectClass::Reveal` | 0x005F4EC0 | → register `0x0055BAA0` | 10-gate path (coords≠sentinel, GameActive, InLimbo set, IsMarked, CanEnter unless editor, clear InLimbo + Mark(PUT) must succeed, IsAlive, **type gate `ObjectTypeClass+0x234`**, mission≠0x24, **mode gate `g_GameMode==0\|\|5\|\| UniqueID!=-2`**). Failed Mark re-sets InLimbo and returns 0 — no entry. |
| `TechnoClass::Unlimbo` | 0x006F6CA0 | → Reveal; Foot/Building Unlimbo funnel through | failed Unlimbo deletes, no entry |
| direct registrants | — | BuildingLight ctor `0x435820`, `SetInOpenTransport 0x710470`, wave/light helpers | same `+0x98` API |
| `ObjectClass::Conceal` | 0x005F4D30 | → remove `0x0055BAE0`, set InLimbo | enter-transport, garrison, deploy, death-to-limbo |
| `ObjectClass::UnInit` | 0x005F65F0 | Detach_From_All_Lists → `+0xD4` (Limbo→Conceal→unregister) → clear IsAlive → **PendingDeleteList** (deferred free) | death/destroy entry |
| `ObjectClass::Destructor` | 0x005F3B80 | guarded remove if `+0x98` set | |

### 2.5 Initial order source

No sort. Order = reveal-call chronology. Map load `ScenarioClass::Full_Init @ 0x00686B20`:
section order **Terrain → Units → Aircraft → Infantry → Structures → Smudge**, then per-section
INI key index, then per-entry construct→Unlimbo (tail-append). Runtime: chronological reveal.

### 2.6 The PerTickUpdate ladder (the driver's full body)

Full ordered ladder (verified `decompile_function 0x0055AFB0`; callee identities via
`get_function_callees` / `decompile_function` per rung). RNG draw order is load-bearing for lockstep.

| # | Rung | Callee @ addr | Active in skirmish | RNG |
|---|------|--------------|--------------------|-----|
| A | scenario cell-action + SW/IC/chrono/psychic-radar timers; `RecalcBridgeShroudFlags` every 120f | `TechnoClass__ProcessCellAction 0x6E53A0`, `FUN_004ACAC0/BC0/AE4C0`, `0x578100` | mostly conditional; bridge-shroud periodic | UNCHECKED |
| B | tiberium **growth** | `TiberiumClass__GrowthDriver_AllTypes 0x00722C40` | Yes | **Yes** `Random__Next 0x65C780` |
| C | tiberium **spread** | `TiberiumClass__SpreadDriver_AllTypes 0x007221B0` | Yes | **Yes** |
| D | bombs (placed-charge timers) | `BombClass__UpdateAll 0x00438BF0` | when bombs exist | No |
| E | 30-frame bomb/timer batch | `FUN_0054E4D0` | Yes (no-op empty) | **Yes** `RateTimer__Current 0x4C93D0` |
| F | teams (copy-to-scratch + AI) | `g_TeamClass_Array` `+0x5C`; ctor `FUN_0055BB40` | when teams exist | UNCHECKED |
| G | disk-lasers (reverse) | `g_DiskLaserClass_Array` `+0x5C` | when active | UNCHECKED |
| H | particle/spark aging (reverse) | `FUN_005FF390` (`DAT_00AC167C`) | when particles exist | UNCHECKED |
| I | laser-draw | `LaserDrawClass__UpdateAllAI 0x00550150` | when beams exist | No |
| J | lightning storm | `LightningStorm__Process 0x0053A6C0` | when storm active | **Yes** `RandomRanged 0x65C7E0` |
| K | radiation sites (reverse) | `RadSiteClass` array `DAT_00B04BD4` `+0x5C` | when rad sites exist | UNCHECKED |
| L | cell relight/terrain cache (budgeted) | `FUN_00554D50` | Yes | UNCHECKED |
| M | EMP pulses (reverse) | `EMPulseClass__UpdateAll 0x004C54A0` | when pulses exist (unconditional call) | No |
| **N** | **MAIN live-object AI vector** (`+0x04`/`+0x10`, count reload, `+0x5C`) | inline | **Yes — primary driver** | **Yes** (per-object) |
| O | AnimClass independent-AI loop | `DAT_00A83E04` `+0x5C` | **SKIPPED in modes 0/5** | UNCHECKED |
| P | wave splash forces | `FUN_0053D310 → Wave_splash_forces 0x0053CBE0` | when waves exist | **Yes** (area damage) |
| Q | alpha-shape purge | `AlphaShapeClass__PurgeDisabled 0x00420E90` | Yes | No |
| R | crate regen timers | `MapClass__UpdateCrateRegenTimers 0x0056BBE0` | when crates on | **Yes** (place at random) |
| S | tactical AI | `g_Tactical->+0x5C` | Yes (render/UI layer) | UNCHECKED |
| T | factories | `g_FactoryClass_Array` `+0x5C` (live count) | Yes | UNCHECKED |
| U | houses | `g_HouseClass_Array` `+0x5C` (null-guarded, live count) | Yes | **Yes** (AI brain) |
| V | recenter on last-ref object | `DisplayClass__GetLastRefObject → FUN_006D6070` | when flagged | No |

**Three distinct loop shapes coexist — do not generalize one to all:** (a) team = copied-count
scratch list; (b) disk-laser / radsite / FUN_005FF390 = reverse loops with count snapshot at entry;
(c) main object vector / factory / house = forward loops with **live count reload**. Factory and
house are *separate global arrays*, not the object vector.

Confirmed RNG draw order within a tick: **B → C → E → J → N(per-object) → P → R → U**.

---

## 3. Active vs inactive / legacy / dormant

**Active in standard skirmish:** R1–R5; rungs B, C, D, E, I, L, M, N, P, Q, R, S, T, U, V (D/F/G/H/
I/K/M/P run every tick but iterate zero objects when their arrays are empty — "active-but-empty,"
not skipped).

**Mode-gated SKIP in skirmish (modes 0/5):**
- **Rung O** — AnimClass independent-AI loop `DAT_00A83E04`, gated `g_GameMode != 0 && != 5`.
  In skirmish, anims tick through the **main object vector** (N) when revealed, not through O.
- `Network_Keepalive` (Main_Tick) — gated `g_GameMode==4`.
- Main_Tick `RandomRanged(0,2)` ambient spend — gated `g_GameMode==3||4` (MP stream alignment).
- MP bandwidth-throttle blocks — `g_GameMode==4` only.

**DORMANT by INI/SpecialFlags default in YR (do NOT implement as always-on):**
- **Shroud regrowth** (`FUN_004ACAC0`, rung A) — gated `ShroudGrow != 0`; **`ShroudGrow=no`** default.
- **Fog regrowth** (`FUN_004ACBC0`, rung A) — gated `ScenarioFlags & 0x1000` (FogOfWar);
  **`FogOfWar=no`** default. (Matches the project's known TS-fog ghost.)
- Meteor/ion weather timers (rung A) — need Rules flags set; off by default.
- Lightning storm (A6/J) — only when a storm is armed/active.

**TS-legacy / out of scope:** tunnel/subterranean movement (already excluded project-wide). Note
radiation sites (rung K) are **NOT** TS-legacy — Desolator / nuke fallout are live YR.

**Out-of-sim (render/UI layer, not `advance_tick`):** Tactical AI (S), last-ref recenter (V) —
their per-tick cadence must be matched in the render/app layer, never inside `sim/`.

---

## 4. Current Rust architecture comparison

### 4.1 What exists and is faithful (the primitive)

| Native contract | Rust | Verdict |
|---|---|---|
| tail-append register | `LogicVector::push` (`logic_vector.rs:24`) | MATCH |
| idempotent `+0x98` guard | `register_live_object` (`mod.rs:668`) + `in_logic_vector` (`game_entity.rs:171`, `#[serde(skip)]`) | MATCH |
| order-preserving compacting remove | `LogicVector::remove` via `retain` (`logic_vector.rs:29`) | MATCH (same surviving order) |
| live forward pass, count reload, no index repair | `for_each_live_object` (`mod.rs:711`) | MATCH (4 contract tests, `snapshot.rs:322-427`) |
| save order verbatim, swizzle | `LogicVector` serde as inner `Vec` (`logic_vector.rs:62`); restored positionally | MATCH |
| membership rebuilt on load (flag not persisted) | `rebuild_logic_membership` (`mod.rs:923`) | MATCH |
| despawn unregisters before free | `despawn_entity` (`mod.rs:765-788`) | MATCH |
| order is lockstep state | hashed in `state_hash` (`world_hash.rs:47-53`) | Correct — keep |

**The order primitive is done and faithful** (= Plan B, all 8 phases complete). Register/unregister
**coverage** of current spawn/limbo paths is also correct (world_spawn, passenger unload/board,
garrison eject, paradrop, despawn all wired; limbo spawn correctly does NOT register).

### 4.2 What drifts (usage + ordering)

1. **The object pass is not routed through the vector.** `advance_tick` (`mod.rs:1450`) is **~22
   distinct entity-iterating passes**, almost all `entities.keys_sorted()` snapshots or
   `values()/values_mut()` scans (stable-id / BTreeMap order). `for_each_live_object` is called from
   **0 production phases** (4 callers, all `#[cfg(test)]` in `snapshot.rs`). So gamemd's **one**
   ordered insertion-order fan-out → **~22** independent stable-id sweeps. **DRIFT** wherever
   processing order is observable (cell contention, fire/target tie-breaks, dock reservation races,
   same-tick spawn visibility).

2. **Global-rung macro-order is wrong; the biggest is ore.** Native runs tiberium **growth then
   spread before** the object pass (rungs B/C). Rust runs `tick_native_growth_driver` /
   `tick_native_spread_driver` (`mod.rs:1953/1966`) **late in Phase 7**, after combat and production.
   Growth-before-spread internal order is preserved; the macro-position is inverted. **DRIFT, every
   tick of every match.**

3. **Missing global rungs entirely** (grep-confirmed absent): bomb-timer driver
   (`BombClass::UpdateAll` + the RNG-consuming `FUN_0054E4D0`), disk-lasers, laser-draw fade,
   radiation sites (+ `FUN_005FF390` aging), **EMP-pulse expiry**, wave splash, alpha-shape purge,
   crate-regen timers, and the early IC/chrono/psychic visual-timer block + 120-frame bridge-shroud
   recalc. Each is a behavior gap **and** an RNG-cursor/lockstep risk when those units exist.

4. **Inter-phase interleaving DRIFT (the big, pre-existing one).** Native: object A's *entire* AI
   (move→mission→fire) commits before B's begins, so A sees this-tick mutations of earlier objects
   and last-tick state of later ones. Rust phased tick = all-move-then-all-fire, so a firing unit
   sees post-move positions of *every* other unit. Changes leads/facing/who-hits-first. Named and
   **deferred** in Plan A.

5. **Frame-counter timing — already fixed.** Native increments `g_CurrentFrameCounter` LATE
   (`Main_Tick 0x0055DE7E`, after PerTickUpdate). Rust now commits `binary_frame`/`tick` late in
   `run_late_region` (`mod.rs:1446`). **MATCH** (supersedes older "1-frame-early" DRIFT notes).

6. **Hash/desync — design caution, not a port bug.** There is **no** live per-frame state-hash
   compare in gamemd `Main_Tick` (the only compare is replay-playback selection-sum; lockstep is
   command-queue sync). Rust `state_hash` is a correct internal/replay tool; the net layer must
   **not** implement "hash-mismatch → abort" expecting native parity, and must block `advance_tick`
   (pause gate) rather than run-then-compare.

7. **RNG is single-stream in Rust; native is two streams** (`g_MainRng` combat/particles/growth vs
   `Scen->Random` scatter/sub-cell/house-roll), diverging from tick 1. Designed in the RNG handoff,
   not yet implemented. **DRIFT** for any cross-stream interaction.

---

## 5. The gamemd-native behavior contract (what the substrate must reproduce)

This is the observable contract; reproduce the *outputs*, with clean Rust internals.

**C-REGISTRY (membership):**
1. Register = **tail-append, no sort**; idempotent via per-object membership bit; flag set only on
   successful insert.
2. Unregister = **order-preserving compacting remove** (never swap); clears the bit even if absent;
   no tail-zero (count governs membership); no index repair.
3. Insertion point is **reveal/unlimbo** (gated by type flag + game-mode), not construction. Failed
   reveal → no entry. Limbo objects are absent until revealed.
4. Save serializes the order **verbatim**; load restores in saved order; the membership bit is
   derived from vector presence, not persisted; vector cleared before the load stream is applied.

**C-SCHEDULER (the object pass):**
5. **One forward pass, ascending insertion order**, each object updated by exactly one whole-frame
   call.
6. **Count re-read every iteration** → an object tail-appended before the cursor reaches it runs the
   **same** pass.
7. **No index repair on removal** → a self-/earlier-unregister compacts left and the cursor still
   advances, so the pulled-in successor is **skipped** this pass.
8. **No null guard** on the item — list integrity is a precondition (Rust must ensure the order
   never references a freed entity; `despawn` unregisters first).
9. **Same-tick read-after-write across objects** — object N's committed mutations (movement,
   occupancy, death, radio, dock link) are visible to object N+1 the same tick.

**C-LADDER (global ordering):**
10. Fixed rung order (§2.6) with the object pass (N) sandwiched: tiberium growth/spread, bombs,
    teams, lasers, lightning, radiation, EMP **before**; anims(skip), wave, alpha, crate, tactical,
    factories, houses, last-ref **after**.
11. **RNG draw order** B→C→E→J→N→P→R→U is part of the contract; reordering any rung shifts every
    later RNG result even if each system is individually correct. Use **raw `Random__Next`** (not
    `RandomRanged`) where native does (TIBTRE prob, particle lifetime/jitter).
12. Three loop shapes (copied-count / reverse-snapshot / live-reload) are per-rung; only N/T/U use
    live reload.

**C-TIMING:**
13. Whole tick reads the **pre-increment** frame counter; increment is late and pause-gated.

---

## 6. Rust-native replacement boundary

**Principle:** *Rust-native structure, gamemd-native semantics.* Do **not** port the
`DynamicVectorClass`/vtable/COM machinery. Model the **behavior contract** behind a small set of
owners. This refines (does not replace) the approved Plan A boundary.

```
                       ┌─────────────────────────────────────────────┐
   spawn / map load    │  LIFECYCLE CHOKEPOINT (Simulation methods)   │
   production / unload │   reveal(id)   → register_live_object        │
   paradrop / deploy   │   conceal(id)  → unregister_live_object      │
   death / destroy ───▶│   unlimbo(id)  → reveal                      │
                       │   uninit(id)   → conceal, then store-remove  │
                       └───────────────┬─────────────────────────────┘
                                       │ owns reveal/conceal effects
                                       ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │  REGISTRY  (LogicVector + GameEntity.in_logic_vector)              │  ← built, faithful
   │   order: Vec<u64> (insertion order) ; flag mirrors +0x98          │
   │   push/retain/snapshot/as_slice ; serde = inner Vec               │
   └───────────────┬──────────────────────────────────────────────────┘
                   │ iteration authority
                   ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │  SCHEDULER DRIVER  (advance_tick, kept phased per Plan A)          │
   │   • object-AI phases iterate logic order (snapshot or live pass)   │  ← migration target
   │     - for_each_live_object where same-pass membership matters      │  ← built, unused
   │   • global rungs run in native MACRO-ORDER around object phases    │  ← reorder + scaffold
   │   • RNG draw order honored ; pre-increment frame ; late commit     │
   └──────────────────────────────────────────────────────────────────┘
```

**Three owners, clean responsibilities:**

- **Registry** = `LogicVector` + `in_logic_vector`. *Already exists.* It is the single authority on
  object processing order. Stable-id (`BTreeMap`) order is demoted to what it is natively — a
  *storage* determinism detail, not the scheduler.
- **Lifecycle chokepoint** = `reveal/conceal/unlimbo/uninit` Simulation methods (Plan A named these;
  not yet built as a single chokepoint — coverage is hand-wired per site). Every spawn/limbo/death
  transition routes through these so future spawn paths cannot silently forget registration. Guard
  with a `debug_assert!` invariant: `order.len() == count of in-store entities with in_logic_vector`.
- **Scheduler driver** = the phased `advance_tick`, reorganized so (i) object phases iterate logic
  order, (ii) global rungs sit in native macro-order, (iii) the RNG draw order matches §5/C-LADDER.

**Decided scheduler shape (Plan A, do not re-open):** keep the phased tick as *order-authority over
phases* — phases iterate the vector instead of `keys_sorted()`. The literal **single per-object
interleaved AI walk** (which alone closes C9's cross-object interleaving) is **deferred**: it is a
multi-session rewrite with heavy borrow-checker cost (each object's AI needs `&mut self` +
read-all-others + spawn) and high regression risk. The boundary above is compatible with later
swapping the driver to the true walk without touching Registry or Lifecycle.

---

## 7. Ad hoc Rust logic to retire / demote

When the scheduler routes object work through the Registry, the following are retired, demoted, or
unified:

1. **Per-subsystem `entities.keys_sorted()` scheduling** as the de-facto object order — across
   movement (`movement_tick.rs`), combat (`combat/mod.rs:1212`, targeting `:328`), turret
   (`turret.rs:95`), deploy (`deploy.rs:80`), infantry fear (`infantry.rs:135`), retaliation
   (`combat_targeting.rs:328`), building-up/down (`mod.rs:1223/1246`), ship wakes (`mod.rs:1593`),
   and the `world_orders.rs` order-intent scans. Each currently re-derives stable-id iteration;
   they should consume the logic-order snapshot (or `for_each_live_object`). Stable-id iteration is
   **demoted** to storage determinism, not deleted.
2. **`live_object_order_snapshot()` consumers that need same-pass semantics** (`mod.rs:693`) — a
   point-in-time copy that silently loses same-pass append/skip. Migrate the ones that need it to
   `for_each_live_object`; keep snapshot only for read-only, order-only consumers
   (e.g. `state_hash`, passenger reconciliation).
3. **The bridge-repair `key_idx += 2` local skip hack** — mimics the native compacting-skip over
   *sorted stable IDs*; once real logic-vector order + the skip semantics are in place, this ad hoc
   workaround is retired.
4. **`garrison_original_owner`** (passive map-authored owner) — native resolves civilian revert via
   the live Civilian house with no per-building original-owner field; the relative vector position
   of building vs infantry decides same-frame vs next-frame transfer. Candidate for retirement once
   object order is faithful (UNCHECKED — verify the civilian-revert path first).
5. **Ore growth/spread late-Phase-7 placement** (`mod.rs:1953/1966`) — relocated to a pre-object
   rung (not deleted; moved). The growth-before-spread internal order is already correct and stays.

**Not retired (correct as-is):** `LogicVector`, register/unregister, `for_each_live_object`,
`rebuild_logic_membership`, despawn-unregister-before-free, hashing the order, limbo-spawn-no-register.

---

## 8. Migration slices + acceptance tests

Sequenced to land observable parity earliest with lowest regression risk. Each slice is gated on a
**full-skirmish replay state-hash regression** (hash unchanged, or changed only in the
expected parity-improving direction), per Plan A.

### Slice 0 — Primitive (DONE)
`LogicVector` + register/unregister + `for_each_live_object` + save/load + hash + 4 scheduler tests.
*Already complete (Plan B). Baseline.*

### Slice 1 — Lifecycle chokepoint (low risk, structural) — DONE 2026-05-29
**Status: implemented.** `reveal/conceal/unlimbo/uninit` added on `Simulation` as delegators over the
existing `register_live_object`/`unregister_live_object` primitives; `uninit` is now the canonical
despawn impl and `despawn_entity` delegates to it; production spawn/limbo/death sites migrated
(`world_spawn` reveal/uninit, `passenger` reveal/conceal, `production_sell`/`drop_payload` reveal→
unlimbo for paradrop, `slave_miner`/`world_orders`/`app_sim_tick`/`mod.rs` uninit); a debug-only
order↔membership invariant (`debug_assert_logic_membership_consistent`) runs at the end of each tick.
Coverage unchanged from the §4.1 audit (1:1 migration — limbo spawn still does not reveal). Verified
behavior/hash-neutral: the full lib suite shows the **same 11 pre-existing baseline failures** with
the change as without it (3319 vs 3314 passed = +5 new acceptance tests), and the invariant **never
fired** across 3330 tests. Plan: `docs/plans/2026-05-29-logicclass-slice1-lifecycle-chokepoint-plan.md`.

Add `reveal/conceal/unlimbo/uninit` Simulation methods; route every existing spawn/limbo/death site
through them; add the `debug_assert!` order/flag invariant.
- **Acceptance:** `reveal_then_conceal_roundtrips_membership`; `uninit_unregisters_before_store_free`;
  `every_active_spawn_path_registers` (audit test enumerating spawn entry points);
  `limbo_object_registers_only_on_reveal_tail_append`; invariant assert holds across a full-skirmish
  replay. **No hash change expected** (pure refactor).

### Slice 2 — Object phases iterate logic order (the bulk, incremental, one phase per step)
**Phase 1 DONE 2026-05-29:** combat firing/kill-credit order and retaliation now use
`live_object_order_snapshot()` threading from `Simulation::advance_tick`, with stable-id fallback
only for direct test shims. Code commit: `e94eaa3` (`sim/combat: resolve combat in live object
order`). Verified in a clean detached worktree at parent `a05886e`: baseline
`cargo test -p vera20k --lib` was `test result: FAILED. 3320 passed; 11 failed; 4 ignored;`
phase-1-only `cargo test -p vera20k` was `test result: FAILED. 3321 passed; 11 failed; 4
ignored;` with the same 11 non-combat failures and the new discriminating combat test passing.

**Phase 2 DONE 2026-05-29:** miner dock reservation / same-tick refinery pad handoff now snapshots
miners in `live_object_order_snapshot()` order, with stable-id fallback only for direct unit tests
that bypass reveal/register. Code commit: `ff9ffb9` (`sim/miner: resolve dock handoff in live
object order`). Live Ghidra spot-check confirmed the native object pass is forward live-vector order
and that a releasing miner only clears contact/reservation state; a waiting miner can claim only when
its own later mission slot runs. Verified in a clean detached worktree at Phase 1 commit `e94eaa3`:
`two_miners_refinery_takeover_uses_live_object_order_not_stable_id` passed, and full
`cargo test -p vera20k` was `test result: FAILED. 3322 passed; 11 failed; 4 ignored;` with the same
known non-dock failures.

**Phase 3 DONE 2026-05-29:** ground movement / occupancy contention now threads
`live_object_order_snapshot()` from `Simulation::advance_tick` into `tick_movement_with_grids`, with
stable-id fallback only for direct wrapper/tests. Code commit: `27c0beb` (`sim/movement: resolve
occupancy contention in live object order`). Live Ghidra re-check confirmed movement/locomotor work
runs inside each object's forward LogicClass `vtable+0x5C` slot; this phase uses a snapshot reorder
for the existing Rust movement pass and defers full live count-reload/interleaving semantics to Slice
5. `two_movers_contest_same_cell_in_live_object_order_not_stable_id` passed, hash tests were neutral
(`cargo test -p vera20k --lib world_hash`: `test result: ok. 29 passed; 0 failed; 0 ignored; 0
measured; 3316 filtered out;`), and full `cargo test -p vera20k` was `test result: FAILED. 3330
passed; 11 failed; 4 ignored; 0 measured; 0 filtered out;` with the same known non-Phase-3 failures.

Replace `keys_sorted()` with `live_object_order_snapshot()` (or `for_each_live_object` for
same-pass-sensitive phases) **one phase at a time**, each gated by the hash regression. Start with
phases where order is most observable (combat targeting/fire, movement/occupancy contention, dock
reservation).
- **Acceptance per phase:** existing per-phase tests pass; full-skirmish hash unchanged or
  parity-improving; a targeted tie-break test (e.g. `two_units_contest_same_cell_resolve_in_reveal_order`,
  `two_miners_one_refinery_pad_takeover_same_tick`). Validate tie-break direction against a gamemd
  observation before flipping each phase.

### Slice 3 — Global-rung macro-reorder (ore first)
Move tiberium growth/spread from late Phase 7 to a **pre-object** rung (matching native B/C).
- **Acceptance:** `ore_growth_runs_before_object_phases`; existing ore-growth tests still pass;
  combat-crater/Reduce_Tiberium-vs-growth same-tick ordering test; hash regression. (Expect a hash
  change in the parity-improving direction — document the before/after.)

### Slice 4 — Scaffold missing global rungs (per-weapon, each its own mini-contract)
Add the absent rungs in native order, each only active when its array is non-empty:
EMP-pulse expiry, radiation sites, disk-lasers, laser-draw fade, wave splash, bomb-timer driver
(+ the RNG-consuming `FUN_0054E4D0`), crate-regen timers. **Each must reproduce native RNG draw
order** (raw `Random__Next` vs `RandomRanged`) — these are lockstep-critical even before the effect
is visible.
- **Acceptance per rung:** rung runs in the correct ladder slot; RNG-cursor test
  (`rng_cursor_matches_native_after_<rung>`); behavior test for the weapon; hash regression.
- *Note:* two-stream RNG split (`g_MainRng` vs `Scen->Random`) is a prerequisite for exact cursor
  parity — sequence it before/with this slice.

### Slice 5 — True single interleaved object walk (DEFERRED, large)
Close the inter-phase interleaving DRIFT (C9 cross-object same-tick read-after-write) by collapsing
the object phases into one per-object AI walk over the Registry. Requires its own Phase 1–4. Decide
via a measured divergence scenario (two tanks crossing while both firing) whether the player-visible
gain justifies the rewrite.
- **Acceptance:** `object_N_movement_visible_to_object_N+1_combat_same_tick`; full regression suite;
  measured parity against gamemd on the crossing-tanks scenario.

---

## 9. Open questions / deferred DRIFTs (carried forward)

- **Inter-phase interleaving** (Slice 5) — unproven-equivalent, deferred; needs a measured scenario.
- **Two-stream RNG split** — designed, not implemented; blocks exact cursor parity.
- **×3 sub-step vs collapse** (timing) — Option A (`sim.tick == binary_frame == native frame`,
  render interpolates) vs Option B (keep ×3, raise to ~187 sim-ticks/s). Speed-byte→cap mapping
  already done; architecture choice open. Current default runs ~3× slower wall-clock at the same
  speed setting (DRIFT, flagged).
- **Within-vector order: insertion vs `BTreeMap` stable-id** — Slice 2 fixes intra-phase order, but
  the root-cause divergence for the order-N walk (insertion order ≠ id order whenever spawn order ≠
  id order) needs a discriminating test.
- **Death-to-limbo timing** (RESOLVED 2026-05-29, live Ghidra) — an object leaves the logic vector at
  **UnInit**, not when HP hits 0. `ObjectClass::UnInit @ 0x005F65F0` calls `vtable+0xD4` (Limbo) which
  reaches `ObjectClass::Conceal @ 0x005F4D30`, which calls the remover `FUN_0055BAE0` (gated on the
  logic-enabled type flag `type+0x234`; the MP-only `vtable+0x10 != -2` extra gate is skipped in
  skirmish), sets InLimbo `+0x81`, and clears IsAlive `+0x90`; the freed slot is deferred to
  PendingDeleteList. Multi-tick death sequences **linger in the vector and keep receiving `+0x5C` AI**:
  `InfantryClass::DoType_Sequencer @ 0x00520AE0` advances death DoType 0x0B–0x0F each tick and only
  calls `vtable+0xF8` (UnInit) when the death animation completes (ships sink the same way, per
  `SUBMARINE_AND_SINKING` B.8). Vehicles/buildings are UnInit'd promptly at HP≤0, with death *visuals*
  spawned as separate AnimClass objects. **Slice 2 implication:** dying objects must stay in the
  logic-order iteration and still be updated until `uninit` — which the current Rust `dying`→`uninit`
  model already does. (verified via `decompile_function` 0x005F65F0 / 0x005F4D30 / 0x00520AE0;
  corroborated by COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES, SELECTION_LIFECYCLE §3.4,
  LIMBO_AND_CELL_OCCUPATION_LIFECYCLE §3.9.)
- **Producer-class identity (YELLOW):** `FUN_0054E4D0`, `FUN_005FF390`, `FUN_00554D50` — what
  registers into their arrays — and `TeamClass +0x5C @ 0x006E9140` body (no Ghidra function defined).
- **Doc staleness fixed by this study:** save/load rebuild is **resolved** (direct vector serde,
  supersedes "UNKNOWN/deferred" in `ACTIVE_OBJECT_ORDER` / `LOGICCLASS_..._SPINE` synthesis);
  `FACTORY_CLASS_BUILD_SPEED` mislabels `0x0055AFB0` as `LogicClass::AI` and puts lightning/EMP after
  factories/houses (wrong — they run before the object pass); several docs carry pre-refactor Rust
  line numbers (trust `PERTICKUPDATE_FULL_ORDERING_LADDER` 2026-05-29 re-anchor and this study).

---

## 10. Sources

**Live Ghidra this session (gamemd.exe, read-only):**
- `decompile_function` — `0x0055AFB0` (PerTickUpdate ladder + object loop), `0x0055D360` (Main_Tick),
  `0x0055DEE0` (Process_Command), `0x0055BAA0` (register), `0x0055BAE0` (remover, via
  `disassemble_function`), `0x005519B0` (Insert), `0x00551A90` (SortedInsert/Layer path),
  `0x00551B20`/`0x00551B90` (Save/Load), `0x005F4EC0` (Reveal), `0x005F4D30` (Conceal),
  `0x005F3B80` (Destructor), `0x0067D300`/`0x0067E730` (save/load orchestrators), `0x0040CE50`/
  `0x0040CC70`/`0x0040CDC0`/`0x0040CF00` (DynVec Resize/Clear/index-of), `0x0054E4D0`, `0x005FF390`,
  `0x0053D310`, `0x00554D50`, `0x004C54A0`, `0x00420E90`, `0x00550150`, `0x006D6070`, and the
  per-class AI heads `0x007360C0`/`0x0051BAB0`/`0x0043FB20`/`0x00414BB0`/`0x00423AC0`.
- `get_function_callers 0x0055AFB0` (→ only Main_Tick); `get_function_callers 0x0055BAA0`/`0x0055BAE0`
  (Reveal/Conceal/Destructor/BuildingLight/SetInOpenTransport); `get_function_callees` per rung.
- `read_memory` — `0x0087F778` (singleton), `0x007E18FC`/`0x007E192C` (vtables), `0x008A0360`
  (g_DisplayLayers, distinct), `0x007F5C70`/`0x007E3EBC`/`0x007E3354` (Unit/Building/Anim vtable
  +0x5C), `0x0055DC99`/`0x0055D8B9`/`0x0067D42A`/`0x0067E8C7` (byte-confirmed call sites).
- `get_xrefs_to` — `0x0055AFB0`, `0x007E18FC`, `0x00B04BD4` (RadSite), `0x00A83E04` (anim loop),
  `0x0087F778`. `list_globals` — game-mode/state/running/frame/tactical.

**Research docs digested (`docs/research/`):** LOGICCLASS_PERTICKUPDATE_SCHEDULER,
LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES, PERTICKUPDATE_FULL_ORDERING_LADDER,
PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS, PERTICKUPDATE_UNNAMED_CALLEE_RESOLUTION,
LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0, LOGICCLASS_VS_MAPCLASS,
LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0, ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN,
COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES, CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER,
LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS,
OBJECT_LOGIC_LIFECYCLE_ACTIVE_MEMBERSHIP_SYSTEM_MODEL_SYNTHESIS, GLOBAL_TIMING_MODEL,
NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION, PER_FRAME_RNG_CONSUMPTION_ORDER,
DESYNC_DETECTION_MAINTICK_COMPARE, TIMING_SCHEDULER_TICK_SPINE_SYSTEM_MODEL_SYNTHESIS, LAYER_CLASS.

**Design plans (built upon):** `docs/plans/2026-05-28-logicclass-object-lifecycle-spine-design.md`,
`docs/plans/2026-05-28-logicclass-scheduler-live-pass-contract.md`.

**Rust source mapped:** `src/sim/world/mod.rs` (advance_tick 1450, lifecycle helpers 660-930,
run_late_region 1351), `src/sim/world/logic_vector.rs`, `src/sim/world/world_hash.rs`,
`src/sim/game_entity.rs`, `src/sim/snapshot.rs`, and the per-phase `tick_*` functions across
movement/combat/vision/power/production/docking/tiberium/superweapon.
