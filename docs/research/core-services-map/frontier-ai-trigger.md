# frontier-ai-trigger — AITriggerTypeClass (skirmish AI triggers)

**Service slug:** `frontier-ai-trigger`
**Status:** STRUCTURAL PROFILE (promoted from `_frontier.md` §F4). Decode of the AI
*decision logic* (condition heuristics, team-production economics, weight tuning math) is
**deliberately out of scope** — project rule `feedback_no_ai_yet`. This profile establishes
**spine connectivity only**: owner/global, tick rung, dependency edges, RNG/lockstep
relevance.

**Authority order:** binary → Ghidra → docs. **Session caveat (honest):** no live Ghidra
instance was reachable this session (`list_instances` → 0 instances; TCP 127.0.0.1:8089
refused on every `connect_instance` attempt). Addresses below are therefore
**doc-corroborated from prior live-Ghidra sessions, NOT live-re-verified this session** —
each cites the doc that recorded the original live call. Treat every address as
**LOCATED (prior-session-verified), re-verify-pending** until a live Ghidra pass confirms.

---

## 1. Purpose (one paragraph — structural only)

`AITriggerTypeClass` is the skirmish/AI **trigger-type table**: a list of weighted,
condition-gated rules of the form "when condition C holds for this house, produce team T
with weight W." It is the AI's equivalent of map triggers (F1 `frontier-trigger`) but is
*house-AI-owned* rather than map-scripting-owned — it decides **what the AI builds and
sends** in a skirmish. Each type carries a condition selector, side/tech-level gating,
per-difficulty enable flags, an adaptive weight that the engine nudges up/down based on
hit ratio, and a target/owner team reference. The list is walked by the per-house AI brain;
weighted selection picks which trigger fires. The *decision semantics* (what each condition
means, how teams are chosen, the weight-tuning formula) are **NOT decoded here** by project
rule — only its place in the tick spine and its dependency edges.

---

## 2. What it owns (globals / structs)

| Symbol | Address | Role | Evidence (prior-session) |
|---|---|---|---|
| `vtable__AITriggerTypeClass` | (set by ctor) | class identity; RTTI value **0x3b** | `LABEL_AUDIT_LOG.md` "3 more RTTI values this round" + ctor finding |
| `g_AITriggerTypeClass_Array` | `DAT_00a8b204` | the live list of all AI trigger types (scenario-bound) | `LABEL_AUDIT_LOG.md` "AITriggerTypeClass — verified-but-rename-DEFERRED" |
| `g_AITriggerTypeClass_Array_Count` | `DAT_00a8b210` | element count of the list | same |

**Key structural fact (load-bearing for the map):** the constructor pushes `this` into
`DAT_00a8b204` but **does NOT** push into the master Abstract registry `DAT_00b0f674`. AI
trigger types are **EXCLUDED from the universal Abstract tracking array** — they are an
*isolated subsystem* owned by the scenario/house layer, not registered like ordinary
ObjectClass-derived entities. This means they do **not** participate in the master-heap
save/load walk and are **not** ticked by the main LogicClass object vector (rung T). Their
only driver is the house AI brain (see §4). (Evidence: `LABEL_AUDIT_LOG.md`,
"Isolated Subsystem Pattern" + "verified-but-rename-DEFERRED" entries.)

### Relevant field offsets (from prior `AI_DIFFICULTY_SYSTEM.md` decode — structural, not behavioral)

| Offset | Type | Purpose |
|---|---|---|
| +0x98 | int | Condition type (-1..7) |
| +0xA0 | int | Side restriction (0=any, 1=specific) |
| +0xB0 | int | Tech-level requirement |
| +0xB8 | double | Current adaptive weight |
| +0xC0 / +0xC8 | double | Min / max weight clamp |
| +0xD0 / +0xD2 / +0xD3 / +0xD4 | byte | Per-difficulty enable flags (Easy/Normal/Hard/+) |
| +0xDC / +0xE0 | ptr | Primary / secondary weapon-type ref |
| +0x104 / +0x108 | int | Success count / total-attempt count (drive adaptive weight) |

These offsets are carried forward for completeness; their *use* is AI behavior and is out
of scope.

---

## 3. Key functions (re-verify-pending; doc-corroborated)

