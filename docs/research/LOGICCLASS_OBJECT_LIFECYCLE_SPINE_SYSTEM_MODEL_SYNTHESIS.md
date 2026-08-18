# LogicClass Scheduler + ObjectClass Lifecycle Spine — System Model Synthesis

**Output type:** model-synthesis (sources agree on the normal YR path; no normal-path conflict)
**Date:** 2026-05-28
**Scope:** the engine spine every object flows through — active-object registration (`ObjectClass+0x98`)
via reveal/conceal, unlimbo/uninit/delete lifecycle, the per-tick live forward object loop, and the
`Main_Tick` → `PerTickUpdate` global subsystem ordering + late frame-counter contract.
**Non-scope:** class-specific `vtable+0x5C` AI bodies, full object destructor call graph, save/load
stream reconstruction, replay/network resync, MapClass/CellClass internals.
**Spot-checked live this session (Ghidra MCP):** `decompile_function` on `0x005F4EC0` (Reveal),
`0x005F4D30` (Conceal), `0x0055BAA0` (adder), `0x0055BAE0` (remover), `0x005F65F0` (UnInit).

## Claim Table

| # | Claim | Best evidence | Status | Conf | YR | Safe? |
|---|-------|---------------|--------|------|----|----|
| 1 | Main object loop is a live forward walk of `LogicClass+0x04` (items) / `+0x10` (count); count is reloaded after each `vtable+0x5C` (no pass-entry snapshot). | `BINARY_HIGH` — scheduler `0x0055B5FB..0x0055B619` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 2 | Registration: `ObjectClass::Reveal` calls the adder `FUN_0055BAA0(obj,0)` with `ECX=0x87F778`; gated by type byte `ObjectTypeClass+0x234` (`piVar5[0x8d]`) and game-mode checks. | `BINARY_HIGH` — Reveal decompile (this session) `0x005F4EC0`; call `0x005F5038..0x005F5040` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 3 | Membership byte is `ObjectClass+0x98`, **distinct** from InLimbo `+0x81`. Adder sets `+0x98`; remover clears it; Reveal/Conceal own `+0x81`. | `BINARY_HIGH` — adder/remover (this session) `0x0055BAA0`/`0x0055BAE0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 4 | Adder is duplicate-guarded (early-return if `+0x98` set) and tail-appends via `DynamicVector__Insert`; insert failure leaves `+0x98` clear (no phantom entry). | `BINARY_HIGH` — adder decompile (this session); `DynamicVector__Insert 0x005519B0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 5 | Unregistration: `ObjectClass::Conceal` calls remover `FUN_0055BAE0` when `type+0x234` set + game-mode gate, then sets InLimbo `+0x81=1`. Remover compacting-shifts left, decrements `+0x10`, clears object `+0x98`; no tail-zeroing. | `BINARY_HIGH` — Conceal + remover (this session) `0x005F4D30`/`0x0055BAE0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 6 | Active insertion point is **reveal**, not constructor: `Foot/BuildingClass::Unlimbo → TechnoClass::Unlimbo → ObjectClass::Reveal` first; failed Unlimbo deletes/uninits and creates no live entry. | `RESEARCH_HIGH` — ACTIVE_OBJECT_ORDER §3.3 `0x006F6CA0`,`0x004D7170`,`0x00440580` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 7 | `ObjectClass::UnInit`: Defuse bomb → EMP passengers if Foot → `Detach_From_All_Lists` → `vtable+0xD4` (Limbo→Conceal, unregisters) → clear IsAlive `+0x90` → append to **PendingDeleteList @ `0x00B0F69C`**. Delete is deferred, not inline free. | `BINARY_HIGH` — UnInit decompile (this session) `0x005F65F0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 8 | A tail append made before the forward loop reaches the old end runs **same pass** (count reload). Drives first-tick projectile timing. | `BINARY_HIGH` — scheduler + insert; AAHeatSeeker2 latency report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 9 | Scheduler does not repair the index after a `vtable+0x5C` call; a compacting remove at/before the current index can skip the shifted object. Mechanics proven; *which* AI bodies self-remove mid-pass in normal play is not enumerated. | `BINARY_HIGH` (mechanics) / `INFERENCE` (which callers) | confirmed / unknown | high / low | yes | IMPLEMENTATION_SAFE (mechanics); NEEDS_REINVESTIGATE (cases) |
| 10 | Map-load active order = loader sequence Terrain → (vein/tib) → Units → Aircraft → Infantry → Structures → Smudge, then per-section INI key order, then reveal timing. Not a sort-by-ID. | `RESEARCH_HIGH` — Full_Init `0x00686B20`; calls `0x00687A74..0x00687B13` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 11 | `PerTickUpdate` runs late in `Main_Tick` (after input/AI/`Map::Logic`/render, before service/network), and `g_CurrentFrameCounter` increments **late** + conditionally. PerTickUpdate sees the pre-increment frame. | `RESEARCH_HIGH` — Main_Tick `0x0055DC99..0x0055DCA3`, increment `0x0055DE73..0x0055DE81` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 12 | Global order inside `PerTickUpdate`: scenario timers → bridge-shroud recalc → tiberium growth → spread → bombs → teams → disk lasers → laser draw → lightning → EMP → **main object vector** → (non-local loop, modes ≠0,5) → tactical → crate regen → factories → houses → last-ref-object. | `RESEARCH_HIGH` — GLOBAL_SUBSYSTEM_ORDER table (dated today) `0x0055AFB0` | confirmed | medium-high | yes | IMPLEMENTATION_SAFE as target order; sequenced port required |
| 13 | `ObjectClass::Save`/`Load` do **not** serialize/restore `+0x98`, and `Load` does not reveal/register. The post-load active-vector rebuild owner is unidentified. | `RESEARCH_HIGH` (negative) / unknown (owner) — `0x005F6250`/`0x005F5E80` | confirmed (negative) / unknown | high / — | conditional | NEEDS_REINVESTIGATE |

