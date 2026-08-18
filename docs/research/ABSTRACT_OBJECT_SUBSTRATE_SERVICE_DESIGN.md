# Abstract/Object Substrate — Service Design (gamemd-native semantics, Rust-native structure)

**Status:** design / synthesis (study only — no code). Built 2026-05-30 from a 16-agent verify→map→synthesize→critique workflow, then a 7-agent research follow-up that resolved the open items against the live binary. All 12 adversarial-critic findings and all 7 follow-up results are folded in (see §9, §10).
**Rule:** Rust-native STRUCTURE, gamemd-native SEMANTICS.
**Authority order:** binary → Ghidra → docs. The §2 *WRONG/UNVERIFIABLE* subsection corrects stale claims in `ABSTRACTCLASS_GHIDRA_REPORT.md` / `OBJECTCLASS_GHIDRA_REPORT.md`.

> **Provenance honesty note.** An earlier version of this file contained a *fabricated* critic verdict written before the workflow's full output was available (the completion notification truncated mid-stream). This version is rebuilt from the agents' verbatim verified output; §9 carries the **real** verdict — **REVISE, "not ready to execute as written," 12 findings** — and the 2026-05-30 follow-up has since cleared most blockers.

> **Burden of proof.** Every difference defaults to **DRIFT** unless proven equivalent. Do not re-upgrade a downgraded clause without algebraic/empirical proof.

> **✓ OnBridge offset resolved (follow-up).** OnBridge is ObjectClass byte **`+0x8C`** (1 = on bridge). The "+0x23" some docs cite is the Ghidra decompiler's `int*`-stride rendering of the *same* byte (0x23 × 4 = 0x8C). Verified from raw bytes: ctor `0x005F396F` inits `byte[this+0x8C]=0`; Unlimbo `0x005F5986` writes `=1` on `cell.Flags(cell+0x140)&0x100`; GetHeight `0x005F5F7B` reads it and subtracts BridgeHeight (`DAT_00AC13BC`). Byte `+0x8D` is a *different* fall/height-settle flag set by Unlimbo/DropIn and consumed by `ObjectClass::AI` — do not conflate.

---

# PART A — gamemd-facing (verified)

## 1. Verified active-YR responsibilities of the substrate

`AbstractClass` → `ObjectClass` is the object-hierarchy root every world object inherits. In a live YR skirmish it owns:

1. **Root identity & RTTI.** COM-style MI base (`IUnknown`+`IPersistStream`+custom `IRTTITypeInfo`); 4-vtable-ptr header (`+0x00` primary, `+0x04/08/0C` MI sub-objects). Primary vtable `0x007E1F50` (`read_memory len 48`); secondaries `0x007E1F34/2C/24`. RTTI delivered via the per-type virtual at primary-slot **+0x2C** (base = ret-0 stub `0x004C9150`, subclass-overridden — `ReceiveDamage 0x005F5390`/`Destroy` switch on 6/0xF/0x24). **Not** a hand-rolled type directory.
2. **Per-instance heap identity.** `+0x0C`-reached instance id from the `ScenarioClass+0x214` monotonic counter (`AssignUniqueID 0x00410230`, post-inc `0x0068BCB0`, `GetID 0x00410220`). Distinct from the per-type WhatAmI enum.
3. **Physical presence & coordinates.** World lepton triple at **+0x9C/A0/A4** (`GetCoords 0x005F6690`; ctor seeds from `DAT_00AC1380/84/88`). Layer (Ground=1/Air=4) **derived from Z** via `In_Which_Layer 0x005F42E0`. Sub-objects at `+0x3C/+0x50` (`FUN_00405BE0`) — role UNVERIFIED.
4. **Health.** Instance HP at **+0x6C** (`ReceiveDamage 0x005F5390`, `GetHealthRatio 0x005F5C60` reads `param_1[0x1B]`). **MaxHealth is a TYPE field** (`Type+0xA0` Strength); no per-object `Get_Max_Health`.
5. **Cell occupancy.** `Mark_Put 0x005F60A0` sets cell flag `0x40`; `Mark_Remove 0x005F6120` clears it; bridge-elevated → cell `+0x128` vs `+0x124`.
6. **Limbo↔active lifecycle.** Authoritative InLimbo bit **+0x81** (born=1). `Reveal 0x005F4EC0` clears; `Conceal 0x005F4D30` sets; `Unlimbo 0x005F5940` initial placement; `UnInit 0x005F65F0` teardown. All full-body decompiled.
7. **Active-vector membership & ticking.** Gated by 1-byte boolean **+0x98**. `FUN_0055BAA0` add-once / `FUN_0055BAE0` compacting-remove both test it. ✓ **Follow-up resolved:** the per-tick consumer is `0x0055AFB0` (in `Main_Tick 0x0055D360`), iterating the `DynamicVectorClass` **embedded in the LogicClass object** (data `+0x04`, live count `+0x10`/`+0x0C`); it re-reads the count each iteration → **same-pass** (see C9).
8. **Selection.** Separate bit **+0x83**, list `g_CurrentObjects` (count `0x00A8ECC8`). `Select 0x005F4520` / `Deselect 0x005F44A0` (O(n) shift-down). Gated by `CanBeSelected 0x005F6C30` (Type+0x230). Orthogonal to InLimbo/active.
9. **Save/load.** IPersistStream `Load 0x005F5E80` / `Save 0x005F6250` / `GetSizeMax 0x004103E0` / `IsDirty 0x00410450`. COM refcount dead (const-1 stubs).
10. **Type resolution.** Per-class linear scan by name (`UnitTypeClass__FindOrAllocate 0x007480D0` → compare entry `+0x24` via `FUN_007C8D20`; miss → `operator new` + ctor). **No** global `Find_By_ID`. ✓ The comparator is **case-insensitive** (see C13).

## 2. Full substrate inventory

### (a) Fields / offsets

**AbstractClass** (size `0x24` — *UNVERIFIABLE: no `sizeof`/`new` site read; abstract class*):

| Off | Field | Ctor `0x00410170` | Verified |
|---|---|---|---|
| +0x00 | primary vtable `0x007E1F50` | set | VERIFIED |
| +0x04/08/0C | MI sub-object vtables `0x007E1F34/2C/24` | set | VERIFIED |
| +0x10 | sentinel | `0xFFFFFFFF` | VERIFIED |
| +0x14 | status-flag byte | low-3-bits cleared (`AND 0xF8`) | VERIFIED |
| +0x18 / +0x1C | ptr fields (one swizzled in Load) | `0` | VERIFIED |
| +0x20 | dirty byte (`IsDirty`) | `0` | VERIFIED |