| Function | Address | Role | Evidence |
|---|---|---|---|
| `AITriggerTypeClass__Constructor` | `0x0041e350` | **representative fn** — constructs a type, sets `vtable__AITriggerTypeClass`, pushes into `g_AITriggerTypeClass_Array` (`DAT_00a8b204`), count `DAT_00a8b210`; does NOT push master registry | `LABEL_AUDIT_LOG.md` "AITriggerTypeClass — verified-but-rename-DEFERRED" (recorded from a live Ghidra session) |
| `AITriggerType__IncreaseWeight` | `0x0041FD60` | adaptive difficulty: raise weight from hit-ratio (uses `Rules+0xC0` base rate); 191 bytes | `AI_DIFFICULTY_SYSTEM.md` §9 |
| `AITriggerType__DecreaseWeight` | `0x0041FE20` | adaptive difficulty: lower weight (uses `Rules+0xC8` base, `Rules+0xD0` decay); 187 bytes | `AI_DIFFICULTY_SYSTEM.md` §9 |

**Per-tick evaluation entry:** the AI-trigger list is evaluated **inside the house AI brain
subtree** rooted at `HouseClass::Update`. The brain's top-level strategy tick is
`AI_Building_Strategy @ 0x4fd500`, with production routed through
`AI_ChooseNextProduction @ 0x506ef0` / `AI_DispatchProduction @ 0x5098f0`
(`HOUSECLASS_GHIDRA_REPORT.md` "Key Named Functions"). The exact callee that walks
`DAT_00a8b204` and runs the weighted pick is **UNVERIFIED** — it was not isolated this
session and the AI-brain decode is deferred (`feedback_no_ai_yet`). Located scope is "the
`0x4fd500`/`0x506ef0` brain subtree within rung AA," not a single confirmed eval address.

---

## 4. Tick plug point (spine rung)