## 1. Current Model

Every world object carries two independent bytes: **InLimbo `+0x81`** (visible/marked on the
playfield) and **logic-membership `+0x98`** (receives per-tick AI). They are toggled by different
functions and must not be conflated. `Reveal` clears InLimbo and — if the object's *type* is
logic-enabled (`type+0x234`) and game-mode gates pass — calls the adder to tail-append the object to
the singleton LogicClass vector (`0x0087F778`, items `+0x04`, count `+0x10`), setting `+0x98`.
`Conceal` runs the mirror: remover compacts the object out (preserving order, decrementing count,
clearing `+0x98`) and sets InLimbo. `Unlimbo` reaches the vector through `Reveal`, so the active
insertion point is reveal timing, never the constructor. `UnInit` (death/destruction entry) tail-calls
Limbo→Conceal to unregister, clears IsAlive `+0x90`, and queues the object on the deferred
PendingDeleteList rather than freeing inline.

The scheduler walks the vector forward by index, calling `vtable+0x5C` and **reloading count after
each call** — so same-tick tail appends are visible to the same pass, and compacting removes can shift
later entries relative to the already-incremented index. This loop is one stage in a fixed global
order (claim 12) that `Main_Tick` invokes late, under the **pre-increment** frame counter (claim 11).

## 2. Implementation-Safe Facts

Claims 1–8, 10, 11 are binary-verified with addresses and safe to build against now:
- Two distinct bytes: InLimbo `+0x81` vs logic-membership `+0x98`. Model both; do not collapse.
- Active list = separate insertion-ordered, membership-gated vector; tail-append on reveal,
  compacting-remove on conceal, count reloaded per object AI.
- Lifecycle wiring: reveal→register, conceal→unregister+InLimbo, unlimbo→reveal, uninit→limbo→
  deferred-delete. Type gate `type+0x234`; failed Unlimbo creates no live entry.
- Map-load order is loader-sequence + section-key + reveal timing (claim 10).
- Tick clock: expose the pre-increment frame for the whole tick; commit next frame late (claim 11).

## 3. Doc-Patch-Ready Facts (narrow corrections; not new investigation)

- `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` "LogicClass::AI — Global Tick Order" mislabels the
  function (it is `LogicClass::PerTickUpdate @ 0x0055AFB0`) and collapses the order to
  "objects → factories → houses," omitting tiberium/bombs/teams/lasers/lightning/EMP/tactical and
  placing lightning/EMP after factories (binary has them **before** the object vector). Supersede with claim 12.
- `UNITCLASS_GHIDRA_REPORT.md:318` and `INFANTRYCLASS_GHIDRA_REPORT.md:335`: "LogicClass::AI() tick
  loop → iterates all entities → calls AI on each" — replace with the live forward-vector contract (claim 1).

## 4. Stale / Superseded Claims

- "`binary_frame` advanced at start of `advance_tick` matches a 15 Hz native frame clock" — superseded
  by the late-increment contract (claim 11). Native holds the old frame through the whole tick.
