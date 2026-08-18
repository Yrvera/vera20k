# Core Service Profile — frontier-ai-house (HouseClass AI brain)

**Service slug:** `frontier-ai-house`
**Status:** FRONTIER — promoted from catalog stub (`_frontier.md` §F3) to a **STRUCTURAL** profile.
**Scope note (project defers AI):** this profile maps the AI brain's **spine connectivity,
dependency edges, and RNG/lockstep relevance only**. It does NOT decode the AI decision logic
(target heuristics, build-order economics, script-step semantics, threat-map math). Those are
deferred per the project AI rule. Where a decision-function address is named below it is a
**structural anchor** (where the brain reads/writes / which list it walks), not a behavioral spec.

**Source of truth:** `docs/research/core-services-map/_spine-rung-27.md` (rung AA, VERIFIED
from binary), `docs/research/HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS_GHIDRA_REPORT.md`,
`docs/research/FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`,
`docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`. Addresses below are re-verified against
those Ghidra-cited docs this session (the live Ghidra instance was not reachable this session;
authority chain used = binary→Ghidra **as captured in the cited verified docs**→this map).

---

## Purpose (one paragraph)

The HouseClass AI brain is the **per-house skirmish/campaign opponent driver**: for each
non-current-player, non-passive house it runs a periodic build-choice and production-management
loop — pick the next building / unit / aircraft / infantry to produce, manage the AI base-plan
build queue, gate by credits/power/prerequisites, and suspend/resume superweapon and production
state. It is NOT a separate object or scheduler — **the AI brain lives entirely inside the
per-house tick `HouseClass::Update`**, interleaved with that same function's non-AI bookkeeping
(power/radar recheck, super-weapon ready poll, low-power EVA, defeat detection). The brain's
*outputs* are production/queue mutations that the FactoryClass step and the delivery
(Place_Production) path then realize; it does not directly move units (that is the separate
TeamClass tick) — it weights and queues what gets built. Decoding the actual choice formulas is
out of scope here.

---

## Representative-address re-verification (stub correction)

The stub (`_frontier.md` §F3) named three representative functions. Re-checked against the
verified docs this session:

| Stub claim | Verdict | Correction / evidence |
|---|---|---|
| `HouseClass__AI_EconomyStateMachine @ 0x00509700` (called "representative fn") | **UNVERIFIED — corrected** | No doc names a function `AI_EconomyStateMachine` at `0x00509700`. The verified AI-dispatch function in that neighborhood is `HouseClass::AI_DispatchProduction @ 0x005098F0` (`HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` header). The true **per-tick brain entry is `HouseClass::Update @ 0x004F8440`** (vt+0x5C / slot 23), which the spine walks; the AI choosers run *inside* it. Treat `0x00509700` as a navigation hint of unknown identity, NOT the representative fn. |
| `HouseClass__AI_ChooseNextProduction @ 0x00506EF0` | **VERIFIED (identity)** | `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` cites `AI_ChooseNextProduction @ 0x00506EF0` (base-plan / placement search; touched-not-exhausted). Real function, AI build-choice. |
| `HouseClass__AI_Manage_Build_Queue @ 0x004FDD10` | **VERIFIED (identity)** | Same doc cites `AI_Manage_Build_Queue @ 0x004FDD10` (AI base-plan/build-queue manager; can suspend/abandon all owned factories when plan cost exceeds budget — evidence `0x004FE05B..0x004FE097`). |

**Corrected representative function: `HouseClass::Update` @ `0x004F8440`** (a.k.a. HouseClass::AI),
vtable `vtable__HouseClass @ 0x007EA8A0`, slot **+0x5C** (slot index 23). This is the function
the per-tick spine dispatches as rung AA, and it is where the AI brain executes. Verified in
`_spine-rung-27.md`: `read_memory 0x007EA8FC` → vt+0x5C = `0x004F8440`; `get_xrefs_to 0x004F8440`
→ only `0x007EA8FC [DATA]` (pure virtual dispatch, no direct callers).

---

## Owns (state / globals / structs)