**Rung AA (#27)** — `HouseClass::AI/Update @ 0x004F8440`, vt+0x5c, walking
`g_HouseClass_Array 0x00A8022C` count `0x00A80238` FORWARD with a per-entry non-null guard.
(Evidence: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` rung table row 27.)

> **Stub correction.** `_frontier.md` §F4/§F3 loosely call the HouseClass AI brain "rung U."
> Per the verified spine spec, the HouseClass tick is **rung AA (#27 @ 0x004F8440)**; the
> letter **U (#21 @ 0x00423ac0)** is the *AnimClass MoveFlash* vector, an unrelated service.
> AITrigger evaluation rides inside rung AA, **not** rung U. This profile uses the
> spine-spec lettering (AA) as authoritative.

AI trigger types are **not** a separate rung and are **not** in the main object vector
(rung T) — they tick only as a nested step of the house brain on rung AA. They are
scenario-bound and evaluated per-house each tick the brain runs (subject to the brain's own
cadence/gates, which are part of the deferred AI decode).

**Lockstep / RNG relevance.** Rung AA's RNG draws are documented as the (1) one-time
`g_MainRng(0,1)`, (2) per-tick `g_MainRng(0,2)`, and (3) a `Scen->Random(0,2)` that is
**local-player-gated** (network mode, non-spectating) → **0 synchronized draws on AI/remote
houses**. AI-trigger weighted selection is performed *on AI houses*; if it consumes the
synchronized `Scen->Random` stream, its draw **count and order would be part of the lockstep
contract** and any reordering of the brain's trigger walk would desync. Whether the weighted
pick draws from `Scen->Random` vs `g_MainRng` vs a deterministic weight sum is **UNVERIFIED**
(weighting was catalogued as `DiscreteDistributionClass` in `_frontier.md` but the stream
binding was not confirmed). Flag this as a **lockstep-sensitive open question** for the AI
decode. (Evidence: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` rung AA + §3 stream rules.)

---

## 5. Dependency edges

### Outgoing (this service depends on →)

| → service | via | evidence |
|---|---|---|
| `factory-house` | evaluated inside `HouseClass::Update`/`AI_Building_Strategy` (rung AA); reads house difficulty, side, tech level, threat state; production it requests routes through HouseClass `AI_DispatchProduction`/Begin_Production | `HOUSECLASS_GHIDRA_REPORT.md` Key Named Functions; spine rung AA |
| `rules-class` | adaptive weight tuning reads `Rules+0xC0/+0xC8/+0xD0` rate/decay constants; per-difficulty enables and ratios (e.g. `RatioAITriggerTeam` House+0x565C from map INI) | `AI_DIFFICULTY_SYSTEM.md` §9; `HOUSECLASS_GHIDRA_REPORT.md` Read_Scenario_INI |
| `random-scenario` | **CONDITIONAL/UNVERIFIED** — if weighted selection draws the synchronized `Scen->Random` stream on AI houses (lockstep-relevant); not confirmed this session | `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` rung AA RNG note (open question) |
| `frontier-ai-team` | a fired trigger's payload is a **team** (TeamTypeClass → TeamClass) produced/dispatched by the house brain; teams then tick on rung L (#12) | `_frontier.md` §F2/§F4; `GAMEMD_ARCHITECTURE.md` AI System (AITriggerTypeClass → team production) |
| `frontier-ai-house` | the house AI brain (rung AA subtree) is what walks `g_AITriggerTypeClass_Array` and runs the pick — AITrigger is a *step of* the brain | `_frontier.md` §F3/§F4; `HOUSECLASS_GHIDRA_REPORT.md` AI Subsystems |

### Incoming (← depends on this service)

| ← service | via | evidence |
|---|---|---|
| `frontier-ai-house` | the per-house AI brain consumes the AITrigger list to decide skirmish production; it is the sole driver/consumer (rung AA) | `HOUSECLASS_GHIDRA_REPORT.md`; `_frontier.md` §F3 |
| `frontier-saveload` | AITrigger *types* are scenario-bound and **excluded from the master Abstract registry**, so they are NOT serialized via the master-heap walk; their persisted state (adaptive weight, success/attempt counts) saves through the scenario/AI path, separate from the object save graph | `LABEL_AUDIT_LOG.md` "EXCLUDED from the universal Abstract tracking array" |

### Most-depends-on (single strongest edge)

`frontier-ai-house` — AITrigger has no independent driver; it exists only as a sub-step of
the HouseClass AI brain on rung AA. If/when AI is decoded, this service is a leaf of that
brain.

---

## 6. Active-in-YR / TS-legacy

- **Active in YR:** YES, in principle — `AITriggers.ini` / `[AITriggerTypes]` drive stock
  skirmish AI build-and-attack behavior; the brain runs on rung AA every match with AI
  players. The *types* are loaded and the array is populated in any skirmish with AI.
- **TS-legacy:** the AITriggerType **mechanism** is shared TS/RA2/YR lineage but is **live in
  YR** (unlike fog-of-war or subterranean). No part of this service is gated off in stock YR.
- **Project status:** DEFERRED — `feedback_no_ai_yet`. This profile is structural-only; the
  AI decision logic is intentionally not decoded.

---

## 7. Open questions (for the eventual AI decode — do not resolve here)

1. **Exact per-tick eval address.** The callee inside the `0x4fd500`/`0x506ef0` brain
   subtree that walks `DAT_00a8b204` and runs the weighted pick — UNVERIFIED.
2. **RNG stream binding of the weighted pick** (`Scen->Random` vs `g_MainRng` vs
   deterministic). **Lockstep-critical if synchronized.** UNVERIFIED.
3. **`DiscreteDistributionClass` linkage** — `_frontier.md` names it as the weighting
   structure; the actual selection function and its stream were not confirmed.
4. **Live re-verification** of `0x0041e350`, `0x0041FD60`, `0x0041FE20`, and the
   `DAT_00a8b204`/`DAT_00a8b210` array binding against the binary in a session with Ghidra
   reachable (this session could not connect).

---

## 8. Cross-references

- Spine: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` (rung AA #27 @ `0x004F8440`)
- House brain: `HOUSECLASS_GHIDRA_REPORT.md` (AI Subsystems, Key Named Functions)
- Field/weight decode: `AI_DIFFICULTY_SYSTEM.md` §9
- Array binding + isolation: `LABEL_AUDIT_LOG.md` (2026-05-17 round 19; Isolated Subsystem)
- Architecture context: `GAMEMD_ARCHITECTURE.md` §5.D (AI System)
- Sibling frontier stubs: `_frontier.md` §F1 (`frontier-trigger`), §F2 (`frontier-ai-team`),
  §F3 (`frontier-ai-house`)