- Any "Rust stable-ID sorted order == gamemd active-object order" — superseded by claim 10 (reveal-timing
  tail append, no sort).

## 5. Cross-Doc Conflicts

None on the normal path. The cluster cross-references cleanly and each doc declares its non-scope.
Two ordering docs coexist — `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER` (used here, dated today) and
`PERTICKUPDATE_FULL_ORDERING_LADDER`; both `ghidra/verified`. A line-by-line reconcile was not done
this pass; treat GLOBAL_SUBSYSTEM_ORDER as the working table and reconcile if a discrepancy surfaces.
Minor confidence nuance: the adder `0x0055BAA0` is not typed `__thiscall` in Ghidra, so its decompile
renders the arg order ambiguously; the `+0x98` membership semantics are certain (the cleanly-typed
remover confirms object`+0x98` and vector `+0x04/+0x10`), but the exact `(this=vector, obj, flag)`
signature is inferred by symmetry, not from a clean thiscall body.

## 6. Needs Re-Investigation (bounded targets)

- **Save/load active-vector rebuild owner** (claim 13). Negative facts are proven; the rebuild owner is
  unknown. → `/re-investigate post-savegame-load LogicClass active-object vector reconstruction owner`
- **Mid-pass self-removal cases** (claim 9): which high-frequency `vtable+0x5C` bodies (Anim, Bullet,
  Techno, debris self-delete) remove from the vector during the pass. → `/re-investigate class-specific mid-pass LogicClass self-unregister cases`
- **Unknown `FUN_*` callees** in the global order (e.g. `0x004ACAC0`, `0x0053A110`, `0x0054E4D0`,
  `0x005FF390`, `0x00554D50`) and the reverse `DAT_00B04BD4` loop's class.

## 7. Do-Not-Implement Notes

- Do not snapshot the active-object count at pass entry; reload after each object AI.
- Do not use `swap_remove` for the active list; native is order-preserving compacting remove.
- Do not treat `EntityStore` `BTreeMap` sorted iteration as the active-object vector, and do not keep a
  sorted-ID fallback (`live_object_order_snapshot` at `src/sim/world/mod.rs:622` currently does — DRIFT).
- Do not register limbo-created objects merely because they exist in storage
  (`spawn_object_limbo_at_height` calling `register_live_object` — DRIFT).
- Do not derive the next frame at tick start, or "fix" frame drift by subtracting 1 at call sites.
- Do not collapse the global order to "objects → factories → houses," and do not place lightning/EMP
  after the object vector.

## 8. Rust Delta Snapshot (current status, all unimplemented/DRIFT)

| gamemd contract | Rust today | Verdict |
|---|---|---|
| Live appendable membership-gated active vector | `EntityStore` BTreeMap + phased `advance_tick` | DRIFT |
| Reveal-time tail append, no sort | `live_object_order_snapshot` sorted-ID fallback | DRIFT |
| `+0x98` membership byte | `register_live_object` present, no membership bit | DRIFT |
| Global order (claim 12) | tiberium after combat; teams/AI near end; no bomb/EMP global phase | DRIFT / UNCHECKED |
| Late pre-increment frame | `binary_frame` derived at tick start; 45 Hz step vs 15 Hz frame | DRIFT |

## 9. Source Ledger

- **Live Ghidra (this session):** `0x005F4EC0`, `0x005F4D30`, `0x0055BAA0`, `0x0055BAE0`, `0x005F65F0`.
- **Docs (read in full):** `LOGICCLASS_PERTICKUPDATE_SCHEDULER`, `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN`,
  `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0`, `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK`.
- **Docs (cited):** `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0`, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES`,
  `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE`, `LOGICCLASS_VS_MAPCLASS`, `LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES`,
  `PERTICKUPDATE_FULL_ORDERING_LADDER`, `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY`, `GLOBAL_TIMING_MODEL`.
- **Rust surfaces:** `src/sim/world/mod.rs` (`advance_tick`, `register_live_object`, `live_object_order_snapshot`),
  `src/sim/entity_store.rs`, `src/sim/world/world_spawn.rs`, `src/app_sim_tick.rs`.
- **Globals/offsets:** LogicClass singleton `0x0087F778` (`+0x04` items, `+0x10` count); `ObjectClass+0x81`
  InLimbo, `+0x90` IsAlive, `+0x98` membership; `ObjectTypeClass+0x234` logic gate; PendingDeleteList `0x00B0F69C`;
  `g_CurrentFrameCounter 0x00A8ED84`.