**ObjectClass** (from `disassemble 0x005F3900`):

| Off | Field | Ctor init | Verified |
|---|---|---|---|
| +0x14 | status byte, bit `0x2` (IsObject) | `\|= 2` | VERIFIED |
| +0x24/28/2C | cell-list Next/Prev candidate | `0` | init VERIFIED, **role UNVERIFIED** |
| +0x34 | height/Z candidate | `0` | init VERIFIED, **role UNVERIFIED** |
| +0x3C, +0x50 | sub-objects (`FUN_00405BE0`) | constructed | present VERIFIED, **role UNVERIFIED** |
| +0x64 / +0x94 | -1 sentinels | `0xFFFFFFFF` | VERIFIED |
| +0x6C | **Health** | (set later) | VERIFIED |
| +0x81 | **InLimbo** | `1` | VERIFIED |
| +0x83 | **IsSelected** | `0` | VERIFIED |
| +0x8C | **OnBridge** (1=on bridge); the "+0x23" some docs cite = decompiler int*-stride view of the same byte (0x23×4=0x8C) | `0` (ctor `0x005F396F`) | VERIFIED (write `0x005F5986`, read `0x005F5F7B`) |
| +0x8D | fall/height-settle active flag; set by Unlimbo/DropIn, consumed by `ObjectClass::AI`, then cleared after landing | `0` | VERIFIED |
| +0x90 | **IsAlive** (UnInit clears) | `1` | VERIFIED |
| +0x98 | **active-vector membership** | `0` | VERIFIED |
| +0x9C/A0/A4 | **coordinate triple** | `{0,0,0}` | VERIFIED |
| +0xA8 | LineTrail/effect-owner ptr | `0` | VERIFIED (reader `0x005F3D90`) |

### (b) Dispatch surface

**AbstractClass primary vtable `0x007E1F50` (12 slots, `read_memory`):** QueryInterface `0x410260` · AddRef→1 `0x410300` · Release→1 `0x410310` · GetClassID stub `0x4C9150` · IsDirty `0x410450` · Load stub · Save stub · GetSizeMax `0x4103E0` · scalar-deleting dtor `0x4105A0` · 2 empty RETs · RTTI-dispatch stub `0x4C9150` (+0x2C, subclass-overridden).
**Secondary IRTTITypeInfo `0x007E1F34`:** QI/AddRef/Release thunks, WhatAmI-adjustor `0x410210`, GetID `0x410220`, AssignUniqueID `0x410230`.

**ObjectClass primary vtable `0x007EF060` (122 slots, `read_memory len 512`).** Body-verified load-bearing slots: Load(5) `0x5F5E80` · scalar dtor(8) `0x5F6DC0` · Save(13) `0x5F6250` · GetCoords(17) `0x5F6690` · AI(23) `0x5F3E70` · GetType(34) base-stub `+0x88` · Conceal(53) `0x5F4D30` · Reveal(54) `0x5F4EC0` · In_Which_Layer(56) `0x5F42E0` · Mark_Put(60) `0x5F60A0` · Mark_Remove(61) `0x5F6120` · CanBeSelected(78) `0x5F6C30` · Select(83) `0x5F4520` · Deselect(84) `0x5F44A0` · Receive_Damage(91) `0x5F5390`. Other cross-system points: `+0x2C` RTTI, `+0x124` cell-mark, `+0x1AC` cell-blocked gate, `+0x1B4` commit-position, `+0x1C8` GetHeight, `+0x280` pre-limbo hook.

### (c) Global helpers
`FUN_0055BAA0` add-once (test +0x98, inner `0x005519B0`, set +0x98) · `FUN_0055BAE0` compacting remove (gate +0x98, find via vtable+0x10, shift-left, clear +0x98) · **`FUN_005519B0` inner DynamicVector add** (sorts only when its flag is nonzero; the active-vector path passes no sort → tail append; see C8) · `FUN_007258D0` Detach_From_All_Lists (RTTI-keyed observer-notify via vtable+0x28) · `FUN_0068BCB0` counter post-inc · `FUN_007C8B3D` free · **`FUN_007C8D20` name compare — VERIFIED case-INSENSITIVE** (OR-0x20 fold; see C13).

### (d) Singleton state & registries (instance side)
ObjectClass ctor inline-appends `this` to FOUR fixed-array registries in order: `0x00A8E360` g_ObjectClass_Array · `0x00B0F720` removal-observer · `0x00B0F670` master Abstract registry · `0x00B0F618` g_TagClass_RemoveListeners. Pending-delete: `0x00B0F698`. FootClass adds `0x008B3DC0` + g_TeamClass_RemoveListeners `0x00B0F5D8`. Selection: `g_CurrentObjects` (data `0x00A8ECBC`, count `0x00A8ECC8`). The **+0x98-gated active list** is the `DynamicVectorClass` embedded in the LogicClass object (data `+0x04`, count `+0x10`), iterated by `0x0055AFB0` — SEPARATE from the 4 master registries.

### (e) Static TYPE heaps & Find
Master abstract-type vector `0x00A8E968` · object-type vector `0x00AC1418` · techno-type extra (header `0x00A8EB00`). Membership LAYERED/cumulative (a UnitType lives in 4 type registries). Type ID at type `+0x24` (strncpy 0x18), label `+0x64`. Find = per-class linear scan by `+0x24` (`FindOrAllocate`→ptr; `Find_By_Name_Index`→idx/-1), comparator case-insensitive. Instance→type at TechnoClass `+0x14C` (cache `+0x21C`).