The AI brain does **not** own a separate manager object — it is a behavior layered on the
HouseClass struct (owned by the `factory-house` service). The fields it specifically reads/writes
as the AI brain (verified offsets, `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` §2):

- **AI chooser mode/state** `House+0x1E4` — `0`/`1`/`2` select different chooser order/recheck
  paths (evidence `0x004F9087..0x004F9265`, `0x004FE109`, `0x004FE3AB`).
- **AI/player gates** `House+0x1EC` (CurrentPlayer / IsHuman) + `House+0x1ED` (PlayerControl) —
  the recurring "is this the controllable/current player?" predicate that short-circuits the AI
  stages; forced true in `g_GameMode==0` campaign.
- **Defeated byte** `House+0x1F5` — gates `AI_ResumeProduction` (`0x0050B1D0`).
- **Production-dirty tail flag** `House+0x1FC` — consumed at the end of `HouseClass::Update` to
  fire `AI_ManageProduction`/`AI_ResumeProduction` (evidence `0x004F92ED..0x004F92FD`).
- **AI urgency / money / under-attack state** `House+0x250` (evidence `0x004FD6F9..0x004FD848`).
- **Strategy timer** `House+0x5634/+0x563C` — gates `AI_Building_Strategy`; the function's return
  value becomes the next duration (evidence `0x004F8FE1..0x004F9043`).
- **Category choice slots** (`-1` = empty): building `House+0x564C`, unit `House+0x5650`,
  infantry `House+0x5654`, aircraft `House+0x5658` (evidence `0x004FE3E0`, `0x004FEA70`, etc.).
- **AI base-plan / build queue** `House+0x5708` (16 bytes/entry), count `House+0x5714` (evidence
  `0x004FDD10`, `0x004FE3E0`, `0x00506EF0`).
- **AI "currently producing" tracking** (from `FACTORY_CLASS_BUILD_SPEED` §HouseClass AI fields):
  Vehicle `House+0x564C`, Building `House+0x5650`, Aircraft `House+0x5654` (RTTI-tagged slots
  the AI consults to avoid double-queueing).