### (f) Lifecycle composites (ordered)
- **Reveal `0x005F4EC0`:** reject null-coord/`!g_GameActive` → require +0x81 → non-editor cell-blocked gate `+0x1AC` (abort if blocked) → clear +0x81/+0x80 → compute display coord (`Type+0x88→+0x6C`) → **commit position +0x1B4** → **mark cell +0x124(1)** → on fail RE-SET +0x81 (rollback) → if eligible: display submit + `FUN_0055BAA0` (active-vector add, gated by Type+0x234) + AlphaShape/LineTrail/DirtyRect → return 1.
- **Conceal `0x005F4D30`:** early-out if `!g_GameActive`/already +0x81 → **Deselect +0x150 (FIRST)** → unmark +0xDC(1) → clear +0x124(0) → RemoveFromLayer → Anim::Detach → Voc::Stop → fixed-cell vacate `FUN_0055BAE0` if Type+0x234 → DirtyRect → +0x11C → **set +0x81 (near LAST)** → +0x80=0.
- **Unlimbo `0x005F5940`:** playfield gate → set +0x8D=1 → Get_Cell_At → **bridge gate** (cell.Flags&0x100 → set OnBridge `+0x8C`=1, abort if &0x200 clear) → if Foot: zone-occupy + passability → **MARK +0xD8(coord,0x80)** → **commit position +0x1B4** → spawn anim. *(OnBridge before mark; mark before position — reverse of Reveal.)*
- **UnInit `0x005F65F0`:** Defuse bomb → EMPPassengers(0) if Foot → **Detach_From_All_Lists (BEFORE Limbo)** → Limbo +0xD4 (→Conceal→Deselect) → clear IsAlive +0x90=0 → **ENQUEUE pending-delete (never free inline)**.
- **TechnoClass Limbo tail `0x0065AA80`:** if not InLimbo, fire pre-limbo hook +0x280(3), then tail-call Conceal.

#### WRONG / UNVERIFIABLE
- **WRONG:** vtable anchor `0x0057B1A4` — not a vtable. Correct primary = `0x007E1F50`.
- **WRONG:** `0x004101F0` "scalar-deleting destructor" — only the vtable-reset body (no free). Real deleting dtor = slot 8 (`0x4105A0` / ObjectClass `0x5F6DC0`).
- **WRONG:** `0x005F5C60`/`0x005F5CD0` as vtable slots — both NON-virtual helpers; no per-object `Get_Max_Health`.
- **WRONG:** coords `+0x4C` / Strength `+0x5C` — coords are `+0x9C/A0/A4`; Health is `+0x6C`.
- **WRONG:** counter `ScenarioClass+0x1D10` — correct `+0x214`.
- **UNVERIFIABLE (still):** AbstractClass size `0x24`; `+0x24/28` Next/Prev; `+0x34` height; `+0x3C/+0x50` sub-objects; WhatAmI exact slot; base Limbo/Unlimbo slots.
- **RESOLVED (2026-05-30 follow-up):** OnBridge = `+0x8C` (no +0x23 conflict); `FUN_007C8D20` casing = **case-insensitive**; same-pass rule = **same-pass**; the active/logic vector is the `DynamicVectorClass` embedded in the LogicClass object (data `+0x04`, count `+0x10`), iterated by `0x0055AFB0`.

## 3. Active YR vs inactive/legacy (TS)

| ACTIVE (port MUST reproduce) | DORMANT_TS / vestigial (do NOT implement) | Verdict |
|---|---|---|
| QI for IPersistStream + IRTTITypeInfo (live save/load + RTTI entry) | COM refcount: AddRef/Release const-1 no-ops | ACTIVE QI shell, DORMANT refcount |
| IsDirty real one-liner | IsDirty has no proven live caller | method ACTIVE, runtime-use UNVERIFIED |
| GetSizeMax (OleSave path) | GetClassID base ret-0 stub | ACTIVE / DORMANT base stub |
| Save/Load real impls (slots 5/13) | shared `Stub__ReturnZero 0x4C9150` base virtuals | ACTIVE / DORMANT base stubs |
| InLimbo lifecycle, Reveal/Conceal/Unlimbo/UnInit | legacy `+0x14` low-3-bit flags (only 0x2/0x4 used) | ACTIVE lifecycle; `+0x14` mostly dormant |
| Active-vector add/remove via +0x98 | subterranean/tunnel object lifecycle | ACTIVE membership; DORMANT_TS subterranean |
| Selection (+0x83, g_CurrentObjects) | fog-of-war "previously seen" darkening (SpecialFlags&0x1000) | ACTIVE selection; DORMANT_TS fog |

*Note:* subterranean/fog are DORMANT_TS by standing project policy, not a fresh read this session. COM/QI is the one place ACTIVE and DORMANT coexist in one function (QI live; the AddRef it calls is a no-op). Rust implements NO refcounting.

## 5. Gamemd-native behavior contract

Reproduce the observable contract, not the C++ mechanism. Follow-up resolutions marked ✓.

- **C1 — Root identity.** Stable per-INSTANCE id (monotonic) + per-TYPE discriminant, distinct. *Rust:* `stable_id:u64` + `EntityCategory`. ⚠ **(critic #10) APPROX, not FAITHFUL:** Rust never reuses ids and resolves stale refs to `None`; gamemd can dangle/reuse a heap slot. A gamemd dangling read landing on a reallocated live object is a (rare) observable Rust cannot reproduce — label intentional-divergence with a frequency note.
- **C2 — RTTI dispatch.** Type-bucket behavior by discriminant, base = unknown/0. *Rust:* `match category` + `Option::is_some()` — FAITHFUL observable; no type-system guarantee a category-wrong `Option` stays None (caller discipline).
- **C3 — Ordered lifecycle FSM.** LIMBOED→(Unlimbo)PLACED→(Reveal)ACTIVE→(Conceal)LIMBOED→(UnInit)QUEUED→(flush)DESTROYED. Born InLimbo. *Rust:* primitives exist but NO unified InLimbo flag (inferred from 4 gates) — **APPROXIMATION, parity-watch.**
- **C4 — Reveal step order.** Position commit precedes cell-mark; InLimbo cleared before both; rollback on mark fail. *Rust:* reveal decoupled from cell marking; **MISSING the CanEnterCell/Mark-success/Type+0x234 gate-chain. DRIFT.**
- **C5 — Conceal step order.** Deselect FIRST; InLimbo set near LAST. *Rust:* conceal = active-vector delist only; deselect-first not centralized. **DRIFT.**
- **C6 — Unlimbo step order.** OnBridge (`+0x8C`) from cell flags BEFORE mark; mark BEFORE position. *Rust:* occupancy at spawn; bridge state triplicated — **APPROXIMATION.**
- **C7 — UnInit / deferred free.** Detach-all-lists → Limbo→Conceal → clear IsAlive → enqueue; never free inline; one-tick valid-but-dead window. ⚠ **(critic #6) HARD REQUIREMENT:** conceal (logic delist + occupancy unmark + link teardown) stays **SYNCHRONOUS** at uninit time; **only the slot free defers**. A Dying unit must NOT keep ticking/blocking cells for a tick. *Rust:* synchronous remove today; no general PendingDeleteList. **DRIFT.**
- **C8 — Active-vector append & membership.** TAIL-append in register order, +0x98-gated idempotent, order-preserving COMPACTING removal. *Rust:* `LogicVector` (tail push, retain-compacting, no sorted fallback) + `in_logic_vector`. ✓ **(critic #1 mostly resolved) tail-append in observable effect → NOT DRIFT.** Both add paths into the active/logic vector — `Reveal 0x005F4EC0` and `0x005F3D90` — go through the add-once wrapper `FUN_0055BAA0`, whose call site pushes only the element + `this` (no sort-flag arg; `RET 0x8`; `disassemble_function 0x0055BAA0`) into `FUN_005519B0` (which sorts only when its flag is nonzero; `decompile_function 0x005519B0`). So no sort occurs and Rust tail-append is correct. ⚠ Residual (MEDIUM): the true arity of `FUN_005519B0` (is the decompiler's 3rd `param_3` a real parameter?) wasn't re-confirmed (MCP dropped mid-session) — re-verify, and check no *other* caller passes a nonzero sort flag. *(An interim agent claim of an explicit `PUSH 0` sort-flag in the wrapper was a decompiler-adjacent misread and was retracted.)*
- **C9 — Same-pass visibility.** ✓ **(critic #3 RESOLVED) SAME-PASS.** The per-tick consumer `LogicClassPerTickUpdateLiveVector 0x0055AFB0` (sole sim stage in `Main_Tick 0x0055D360`; `get_function_callers 0x0055AFB0`) iterates the LogicClass active-object `DynamicVectorClass` (data `+0x04`, count `+0x10`) **re-reading the live count every iteration** (`disassemble_function 0x0055AFB0`: loop bottom `MOV EAX,[EDI+0x10]` at `0x0055B613`, then `CMP ESI,EAX / JL 0x0055B608`; data ptr re-loaded each iter at `0x0055B608`; dispatch `call [EDX+0x5C]` at `0x0055B610`). AddItem tail-appends, so an object appended mid-pass (unit produced/revealed mid-tick) IS updated the SAME tick. *Rust:* the spine currently uses FROZEN snapshots (`live_object_order_snapshot` mod.rs:807); a re-read variant (`for_each_live_object` mod.rs:825) exists but is unused → **for the active-object AI stage this is DRIFT** (a mid-tick-spawned unit acts one tick late). Port must iterate with re-read length for the AI/update stage. ⚠ Residual: only the AI stage is proven same-pass; removal-during-iteration compaction (`FUN_0055BAE0` shift-down vs the loop index) wasn't fully traced — verify the Rust removal-during-iteration matches. Keep a cross-engine test (unit produced mid-combat: does it act this tick?).
- **C10 — Registry membership invariants.** At construction: 4 master registries (+2 for Foot); +0x98 active list separate/opt-in. On destroy: master removed unconditionally, +0x98 conditionally. ⚠ **(critic #9) specify detach ordering:** state whether observer NotifyOfRemoval fires before logic/occupancy delist; add a mutual-reference same-tick death test (two units kill each other). *Rust:* keep store / active-vector / selection as independent sets (don't couple selection to construction).
- **C11 — Occupancy / cell-list ordering.** Buildings tail, non-buildings head, per layer; Mark sets flag 0x40. *Rust:* `CellListInsertion::from_category` + `occupancy_enter_order` — FAITHFUL on list order. ⚠ **(critic #2 — DRIFT/UNCHECKED, refined mechanism):** the coarse occupation BIT-flags are unmodeled, and the **0x20 occupation pair is asymmetric** — `Mark_Occupation 0x007441B0` sets the high-cell bit (cell+0x128) only when BOTH height AND `cell+0x140&0x100` (bridge flag) hold; `Clear_Occupation 0x00744210` clears it on **height alone, no bridge-flag check** (`disassemble 0x007441B0`/`0x00744210`). The separate 0x40 pair (`Mark_Put 0x005F60A0` / `Mark_Remove 0x005F6120`) is symmetric (both gate on 0x100). A bit can only strand if the cell's bridge flag is cleared while a unit still satisfies the height condition at Clear time — i.e. **bridge destroyed under an elevated unit**. That consequence is **UNCONFIRMED from the binary** (the destroy-path occupancy cleanup wasn't located). Keep DRIFT/UNCHECKED; **an in-game test is required** (place unit on bridge, destroy bridge, compare underlying-cell passability vs gamemd).
- **C12 — Selection independence.** Distinct bit; Deselect O(n) shift-down + clears DisplayClass LastRefObject; downstream gameplay gates on InLimbo not IsSelected. *Rust:* `selected:bool` on the entity, non-authoritative — **DIVERGENT;** keep selection above the sim boundary. ⚠ **(critic #5) specify the hook:** once selection lives above sim/, the app must observe conceal/uninit and clear selection **+ LastRefObject** in the SAME frame BEFORE render reads selection (else a one-frame stale-selection/last-ref glitch).
- **C13 — Name→type resolution.** Per-class linear scan by `+0x24` ID; miss allocates+self-registers; no global Find_By_ID. *Rust:* single `RuleSet` (String-keyed) + per-category accessors — FAITHFUL as consolidated registry, but **DIVERGENT casing**. ✓ **(critic #8 RESOLVED) `FUN_007C8D20` is case-INSENSITIVE** — stricmp-style, OR-0x20 case-fold on both operands before byte compare (`decompile_function 0x007C8D20`; the shared FindOrAllocate matcher across Aircraft/Anim/Building/Infantry/etc.). gamemd matches names case-insensitively, so the Rust case-SENSITIVE `object()`/`weapon()`/`warhead()`/`projectile()` raw gets ARE DRIFT; make all name→type resolution case-insensitive. Slice 8 may proceed.
- **C14 — Instance→type linkage.** Stable reference resolved once. *Rust:* `type_ref:InternedId` two-hop resolve — **APPROXIMATION;** a single intern-id→type-index handle table is the main refactor opportunity.
- **C15 — Save/load round-trip.** Authoritative state round-trips; derived caches rebuilt, never persisted as truth. Active-vector ORDER authoritative (serialized verbatim, no sort); heap id + enter-order + RNG cursors round-trip. *Rust:* `in_logic_vector` serde-skip rebuilt; `occupancy_enter_order` serialized→grid rebuilt; 3 RNG streams serialized+hashed — FAITHFUL on the authoritative-vs-derived split; mechanism DIVERGENT by design (engine-private saves; bump SNAPSHOT_VERSION on any field reorder — bincode is positional).
- **C16 — Despawn ordering.** Conceal precedes freeing the slot; occupancy/logic removal precedes entity removal. *Rust:* `uninit` conceals-then-removes (centralized) — FAITHFUL; only the synchronous-vs-deferred mechanism gap remains (see C7).

**Substrate-level deltas the port owes (from C3–C14):** (1) unified InLimbo predicate (C3); (2) Reveal gate-chain (C4); (3) deselect-first centralized in conceal (C5); (4) deferred PendingDeleteList + one-tick window (C7); (5) **same-pass re-read iteration for the AI stage** (C9 — now resolved); (6) case-insensitive name→type (C13 — verified); (7) optionally collapse dual-id linkage (C14). Selection (C12) and save/load mechanism (C15) are intentional Rust-native divergences — keep.

---

# PART B — Rust-facing (verified against the live tree)

## 4. Comparison against current Rust architecture

One wide `GameEntity` (game_entity.rs:133), `BTreeMap` `EntityStore` (entity_store.rs:33), substrate contract scattered across `Simulation` helpers, `OccupancyGrid`, `LogicVector`, `StringInterner`, `RuleSet`. Faithful already: active-vector tail-append/compacting-remove, conceal-before-free, occupancy enter-order determinism. Gaps cluster where one gamemd authority owned state now split across 2–3 Rust structures kept consistent by convention.

Legend: FAITHFUL / APPROX (hand-synced) / COLLAPSED (deliberate flatten) / DIVERGENT (can differ observably) / MISSING.

| Responsibility (§5) | Rust location | Fidelity | Gap |
|---|---|---|---|
| RTTI discriminator | `EntityCategory` (map/entities.rs:19); `category` (game_entity.rs:154) | COLLAPSED | "which class" = `category`+`Option::is_some()`; category type lives in the **map parser**. ✓ **critic #7 CONFIRMED via dep check:** defined in `map/entities.rs`, consumed one-way by `sim/`; moving it into `sim/` would invert `map/`→`sim/`. Keep in `map/`; derive a sim-side `CapabilityFlags` at spawn (see §6) |
| Per-INSTANCE id | `stable_id` (game_entity.rs:138); `allocate_stable_id` (mod.rs:700-704) | ⚠ APPROX (critic #10) | monotonic, never reused; stale refs degrade to `None` where gamemd dangles — intentional divergence, not parity |
| Object container / iteration | `EntityStore` BTreeMap (entity_store.rs:33) | FAITHFUL | BTreeMap order ≠ active-vector order |
| Per-house membership | `by_owner` rebuilt once/tick (entity_store.rs:138) | APPROX | deferred-rebuild: insert/remove don't maintain it; mid-tick capture reads stale until rebuild |
| Active-vector flag (+0x98) | `in_logic_vector` serde-skip (game_entity.rs:183) | FAITHFUL | two views of one set hand-synced; needs the debug invariant (mod.rs:786-800) |
| Active-vector **order** | `LogicVector` (logic_vector.rs); snapshot mod.rs:807-832 | FAITHFUL (order) | ⚠ iteration is FROZEN-snapshot; gamemd AI stage is SAME-PASS (C9) → DRIFT for mid-tick spawns acting this tick |
| `Reveal` gate-chain | `reveal`→`register_live_object` (mod.rs:707-732) | DIVERGENT/MISSING | Rust reveal = active-vector append only — no CanEnterCell, no Mark-success, no Type+0x234, no rollback; occupancy is a separate caller step |
| `Conceal` teardown | `conceal`→`unregister_live_object` (mod.rs:716-738) | APPROX | only active-vector delist; deselect-first not centralized |
| `Unlimbo` placement | `unlimbo` == alias for `reveal` (mod.rs:742-744); init front-loaded into spawn | COLLAPSED/DIVERGENT | spawn order `insert→reveal→count→occupancy` (world_spawn.rs:241-244) lands occupancy AFTER active-vector → re-entrant observer can see in-logic-not-in-occupancy |
| `Limbo`/general InLimbo | no primitive; inferred from 4 gates | APPROX/MISSING | **central limbo-modeling gap;** first place to look for limbo parity bugs |
| `UnInit` teardown | `uninit` (mod.rs:879-897): count→occupancy→radio→conceal→**immediate** remove | DIVERGENT | conceal-before-free FAITHFUL; **deferred-free MISSING** (gamemd enqueues, one-tick window) |
| Detach_From_All_Lists | partial; other id links nulled only via failed lookup | APPROX | `last_attacker_id`/`capture_target`/`bunker_occupant`/`garrison_original_owner` NOT nulled on despawn |
| Cell occupier list | `OccupancyGrid` + `CellListInsertion::from_category` (occupancy.rs:36) | FAITHFUL (list order) | ⚠ coarse occupation **bits** + 0x20 Mark/Clear bridge asymmetry NOT modeled → **DRIFT, needs destroyed-bridge test** (C11) |
| `occupancy_enter_order` | field (game_entity.rs:185) + counter (mod.rs:762) + rebuild sort (occupancy.rs:114) | FAITHFUL but ad-hoc | counter threaded `&mut` by hand; ✓ confirmed hashed (world_hash.rs:49,387) — that part is sound |
| Bridge/OnBridge layer | triplicated `on_bridge`/`bridge_occupancy`/`locomotor.layer` | APPROX | one source of truth missing |
| name→TYPE resolution | `RuleSet` per-category `HashMap<String,T>` | DIVERGENT | dual-id split (two-hop); **inconsistently cased** accessors (gamemd is case-insensitive — C13); ~77 files re-resolve by `&str` on hot paths |
| Instance→TYPE link | `type_ref:InternedId` (game_entity.rs:152) | APPROX | u32 handle, not cached `&ObjectType`; no intern-id→type-index table though `intern_all_ids` seam exists |
| Save/Load | serde on `Simulation` + `rebuild_caches_after_load` (mod.rs:1001-1031) | DIVERGENT (acceptable) | no Save vtable/swizzle (u64 ids need none); engine-private saves |
| Save/Load skips | serde-skip caches rebuilt on load | APPROX | inconsistent rebuild placement; `particle_systems` restored EMPTY (DIVERGENT for live-particle mid-match saves) |
| App-layer selection | `selected:bool` on the entity (game_entity.rs:177) | DIVERGENT | non-authoritative app flag inside authoritative sim struct, 2 writers; belongs above sim/ |

## 6. Rust-native replacement boundary (design)

**Goal:** one module owns the §5 contract — lifecycle FSM, active-vector ordering+membership, registry membership, the deferred-delete queue, and the name→type seam — so no other module re-implements or hand-syncs any of it. No vtables/COM/`dyn`; dispatch stays `match category` + `Option::is_some()`. Entirely in `sim/` (no render/ui/audio/net; `rules/` stays one-way below).

```
 sim/  (no deps on render, ui, audio, net)
 ┌──────────────────────────────────────────────────────────────┐
 │ Simulation (world/mod.rs) — orchestrator                      │
 │   substrate: ObjectSubstrate  ◄── the ONE owner of contract   │
 │     store:   EntityStore   (BTreeMap + by_owner)              │
 │     logic:   LogicVector   (active order; +0x98 flag flipped  │
 │                              ONLY here; re-read iteration)     │
 │     occupancy: OccupancyGrid (cell lists + enter-order ctr)   │
 │     pending_delete: Vec<u64> (PendingDeleteList)             │
 │     ids:     StableIdAllocator (monotonic, no-reuse)         │
 │   API (the ONLY way to change presence):                      │
 │     unlimbo / reveal / conceal / uninit /                     │
 │     flush_pending_deletes / move_cell / change_owner          │
 │   rules/ ──one-way──► RuleSet (String-keyed)                  │
 │                       + TypeHandleTable (sim/, InternedId→idx) │
 └──────────────────────────────────────────────────────────────┘
   callers (movement/combat/production/spawn/AI) NEVER touch
   store.insert / logic.push / occupancy.add directly — only the API.
```

**Critical borrow discipline.** `ObjectSubstrate` owns `store`/`logic`/`occupancy`/counters; `houses` stays on `Simulation` and the substrate takes `&mut Houses` for count updates (preserves layering). Every API method takes what it needs, mutates, and returns — it never holds a borrow across a sim-system call, so gameplay systems still `store.get_mut(id)` freely between transitions. (Explicit anti-god-object rule: storage stays independently borrowable.)

**The single `Presence` FSM** replaces the four scattered limbo gates (one authoritative `presence` field on `GameEntity`, serde-skip, rebuilt on load): `Limbo | InCell | Concealed | Dying`. Each transition validates the source state and commits in native order. `unlimbo`/`reveal` return `Err(PlaceReject)` and roll back `Presence` on Mark failure (C4). `uninit` does detach-all-links → conceal → IsAlive=0 → **enqueue** (C7); `flush_pending_deletes` drains at the cleanup phase.

Enforcement in one place:
- **FSM + active-vector ordering.** `logic.push/remove` and `in_logic_vector` mutate ONLY inside the API; the debug invariant becomes an internal assertion. `for_each_live_object`/`live_object_order_snapshot` move onto the substrate so the same-pass-vs-snapshot choice is in exactly one place. ✓ **(critic #3 / C9 RESOLVED) gamemd's active-object AI stage is SAME-PASS** — consumer `0x0055AFB0` re-reads live count each iteration, so a unit revealed mid-tick acts the same tick. The substrate's active-object iteration should use a **re-read length** (`for_each_live_object`), NOT a frozen snapshot, for the AI/update stage. ⚠ Caveat: gamemd has one combined LogicClass AI stage; the Rust spine is phase-split (movement/combat/… each snapshot independently) — decide per-phase whether same-pass matters; the mid-tick-spawn-acts-this-tick observable applies to the AI/update stage specifically.
- **Registry membership.** `by_owner` + owned counts updated **incrementally** on `unlimbo`/`uninit`/`change_owner` (retires the per-tick rebuild + mid-tick-staleness footgun); `rebuild_owner_index` survives only as deserialize finalizer.
- **Occupancy + enter-order.** `add`/`remove`/`move_cell` own the counter; movement calls `substrate.move_cell(id, to)` instead of threading `&mut u64`. `CellPlacement` carries the resolved layer/sub-cell so FirstObject/AltObject policy lives behind the boundary.
- **Deferred delete.** `uninit` enqueues; `advance_tick` flushes at cleanup — reproducing the one-tick `Dying` window. ⚠ **(critic #6)** conceal/occupancy-unmark/link-teardown happen synchronously at `uninit`; only the slot free defers.

**`TypeRegistry` + `TypeHandleTable`.** Keep `RuleSet` String-keyed in `rules/`. Add ONE canonical, **always case-insensitive** resolver `RuleSet::type_handle(&str)` (✓ verified gamemd is case-insensitive, C13) and a `TypeHandleTable` in `sim/` built at `intern_all_ids` (`InternedId→TypeHandle(index)`); entity→`&ObjectType` in one hop. Respects one-way `rules/`→`sim/`.

**Category dispatch.** Keep `EntityCategory`+`Option<T>`; add `dispatch_category(id)->(EntityCategory, CapabilityFlags)` (bitset derived once at spawn) so scattered `category==X && opt.is_some()` checks route through one audited surface. ✓ **(critic #7 CONFIRMED)** do NOT move `EntityCategory` into `sim/` — verified defined in `map/entities.rs` and produced by the map parser; `sim/` imports it from `map/`, not vice versa (sim/components.rs:14 says verbatim "depends on map/ (EntityCategory type)"; sim/components.rs:117-119 already wraps it as `Category(pub EntityCategory)`). Moving it would invert layering. If a layer-neutral home is desired, push DOWN to `rules/` or `util/`, never UP into `sim/`.

## 7. Old ad-hoc Rust logic to retire

1. **LogicClass membership split** — `in_logic_vector` (game_entity.rs:180-184) + `logic`, hand-synced (mod.rs:707-725, 1038), needs debug invariant (mod.rs:786-800). → substrate owns both atomically.
2. **`selected:bool` in the sim entity** (game_entity.rs:172-177), 2 writers. → move selection above sim/ (with the C12 deselect-first/LastRefObject hook).
3. **`occupancy_enter_order` threaded by hand** through movement (movement_step/tick.rs). → `substrate.move_cell()` owns the counter.
4. **`by_owner` deferred-rebuild cache** (entity_store.rs:30-32; rebuilt mod.rs ~1574). → incremental `change_owner`/`unlimbo`/`uninit`; rebuild only as deserialize finalizer.
5. **Bridge-layer triplication** (`on_bridge`/`bridge_occupancy`/`locomotor.layer`). → `CellPlacement`/occupancy owns layer; `on_bridge` derived.
6. **Scattered cross-entity id links, partial despawn cleanup** (`last_attacker_id`/`capture_target`/`bunker_occupant`/`garrison_original_owner`/`radio_contacts`). → one `detach_all_links(id)` pass in `uninit` (⚠ critic #9: define its order vs observer-notify).
7. **`capture_target` overloaded** (engineer-capture vs CABHUT repair). → distinct mission-intent enum.
8. **`occupancy_list_layer()` layer policy as an entity method** (game_entity.rs:601-625). → `CellPlacement` resolution behind occupancy boundary.
9. **Per-spawn manual 4-step** (world_spawn.rs:241-244) + the near-duplicate limbo-spawn fork. → one `unlimbo()`/`create_limbo()` pair, difference = one flag.
10. **`rebuild_caches_after_load` (7-arg)** + standalone `rebuild_logic_membership` (mod.rs:1001-1038), inconsistent rebuild placement. → substrate owns the post-load finalizer.
11. **Inconsistently-cased / dual-id type accessors** (case-sensitive `object()/weapon()/warhead()/projectile()` vs case-insensitive variants; two-hop on ~77 files). → one case-insensitive `type_handle` + `TypeHandleTable`; retire `Owner(InternedId)` wrapper (components.rs:80) + duplicate `intern_owner`/`intern_type` (app_commands.rs).

## 8. Migration slices & acceptance tests

Each slice independently shippable, reversible, behavior-preserving. ⚠ **(critic #4) the state-hash is a SELF-REPLAY DETERMINISM oracle, NOT a gamemd-parity oracle.** Bit-stability proves "same input→same output across runs," never "matches gamemd." Any slice introducing NEW gamemd-matching behavior (6, 7) **requires a gamemd-side evidence artifact** (Ghidra trace or in-game observation) per new behavior before baselining a new golden. No slice may change the hash without a `SNAPSHOT_VERSION` bump.

**Slice 1 — `ObjectSubstrate` wrapper (no behavior change).** Move `store/logic/occupancy/counter/ids` into `substrate.rs`; delegate existing methods. *Accept:* 5000-tick replay hash bit-identical; lifecycle tests unchanged; membership invariant holds.

**Slice 2 — `Presence` field + assert-only validation.** Add `presence` (serde-skip, rebuilt on load); set on transitions; `debug_assert!` legal source state; keep old gates (presence shadows). *Accept:* `presence == derived-from-old-gates` every tick for a full replay; hash identical (assert presence NOT hashed); save/load (limbo cargo + transport-loaded infantry) restores identical presence.

**Slice 3 — Collapse spawn 4-step into `unlimbo`/`create_limbo`.** ⚠ **(critic #11) classify up front — it cannot be both:** either a **pure no-op refactor** (hash-identical) OR it **adopts gamemd's Mark-before-register order** (hash-changing → `SNAPSHOT_VERSION` bump + new golden). Decide before coding. *Accept (no-op path):* hash identical; ordering regression test that a re-entrant observer cannot see an entity in `logic` before `occupancy`; map-load count + snapshot unchanged.

**Slice 4 — Incremental `by_owner` + owned-counts.** Substrate updates on `unlimbo`/`uninit`/`change_owner`; drop per-tick rebuild (keep for deserialize); route capture through `change_owner`. *Accept:* hash identical (owned-counts hashed via houses); mid-tick capture then `ids_for_owner(new)` returns the id without rebuild (intentional fix; re-verify no live path depended on stale behavior); deserialize-rebuild ≡ incremental.

**Slice 5 — Substrate owns enter-order counter.** `move_cell` owns `next_occupancy_enter_order`; replace `&mut u64` threading. *Accept:* hash identical (both counter + per-entity order hashed — world_hash.rs:49,387); `OccupancyGrid::rebuild` reproduces live order (occupancy.rs:763 test).

**Slice 6 — Deferred-delete queue + one-tick `Dying` window.** `uninit` enqueues + sets `Dying` (conceal/occupancy-unmark/detach stay **synchronous**); `advance_tick` flushes at cleanup; add `detach_all_links`. ⚠ **changes the hash** → `SNAPSHOT_VERSION` bump + NEW golden; **and requires a gamemd-side artifact** confirming the one-tick valid-but-dead semantics before baselining (critic #4). *Accept:* same-tick `last_attacker_id` resolves to valid-but-`Dying` (vs old `None`) — **with a gamemd trace/observation cited**; **(critic #6)** a Dying entity is absent from `logic` AND `occupancy` but resolvable by id for exactly one tick; **(critic #9)** mutual-reference death test (two units kill each other same tick) is deterministic and matches the documented detach order; post-flush `store.len()/logic.len()` correct; no live entity references a freed id.

**Slice 7 — Reveal gate-chain + rollback.** `reveal`/`unlimbo` enforce placement gates, return `Err(PlaceReject)`, roll back `Presence` on Mark fail. ⚠ behavior change → version bump + new golden + **gamemd-side artifact per gate** (critic #4). ✓ The `Type+0x234` eligibility gate IS confirmed (read in Conceal `0x005F4D30` and Reveal `0x005F4EC0`) but has **NO INI key** — it is a hardcoded class default (base ObjectTypeClass=0; Techno/Anim/Bullet/ParticleSystem ctors set =1; verified via constructor stores `0x7116AD`/`0x427750`/`0x46BD05`/`0x64420E` and absence from `ObjectTypeClass__ReadINI 0x005F92D0`). It is NOT `Foundation=` (that writes +0x298). **Implement the gate by class/category, not from INI.** *Accept:* reveal onto occupied/impassable cell rejects + leaves `Limbo` (no `logic`/`occupancy` entry); type-ineligible (non-cell-registering) object not added on reveal.

**Slice 8 — Type-resolution boundary.** Add `RuleSet::type_handle(&str)` (always case-insensitive) and `TypeHandleTable` built at `intern_all_ids`; migrate `GameEntity` type resolution to one hop; deprecate the case-sensitive raw gets; retire `Owner(InternedId)` wrapper (components.rs:80) + duplicate intern helpers (app_commands.rs). ✓ **(critic #8 RESOLVED) `FUN_007C8D20` is case-INSENSITIVE** — the case-insensitive plan matches gamemd; proceed. *Accept:* `state_hash` identical if type identity unchanged (any drift = a casing-sensitive lookup was silently missing a type — investigate, don't paper over); casing regression test (e.g. `htnk` vs `[HTNK]`) quantifying affected references (gamemd confirmed case-insensitive); `intern_all_ids` completeness (no orphan `type_ref`).

**Cross-cutting:** after each slice, `cargo test -p vera20k` green on lifecycle/occupancy/snapshot/world_hash; replay hash bit-identical (1–5, 8) or newly-deterministic-and-versioned-**and-gamemd-evidenced** (6, 7). No slice may change the hash AND skip a `SNAPSHOT_VERSION` bump.

---

## 9. Adversarial critic — verdict & findings (real), with follow-up status

**Verdict: REVISE — "Not ready to execute as written."** Half A's gamemd evidence is solid and the WRONG/UNVERIFIABLE labeling disciplined; problems concentrated in the contract verdicts and migration tests. No TS-legacy leak. **The 2026-05-30 follow-up has since resolved findings 1, 3, 7, 8 and confirmed 2; the design is now substantially unblocked (start at Slice 1).**

| # | Sev | Category | Issue (condensed) | Follow-up status |
|---|---|---|---|---|
| 1 | HIGH | Ungrounded parity | C8 FAITHFUL assumes every active-vector add passes sortedFlag=0 | ✓ mostly resolved: verified tail-append (no sort) → NOT DRIFT; residual: re-confirm `FUN_005519B0` arity |
| 2 | HIGH | DRIFT-default violation | C11 cell-bit/bridge-flag asymmetry called "acceptable" without proof | ◑ CONFIRMED real asymmetry (0x20 pair); leak only if bridge destroyed under elevated unit — **in-game test still required** |
| 3 | HIGH | Contract gap | C9 same-pass visibility UNVERIFIABLE yet designed on | ✓ RESOLVED: SAME-PASS (consumer `0x0055AFB0` re-reads count) → use re-read length for AI stage |
| 4 | HIGH | Test gap | Slices 6/7 use self-replay hash (determinism ≠ parity) for NEW behavior | folded: Slices 6/7 require a gamemd-side artifact before baselining |
| 5 | MED | Missing detail | C12 selection-above-sim lacks deselect-first + LastRefObject hook | folded into C12 |
| 6 | MED | Contract gap | C7/Slice 6 must state conceal/unmark/teardown SYNCHRONOUS, only slot-free defers | folded into C7 + Slice 6 |
| 7 | MED | Architecture drift | Moving `EntityCategory` into `sim/` inverts `map/`→`sim/` | ✓ CONFIRMED via dep check; keep in `map/`, derive `CapabilityFlags` in `sim/` |
| 8 | MED | Ungrounded parity | C13/Slice 8 mandate case-insensitive while casing UNVERIFIED | ✓ RESOLVED: `FUN_007C8D20` case-INSENSITIVE → plan correct; Slice 8 proceeds |
| 9 | MED | Missing detail | C10/§6 detach order vs observer-notify unspecified; no mutual-ref death test | folded into C10 + Slice 6 |
| 10 | LOW | Ungrounded parity | C1/§4 no-reuse `stable_id` rated FAITHFUL despite dangle divergence | folded → C1/§4 APPROX |
| 11 | LOW | Test gap | Slice 3 can't be both hash-identical AND adopt Mark-before-register | folded → Slice 3 classified |
| 12 | LOW | Other | enter-order IS hashed (retracts a disproven "not hashed" concern) | confirmed; §4 + Slice 5 kept |

## 10. Research status (2026-05-30 follow-up closed most gates)

**Resolved (binary-verified):**
1. ✓ **OnBridge = `+0x8C`** — the +0x23 was a decompiler `int*`-stride artifact (write `0x005F5986`, read `0x005F5F7B`, ctor `0x005F396F`). HIGH.
2. ✓ **Same-pass visibility (C9): SAME-PASS** — consumer `0x0055AFB0` (in `Main_Tick 0x0055D360`) re-reads live count; use re-read length for the AI stage, not a frozen snapshot. HIGH.
3. ✓ **`FUN_007C8D20` case-INSENSITIVE (C13/Slice 8)** — OR-0x20 case-fold; case-insensitive resolution plan is correct. HIGH.
4. ✓ **EntityCategory layering (critic #7)** — confirmed; keep in `map/`, derive sim-side `CapabilityFlags`. HIGH.
5. ✓ **`Type+0x234` eligibility (Slice 7)** — gate confirmed; **NO INI key** — hardcoded class default (base=0; Techno/Anim/Bullet/ParticleSystem=1). Implement by class/category. HIGH.
6. ◑ **Active-vector never sorts (C8)** — verified tail-append in observable effect (NOT DRIFT); residual: re-confirm `FUN_005519B0` arity + that no other caller passes a nonzero sort flag (MCP dropped mid-session). MEDIUM.

**Still open (need in-game test):**
7. ◑ **Bridge Mark/Clear asymmetry (C11)** — the 0x20 pair is genuinely asymmetric (`Mark_Occupation 0x007441B0` gates on height AND bridge-flag; `Clear_Occupation 0x00744210` on height alone). A bit strands only if a bridge is destroyed under an elevated unit; the destroy-path occupancy cleanup wasn't located — **needs an in-game test** (place unit on bridge, destroy bridge, compare underlying-cell passability vs gamemd).
8. **BridgeHeight magnitude** (`DAT_00AC13BC`) value — minor, not blocking.

**Bottom line:** Part A facts are implementation-grade; the follow-up cleared OnBridge, C9 same-pass, C13 casing, EntityCategory layering, and the Type+0x234 mechanism, and softened C8 to NOT-DRIFT. One item is genuinely open (the bridge-destruction occupancy leak — in-game test). **Slice 1 (pure refactor, hash-identical, zero research dependency) is the right start; Slices 7–8 are now unblocked** — Slice 8 fully; Slice 7 implement-by-category.