**Globals registered/owned (shared with `factory-house`, used by the brain's tick):**
- `g_HouseClass_Array @ 0x00A8022C`, count `@ 0x00A80238` — the array the spine walks to reach
  each house brain. (Registration site `HouseClass__Constructor` write at `0x004F61E0/0x004F61E6`;
  `get_xrefs_to 0x00a8022c`/`0x00a80238` per `_spine-rung-27.md`.)
- `g_ScenarioClass_Instance @ 0x00A8B230` → `+0x218` = **Scen->Random** (the brain's one
  lockstep-stream draw site, see RNG section).
- `g_MainRng @ 0x00886B88` — the brain's non-lockstep UI-stream draws.

---

## Key functions & globals (addresses)

All identities below are taken from the cited verified docs (each doc records its own
`decompile_function` / `disassemble_function` evidence inline).

**Brain entry (the representative fn):**
- `HouseClass::Update` (a.k.a. HouseClass::AI) **0x004F8440** — vt+0x5C / slot 23 on
  `vtable__HouseClass @ 0x007EA8A0`. The per-house tick; the AI brain runs inside it.

**AI build-choice / strategy (structural anchors — logic NOT decoded here):**
- `HouseClass::AI_Building_Strategy` **0x004FD500** (strategy-timer-gated; picks nearest
  enemy house, threat scoring).
- `HouseClass::AI_Check_Build_Need` **0x004FD9A0**.
- `HouseClass::AI_Manage_Build_Queue` **0x004FDD10** (base-plan/build-queue; may suspend/abandon
  owned factories on over-budget plan — VERIFIED stub address).
- `HouseClass::AI_Choose_Building` **0x004FE3E0**; `AI_Choose_Unit` **0x004FEA60**;
  `AI_Choose_Infantry-like` **0x004FEEE0**; `AI_Choose_Aircraft-like` **0x004FF210**.
- `HouseClass::AI_ChooseNextProduction` **0x00506EF0** (base-placement search — VERIFIED stub
  address; touched-not-exhausted).
- `HouseClass::AI_DispatchProduction` **0x005098F0** (the function nearest the stub's mislabeled
  `0x00509700`).
- `HouseClass::AI_ManageProduction` **0x0050AF10**; `AI_ResumeProduction` **0x0050B1D0**
  (dirty-gated superweapon grant/suspend/deactivate/resume/cameo — NOT the build-queue choosers,
  per `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` §1).

**Non-AI co-resident stages inside the same tick (owned by `factory-house`, listed for context):**
- `AI_AssessPower` 0x00508C30, `CheckSuperweaponReady` 0x00508DF0, `CheckLowPower` 0x00508F60,
  `ScatterAllUnits` 0x004FC6D0, `MPlayer_Defeated` 0x004FC0B0, `SuperClass::AI_Ready` 0x006CBCA0
  (per-house super vector +0x258).

**Spine plumbing:**
- `LogicClass::PerTickUpdate` **0x0055AFB0** — drives the house loop at `0x0055B68D–0x0055B6B3`.
- `Main_Tick` **0x0055D360** — sole caller; bumps `g_CurrentFrameCounter @ 0x00A8ED84` late.

---

## Tick / render / load plug point

**Plug point: the per-tick spine, RUNG AA** (the 27th of 28 rungs in
`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`), driver `HouseClass::Update @ 0x004F8440` via vt+0x5C.

> The stub called this "rung U." That is informal; the **verified spine spec labels it rung AA**
> (rung 27). Use rung AA.

Body loop (verified disassembly, `_spine-rung-27.md` §2):
```
0055b68d: MOV EAX,[0x00a80238]            ; count = g_HouseClass_Array_Count
0055b696: JLE 0x0055b6b3                  ; GATE: count > 0
0055b698: MOV EAX,[0x00a8022c]            ; base = g_HouseClass_Array
0055b69d: MOV ECX,[EAX+ESI*0x4]           ; entry = array[i]  (HouseClass*)
0055b6a2: JZ  0x0055b6a9                  ; PER-ENTRY null-check: skip null slot
0055b6a6: CALL [EDX+0x5c]                 ; HouseClass::AI  (= 0x004F8440)
0055b6b1: JL  0x0055b698                  ; FORWARD walk, ascending index
```
- **Order:** rung AA runs **after rung Z (FactoryClass tick, `0x0055B66A..b68B`)** and **before
  rung AB (last-ref camera follow, `0x0055B6B3+`)**. So **all factories step first, then all
  houses tick** — the AI brain reads post-step factory/production state.
- **Walk:** `g_HouseClass_Array` FORWARD (ascending index), count re-read each iteration, with a
  **per-entry non-null guard** (rungs T/U don't null-check; this one does).
- **Internal AI gate:** the AI brain stages inside `Update` are gated by the
  non-current-player / non-`MultiplayPassive` (`Type+0x1A6 == 0`) predicate, and the choosers run
  on an 8-frame cadence (`g_CurrentFrameCounter & 0x80000007 == 0`). The *rung itself* ticks every
  registered house unconditionally; the AI work is an internal short-circuit.

**Not render, not load, not audio.** The brain is pure sim-side per-tick logic. (Its EVA cues —
insufficient-funds / silos-needed — are emitted by the same `Update` function but routed to the
out-of-sim audio layer; see `frontier-audio-eva`.)

---

## RNG / lockstep relevance

**Rung AA draws up to 3 RNG values, but ALL are local-player + network-gated — the AI brain on
AI/remote houses consumes ZERO synchronized lockstep draws.** (Verified, `_spine-rung-27.md` §4.)

The entire draw block (`0x004f883e–0x004f890d`) is gated by: this house's ArrayIndex == the
local/current player house (`0x00A83D4C`) **AND** `g_GameMode == 3 || == 4` (network/LAN/internet)
**AND** not spectating. Draw sites:
1. `0x004f8888` ECX=`0x886b88` → **g_MainRng** `(0,1)` one-time init.
2. `0x004f889A` ECX=`0x886b88` → **g_MainRng** `(0,2)` per-tick (harass-cell decision).
3. `0x004f8908` ECX=`Scenario+0x218` → **Scen->Random** `(0,2)`, only when the picked cell holds a
   live occupant; **result discarded**.

Lockstep consequence (from `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` §3): rung AA's single
Scen->Random draw is **local-player-gated → 0 synchronized draws on non-local/AI houses**; the
g_MainRng draws are the non-synchronized UI stream. **The AI brain proper (the choosers,
build-queue, strategy) is NOT in this draw block and was not found to draw the synchronized
stream during the per-tick AI cadence** in the cited evidence — it mutates production/queue state
deterministically from already-synchronized inputs. (Caveat: the full chooser subtree's draw set
is part of the deferred AI decode; `AI_Building_Strategy`'s nearest-enemy / threat-score path was
not swept for RNG here. Marked UNCHECKED below.) `HouseClass__Constructor` draws Scen->Random
`(0x1C2,0x708)` for build-delay jitter, but that is **construction, not this per-tick rung**.

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| **logicclass** | The brain runs only because `LogicClass::PerTickUpdate 0x0055AFB0` dispatches `HouseClass::Update` (vt+0x5C) as rung AA. The spine owns the order (factories before houses) and the late frame-counter bump. | `_spine-rung-27.md` §2/§6; `disassemble 0x0055AFB0` loop `0x0055B68D–0x0055B6B3`; spine spec rung AA. |
| **factory-house** | The brain IS layered on HouseClass and reads/writes its own house's fields (wallet `+0x30C`, power `+0x53A4/+0x53A8`, dirty flag `+0x1FC`, choice slots `+0x564C..+0x5658`, base-plan queue `+0x5708/+0x5714`); its outputs feed FactoryClass production via the chooser → queue path. The brain reads **post-step** factory state (factories tick first). | `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` §2/§3; `FACTORY_HOUSE_AI_ORDER…` (Tactical→factories→houses, `0x0055B675..b6b1`). |
| **target-scoring** | `AI_Building_Strategy 0x004FD500` selects a nearest non-self, non-passive, non-defeated enemy house by 3D distance and calls `Update_Threat_Score(1, target)`; chooser candidate validation uses per-type buildable checks (type vtable `+0x94`). Threat/target selection is the target-scoring layer. | `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` §3.2 (`0x004FD538..0x004FD660`); chooser vt+0x94 (`0x004F90B3..0x004F90E2`). |
| **rules-class** | Choosers read parsed INI tunables — `Rules+0x13F4[difficulty]` probability table (nearest/highest-priority vs random max-need candidate; `0x004FEDF2/0x004FF190/0x004FF4C0`); build-queue cost gates read Rules build-economy fields. The choice logic is parameterized entirely by RulesClass data. | `HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS` §2 (`Rules+0x13F4`). |
| **frontier-ai-team** | The house brain is the producer side of skirmish team AI: AI build/production choices and AITrigger evaluation queue what teams later recruit; teams themselves are walked by a **separate rung (rung L, `0x0055B502`, TeamClass::AI `0x006E9140`)**, not dispatched from inside the house tick. So the edge is "brain weights/queues team production," not "brain calls TeamClass::AI." | Spine spec rung L vs rung AA (distinct drivers); `AI_BRIDGE_INTERACTION` (AI subsystem = script opcodes + HouseClass AI + TeamClass + AITriggerTypeClass). **AI — deferred.** |
| **frontier-ai-trigger** | Skirmish AITriggerTypeClass (weighted, condition-gated team production) is evaluated within the HouseClass AI brain's per-tick economy/production logic; it is the mechanism that drives what the AI queues. (Structural edge only — evaluation point is inside the rung-AA AI stages; exact call site is part of the deferred AI decode.) | `_frontier.md` §F4 (evaluated within the house brain, rung U/AA); `AI_BRIDGE_INTERACTION` AI-subsystem inventory. **AI — deferred.** |
| **random-scenario** | The brain's local-player network-only RNG block draws g_MainRng `(0,1)`/`(0,2)` and one local-gated Scen->Random `(0,2)`; the lockstep contract requires rung-AA's position in the synchronized draw order be held even though it consumes 0 synchronized draws on non-local houses. | `_spine-rung-27.md` §4; spine spec §3 (rung AA local-gated). |

## Used-by (incoming edges)

| Source slug | Via symbol / field | Evidence |
|---|---|---|
| **logicclass** | Reciprocal of the depends-on: PerTickUpdate is the caller; it walks `g_HouseClass_Array` and invokes `HouseClass::Update` (vt+0x5C) as rung AA, supplying the order and the live frame clock. | `_spine-rung-27.md` §2; spine spec rung AA. |
| **factory-house** | The shared HouseClass/FactoryClass substrate hosts the brain: the brain's chooser outputs become FactoryClass queue/production mutations realized by `FactoryClass::AI 0x004C9B20` and delivery `Place_Production 0x004FB0E0`. The factory-house profile already names this incoming edge as `frontier-ai` (deferred). | `factory-house.md` Used-by row `frontier-ai (deferred)` (`Update 0x004F8440` AI gate). |
| **frontier-ai-team** | Teams recruit members and run scripts to act on what the brain produced; the team rung consumes the production the house brain queued. (Reciprocal of the outgoing edge.) | Spine spec rung L (`0x006E9140`); **AI — deferred.** |
| **frontier-super** | Superweapon manage/resume (`AI_ManageProduction 0x0050AF10` / `AI_ResumeProduction 0x0050B1D0`) and the per-house SuperClass AI-ready poll (`SuperClass::AI_Ready 0x006CBCA0` over the +0x258 super vector) run inside this same rung-AA tick; the AI brain's dirty-flag tail (`+0x1FC`) drives the manage/resume calls. | `_spine-rung-27.md` §2 (steps 9, 11); `FACTORY_HOUSE_AI_ORDER…` (`0x004f92f4` manage/resume). |

---

## Active-in-YR / Tiberian Sun legacy

**ACTIVE in YR — the AI opponent every skirmish/campaign match has.** NOT TS legacy.
- The rung-AA host (`HouseClass::Update`) ticks every registered house every match; the AI brain
  stages fire for every non-current-player, non-`MultiplayPassive` house (i.e. every AI opponent).
- Player-visible outputs: AI opponents choosing and building structures/units, defending, and
  managing superweapons — exactly the parity-bar surface. (The brain *decision quality* is what
  the project defers; its *existence and spine position* are live and in-scope for the map.)
- The narrow local-player network-only RNG "harass an occupied cell" sub-block (§RNG) is a minor
  cosmetic flourish reachable for the local human in a mode-3/4 game; it does not fire in campaign
  (mode 0) or for AI/remote houses and changes no synchronized state.

---

## Open / unverified

- **Stub `0x00509700` identity** — named "AI_EconomyStateMachine" by the stub; no verified doc
  confirms a function there. Nearest verified neighbor is `AI_DispatchProduction @ 0x005098F0`.
  Identity of `0x00509700` is **UNKNOWN** this session (live Ghidra not reachable to resolve).
- **AI chooser-subtree RNG draws** — whether any of `AI_Building_Strategy` /
  `AI_Choose_*` / `AI_Manage_Build_Queue` draws the synchronized Scen->Random stream during the
  8-frame AI cadence was NOT swept (it is part of the deferred AI decode). If any does, its
  position is part of the lockstep order and must be modeled. **UNCHECKED.**
- **AITrigger evaluation call site** — the exact instruction range inside the rung-AA AI stages
  where AITriggerTypeClass weighting is evaluated was not isolated here (deferred AI decode).
- **Live re-verification pending** — all addresses above are sourced from prior Ghidra-cited
  verified docs, re-read this session, but the live Ghidra instance was unreachable; a future pass
  should re-confirm `0x004F8440` vt-slot and the chooser addresses directly via
  `read_memory 0x007EA8FC` + `decompile_function` when an instance is up.
```
