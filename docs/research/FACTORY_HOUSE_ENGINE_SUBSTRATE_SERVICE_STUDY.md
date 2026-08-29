# FactoryClass / HouseClass — Engine Substrate Service Study & Replacement-Boundary Design

> **2026-08-29 active-binary correction.** This study's H9/defeat rows use the
> stale `ScatterAllUnits` interpretation for `0x004FC6D0`. That function is the
> shared live-Techno destruction sweep, not movement Scatter: it clears incoming
> Temporal state and enters concrete `ReceiveDamage` with current health and
> configured C4. Those rows are superseded by
> `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`;
> unrelated Factory/House substrate findings remain historical evidence.

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**v2 verification pass:** 2026-06-04 — live re-decompile of P0/UNRESOLVED/DOC-ONLY items; ledger §9 rebuilt.
**Rule:** Rust-native structure, gamemd-native semantics.
**Bar:** active in a standard **local skirmish** (`g_GameMode == 0` campaign-local or `== 5` skirmish/LAN). MP-only / SpecialFlags / TS-legacy / AI-house behavior is flagged DORMANT or DEFERRED-AI.
**Provenance:** assembled by a workflow (parallel Ghidra decode digests A–G + Rust-map lane G + adversarial completeness critic). Three load-bearing offset/name conflicts between input digests were re-decompiled live **this session** and adjudicated against the binary (power offsets, factory-pointer category naming, FactoryClass +0x4C/+0x4D); their verdicts are folded into §0/§2. Every address/offset/file:line is cited inline. **Default verdict for any unproven equivalence is DRIFT** — there is no internal-only escape hatch for active gameplay/economy/ordering. DOC-ONLY rows are corroborated by a prior digest but were **not** re-read live this session; the §9 ledger separates verified-this-session from DOC-ONLY.

**Companion:** the in-flight engine-substrate program (Shell/object substrate + Mission/Radio substrate). Master TODO: the core-engine-substrate plan and the mission/radio substrate plan. This substrate **slots into** that program — it does not invent a parallel architecture.

---

## Executive Summary

**Verdict: the current Rust production/economy code is structurally far from the gamemd-native substrate — it is a set of owner-keyed `BTreeMap`s mutated by module-level free functions with no per-(house,category) factory authority, no factory registry, no per-step credit charge, and no central prerequisite revalidation.** The single largest player-visible gap is the economy model: gamemd charges a build's full cost *incrementally over 54 steps* and stalls (OnHold + rewind) when credits run out mid-build, while Rust pre-pays the full cost at enqueue and therefore never halts — and consequently refunds the full cost on cancel instead of the spent portion. Several smaller but real DRIFTs compound this: cancel-one removes the wrong queue end, factory-complete is not separated from delivery, the ore-deposit purifier bonus credit is added raw instead of run through TibValue×IncomeMult (the purifier *count* base itself is CORRECT — the v1 'silo storage capacity' claim was REFUTED in v2), runtime diplomacy is static, and the global factory-step-before-house-tick ordering is not reproduced. The proposed replacement is an additive, shadow-first **production+economy substrate** (`Economy` wallet value-type + per-(house,category) `Factory` + a deterministic `FactoryRegistry`) that mirrors the verified behavior contract while using `InternedId`-keyed structures for the 30-player scale target. Rollout follows the proven Mission/Radio rhythm — shadow → invert hash-invariant → drop shadow asserts → make authoritative → bump `SNAPSHOT_VERSION 17→18` → parity-harness — gated behind a research checkpoint (P0). GetBuildStepTime truncation order is now VERIFIED-LIVE v2 (no ×0.9; per-iteration MultipleFactory truncation); only the AIVirtualPurifiers index-field identity remains open.

---

## Table of Contents

- §0. Conflict resolutions (binary-adjudicated this session)
- §1. Active-YR responsibilities of the combined Factory + House substrate
  - §1.1 FactoryClass — per-house, per-category production state machine
  - §1.2 HouseClass — per-player economy, power, prerequisites, lifecycle
  - §1.3 Global plumbing the substrate depends on
- §2. Full inventory (methods, fields, globals, registries, tables, vtable slots, legacy)
- §3. Active vs inactive/legacy/deferred split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements C1–C20)
- §6. Rust-native replacement boundary
- §7. Old ad hoc Rust logic to retire
- §8. Migration slices + acceptance tests (P0–P9)
- §9. Sources & Verification Ledger

---

## 0. Conflict resolutions (binary-adjudicated this session)

| Contested claim | Inputs disagreed | Verdict (this session) | Evidence |
|---|---|---|---|
| Power sums offset | Digest B said +0x5384/+0x5388 (struct-layout read); Digest E/V4 said +0x53A4/+0x53A8 | **+0x53A4 PowerOutput / +0x53A8 PowerDrain.** Digest B's struct-layout read of +0x5384/+0x5388 as power is **OVERRULED and WRONG**; +0x5384/+0x5388 are the per-RTTI factory-count fields. Digest E's struct layout independently confirms PowerOutput@21412=0x53A4. | `disassemble_function 0x004FCE30` (GetPowerRatio) → `MOV EAX,[ECX+0x53a4]`, `MOV ECX,[ECX+0x53a8]`. A reader of Digest B's Power section must not trust it. |
| `Primary_For*` factory-pointer category↔offset binding | Old doc table: +0x53AC=Infantry, +0x53B0=Aircraft. Digest B (verified ×3 sites this session via struct layout + 2 assignment sites): **the EXACT OPPOSITE** — +0x53AC=Aircraft, +0x53B0=Infantry | **RESOLVED v2 (binary-verified, no longer provisional).** +0x53AC = Primary_ForAircraft (RTTI 2,3 → g_AircraftTypeClass_Array), +0x53B0 = Primary_ForInfantry (RTTI 0xf,0x10 → g_InfantryTypeClass_Array). Digest B CONFIRMED; old-doc table (Infantry@+0x53AC, Aircraft@+0x53B0) **REFUTED** (v1 said Infantry@+0x53AC; v2 REFUTED → Aircraft@+0x53AC, evidence R2/E2). Verified across 4 byte-identical dispatch tables in Begin_Production + FUN_004FAA10 + Place_Production, keyed by RTTI_To_TypeArray. Independently corroborated by live struct layout (21420 Primary_ForAircraft, 21424 Primary_ForInfantry). | `disassemble_function 0x004FA350` (jump tables @0x4fa854/0x4fa890/0x4fa8cc + idx tables, RTTI-1→slot); `decompile_function 0x0048DCD0`; `decompile_function 0x004FAA10`; `get_struct_layout HouseClass` (E2). |
| FactoryClass +0x4C/+0x4D label | Digest A: +0x4C IsAlloc / +0x4D IsInit | **+0x4C IsInit / +0x4D IsAlloc** (live `get_struct_layout FactoryClass`). Cosmetic; both are DynamicVectorClass bookkeeping. | `get_struct_layout FactoryClass`. |
| Ally bitmask | Digests B doc-table & some docs: +0x1D8 "Allies" | **+0x5788 = authoritative Allies; +0x1D8 = constructor-self / map-editor-only mask** | Digest F via `get_struct_layout HouseClass` (offset 22408 `Allies`) + MakeAlly/BreakAlliance bodies. **VERIFIED-LIVE v2.** Re-verified live: `get_struct_layout HouseClass` (22408 Allies); MakeAlly 0x004F9B70 `Allies |= 1<<(other+0x30 & 0x1f)`; BreakAlliance 0x004F9F90 mutual-clear; +0x1D8 = `1<<(ArrayIndex & 0x1f)` self-mask set in ctor (`param_1[0x76]`, 0x76×4=0x1D8), touched by MakeAlly/BreakAlliance only under g_MapEditorMode. |
| "661/54" build-time framing | Task brief implied a 661 constant divisor | **Only divisor is 0x36 (54)**; 661 is one *example* total (MTNK) = `rate(12) × 54` | Digest A/C verified SetRate magic `0x4bda12f7 + SAR 4 = ÷54`. |
| +0x1DC StartingCredits | Digest F: Read_Scenario_INI writes `+0x1dc = ReadInt(Credits)*100` (campaign path); Digest E: no write/read at +0x1DC found this session | **RESOLVED v2 (live re-decode 2026-06-04). +0x1DC IS written in BOTH init paths. Digest F CONFIRMED, Digest E REFUTED.** Campaign Read_Scenario_INI writes +0x1dc = `ReadInt(Credits)*100` (via two `LEA EAX,[EAX+EAX*4]` + `SHL 2`); skirmish/MP Create_Houses → Set_Credits_And_Color writes the raw lobby slot-credits value (slot+0x53) to BOTH +0x1dc and +0x30c (no ×100). +0x1DC seeds Balance(+0x30c) at init (campaign: with [Difficulty] bonus Rules+0xdfc/+0xe00 selected by DAT_00a8eb64, then negative-clamp; MP: identical value). After init +0x1DC is INERT for gameplay/economy — read only to derive Balance at init and serialized in save/CRC. **Scale convention DIFFERS: campaign ×100 vs MP raw** — a DRIFT to confirm against the Rust credit-scale convention. (v1 said Digest E found no write; v2 REFUTED → both paths write +0x1DC, evidence R3/E1.) | `decompile_function 0x00500B40` + `disassemble_function 0x00500B40` (write @0x00500bd0, ×100 @0x00500bb9-d0); `decompile/disassemble_function 0x004FCE00` (writes +0x1dc=+0x30c, asm 0x004fce07/0x004fce0d); `decompile_function 0x00687f10` (R3, E1). |

---

## 1. Active-YR responsibilities of the combined Factory + House production/economy substrate

This is what the substrate **owns** in a normal YR skirmish — the player-observable contract a Rust replacement must reproduce. Each line is the *behavior*, not the C++ structure.

### 1.1 FactoryClass — the per-house, per-category production state machine

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| F1 | One production state machine **per house, per build category** (vehicle, ship, aircraft, infantry, building, defense); lazily allocated on first Begin_Production for that category and stored in the owning house's `Primary_For*` slot. | VERIFIED | `decompile 0x004FA350`: `operator_new(0x74)` + ctor when the `Primary_For*` slot is null, then stored back. |
| F2 | **Progress counter 0→54** (`Production_Value` +0x24, complete at `0x36`); advances by `Production_Step` (+0x3C, always 1 in YR) once per timer expiry. | VERIFIED | `disassemble 0x004C9B20`: `CMP [ESI+0x24],0x36`; `ADD …,Step; MOV [ESI+0x24]`. |
| F3 | **Per-step build timer** (`Production_Timer_Duration` +0x38 / CDTimer block +0x2C..+0x34) holding the per-step frame rate = `GetBuildStepTime()/54`, clamped [1,255]. | VERIFIED | `SetRate 0x004C9EA0`: magic `÷0x36`, clamp [1,255], rate written to both +0x34 and +0x38. |
| F4 | **Pay-as-you-go credit charging**: each step charges `⌊Balance/(54−Value)⌋` (signed-trunc); on the final step (`54−Value==0`) the IDIV is skipped via a divide-by-zero guard and the charge is the **entire remaining Balance, charged once** (the completion-block `Spend_Money` then runs as `Spend_Money(0)` because the pay branch already zeroed Balance — NOT a second full-Balance charge); insufficient credits → set `OnHold`, **rewind progress by 1** (stall, no net advance), no money spent. VERIFIED-LIVE v2. | VERIFIED | `decompile/disassemble 0x004C9B20`: cost formula + `if avail<cost { OnHold=1; PV-- } else { Spend_Money; Balance-=cost }`. `read_memory 0x004C9BD5/0x004C9BF1`; `disassemble 0x004C9B20` (V1). |
| F5 | **Exact-cost settlement on completion**: at `Value==54` set `IsSuspended`, zero timer, `Spend_Money(remaining Balance)`, `Balance=0` — so the house pays exactly `OriginalBalance` total despite per-step rounding — but in the normal path the remaining Balance is already 0 (charged in the final pay step), so this is `Spend_Money(0)`. VERIFIED-LIVE v2 (`disassemble 0x004C9B20` @0x4c9c06-34): `IsSuspended=1`, Duration(+0x38)=0, CDTimer +0x2c..+0x34 zeroed, `Spend_Money(Balance)`, `Balance(+0x60)=0`. | VERIFIED | `0x004C9B20` completion branch. |
| F6 | **Build queue** (`QueuedObjects` DynamicVector +0x40.., count +0x50) capped at `Rules MaximumQueuedObjects` (+0xF0); FIFO. | VERIFIED | `StartProduction 0x004C9C70` append + cap check vs `Rules+0xF0`. |
| F7 | **Object lifecycle**: holds the in-flight produced `TechnoClass*` (+0x58) from start (`type vtable+0x8C` Create) through factory-complete; object remains attached & suspended until delivery clears it. | VERIFIED | `0x004C9C70` create + `0x004CA1A0` CompletedProduction clears Object. |
| F8 | **Suspend / resume** (`IsSuspended` +0x70, `IsManual` +0x71): Suspend clears timer; SetRate resumes and recomputes rate; default `IsManual=1`. | VERIFIED | `Suspend 0x004C9E60`, `SetRate 0x004C9EA0`, ctor `IsManual=1`. |
| F9 | **Cancel + refund**: AbandonProduction refunds `GetCost(Owner) − Balance` (the already-paid amount only — NOT full cost, because money is spent incrementally), resets, destroys the partial object, clears AI tracking fields. | VERIFIED | `AbandonProduction 0x004C9FF0`: `Add_Credits(GetCost − Balance)`. |
| F10 | **Live re-rate on power change**: RecalcAllRates rewrites every same-house factory's `Production_Timer_Duration` (+0x38 only) when power state flips. | VERIFIED | `RecalcAllRates 0x004CA6E0`, sole caller `AI_AssessPower 0x00508C30`. |
| F11 | **Dirty/changed signalling** (`IsDifferent` +0x5D read-and-reset by HasChanged; `Production_HasChanged` +0x28) — sidebar-render-only, **not** state-hash input. | VERIFIED | `HasChanged 0x004C9C60`; flags consumed by StripClass::AI (render path). |
| F12 | **SpecialItem / superweapon path** (`SpecialItem` +0x68, −1 = none): same 54-step progress for special/SW production. | VERIFIED | `IsComplete 0x004CA130`, `AI 0x004C9B20` `SpecialItem != -1` branch. |

### 1.2 HouseClass — per-player economy, power, prerequisites, lifecycle

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| H1 | **The wallet**: `Balance`/AvailableCredits (+0x30C, internal ×100 scale); Add_Credits / Spend_Money operate here, with silo-drain fallback when Balance is insufficient. | VERIFIED | `Add_Credits 0x004F9950` (`[+0x30C]+=`); `Spend_Money 0x004F9790` silo-drain loop. |
| H2 | **Ore→credit conversion**: deposit adds `trunc(TibValue × IncomeMult × amount)` to Balance and `trunc(amount×5.0)` to HarvestedCredits (statistics); + purifier bonus `OrePurifierCount(house+0x538C) × PurifierBonus(Rules+0xf3c) × amount` (+ AIVirtualPurifiers[house+0x184] for AI houses); **both** the base `removed` and the `bonus` are independently passed through Add_Tiberium_Credits, so each is re-multiplied by `TibValue × IncomeMult`. IncomeMult = **HouseTypeClass+0x148** (stock 1.0), not a HouseClass field. The main ore→credit conversion has **no cap** (only the WEED path caps at TiberiumStorageLimit). | VERIFIED | **bonus base = `house+0x538C` = OrePurifier building COUNT (±1 per OrePurifier building), NOT StorageCapacity** (v1 said `house+0x538C` StorageCapacity bales; v2 REFUTED → OrePurifier building count, evidence R4/G1). VERIFIED-LIVE v2 (`decompile 0x00522D50/0x00445F80/0x00445880`; `read_memory 0x004604d8`; R4/G1). IncomeMult @ HouseTypeClass+0x148 (`disassemble 0x004F9610`, `read_memory 0x00511cfb`, R4). |
| H3 | **Power accounting**: sums `PowerOutput` (+0x53A4) / `PowerDrain` (+0x53A8) over owned buildings each assess; floors both to ≥0 each tick; continuous `PowerRatio` (output/drain when output<drain). Occupied-reactor (garrisoned power plant) zeroes output **even past blackout-timer expiry**. | VERIFIED | `AI_AssessPower 0x00508C30`, `GetPowerRatio 0x004FCE30` (`[+0x53A4]/[+0x53A8]`), `Update 0x004F8440` floor; occupied-reactor zeroing via `local_d` (Digest E D8). |
| H4 | **Low-power production slowdown** (continuous): the build-time consumer divides step time by `clamp(1−(1−ratio)×LowPowerPenaltyModifier, Min, Max)`. | VERIFIED | `GetBuildStepTime 0x006F47A0` FPU block; Rules +0x570/+0x574/+0x578. |
| H5 | **Multiple-factory speedup**: step time × `MultipleFactory^(count−1)` per extra same-category factory. | VERIFIED | `GetBuildStepTime 0x006F47A0` loop; `GetFactoryCount 0x00500910`; Rules +0x57C. |
| H6 | **Prerequisite / tech gating**: CanBuild evaluates Prerequisite groups (POWER/FACTORY/BARRACKS/RADAR/TECH/PROC tokens −1..−6), TechLevel (type +0x634 vs house +0x1D4), RequiredHouses (+0xDA0) / ForbiddenHouses (+0xDA4) owner bitmask, stolen-tech bits, BuildLimit (type +0x3B8 → −1 = greyed). | VERIFIED | `CanBuild 0x004F7870` token switch + side-bit `1<<Type[0xB8]`. |
| H7 | **Factory-pointer ownership**: holds the 6 `Primary_For*` factory pointers (+0x53AC..+0x53CC); these are the binding from house+category → FactoryClass instance. | VERIFIED | `Begin_Production 0x004FA350` reads/writes them by RTTI category (binding VERIFIED-LIVE v2 — see §2b corrected table). |
| H8 | **Per-frame tick** (`Update`, vtable +0x5C / slot 23): power/radar recheck, superweapon-ready, defeat detection, AI choosers (8-frame cadence), **superweapon** manage/resume (+0x1FC dirty tail). It does **NOT** drive the factory step — that is the separate global PerTickUpdate factory loop. | VERIFIED | `Update 0x004F8440`; vtable slot confirmed V2; +0x1FC → AI_ManageProduction/AI_ResumeProduction = SUPERWEAPON (Digest D D). |
| H9 | **Win/loss/defeat lifecycle**: IsDefeated (+0x1F5), HasWon (+0x1F7), HasLost (+0x1F8), scatter-pending (+0x1F6); MPlayer_Defeated effects (scatter, reveal, sidebar collapse, EVA, optional destroy owned **units only** (g_UnitClass_Array) under SpecialFlags 0x800); borrowed-time only when `g_GameMode∉{0,5}` (LAN/WOL/MP). | VERIFIED | `MPlayer_Defeated 0x004FC0B0`, `Flag_To_Win 0x004FC9E0`, `Flag_To_Lose 0x004FCBD0`, `Update 0x004F8440` detection. Units-only destroy + GameMode gate VERIFIED-LIVE v2 (E2). |
| H10 | **Identity / diplomacy**: HouseIndex (+0x30), Type (+0x34), **CurrentPlayer/IsHuman (+0x1EC**, struct label 'CurrentPlayer' = human-vs-AI roster flag**), PlayerControl (+0x1ED**, local-viewport / g_PlayerPtr owner**), SideIndex (+0x1E8), TechLevel (+0x1D4); directional `Allies` bitmask (+0x5788) mutated by MakeAlly/BreakAlliance. | VERIFIED | Constructor `0x004F54A0`, `IsAlliedWith 0x004F9A50`, `MakeAlly 0x004F9B70`. VERIFIED-LIVE v2: ctor 0x004F54A0 defaults both 0; Create_Houses sets +0x1EC=1 all humans, +0x1ED=1 + g_PlayerPtr only for local (E1). |
| H11 | **Creation pipeline** (MP): `Create_Houses` allocates `operator_new(0x160B8)`, priority-sorts humans by ColorIndex, then AI slots, then Neutral + Special; sets PlayerPtr, color scheme, starting credits. Sources are session/lobby globals, not INI: TechLevel←DAT_00822cf4, credits←DAT_00a8b25c (Set_Credits_And_Color arg3), color←PriorityToColorScheme; HouseIndex(+0x30)=g_HouseClass_Array_Count at ctor (registration order). VERIFIED-LIVE v2 (E1). | VERIFIED | `Create_Houses 0x00687F10`; size 0x160B8 from `operator_new`. |

### 1.3 Global plumbing the substrate depends on

| # | Responsibility | Active-YR | Evidence |
|---|---|---|---|
| G1 | **Global FactoryClass registry** `g_FactoryClass_Array @0x00A83E34` (count @0x00A83E40): every FactoryClass self-registers in its ctor and unregisters (shift-left removal) in its dtor. | VERIFIED | ctor/dtor register/unregister; RecalcAllRates iterates it. |
| G2 | **The tick driver**: `LogicClass::PerTickUpdate @0x0055AFB0` walks `g_FactoryClass_Array` calling `FactoryClass::AI` (vtable +0x5C) on each, **then** walks `g_HouseClass_Array @0x00A8022C` calling `HouseClass::Update`. Two separate sequential global loops; **factories tick before houses**. | VERIFIED | `disassemble 0x0055AFB0`: factory loop `0x55b66a..b68b`, house loop `0x55b68d..b6b1`; vtable targets read from memory (V3). |
| G3 | **Pending-vehicle delivery globals**: `DAT_00B0FE5C` (land) / `DAT_00B0FE60` (naval) hold the just-completed vehicle awaiting placement-ghost; set by `FUN_00734250`. | VERIFIED | `FUN_00734250 0x00734250` setter; getters 0x007342A0/B0. |
| G4 | **Frame counter** `g_CurrentFrameCounter @0x00A8ED84` advances **late**, in `Main_Tick @0x0055D360` after the full logic pass, gated off when paused/desynced. | VERIFIED | `Main_Tick` write @0x0055DE81; all PerTickUpdate refs are reads (V3). |

---

## 2. Full inventory

### 2a. FactoryClass methods

| Name | Address / offset | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| Constructor | 0x004C98F0 | `new(0x74)`; registers into `g_FactoryClass_Array` (+ second live vector DAT_00B0F720); inits Step=1, SpecialItem=−1, IsManual=1, CapIncr=10 | YES | VERIFIED (Digest A) |
| AI (per-tick stepper) | 0x004C9B20 — **vtable slot +0x5C** | Advance progress by Step on timer expiry; per-step credit charge; OnHold + rewind on shortfall; settle on completion | YES | VERIFIED (`decompile`/`disassemble 0x004C9B20`, V1) |
| HasChanged | 0x004C9C60 | Read-and-reset `IsDifferent` (+0x5D) — sidebar poll | YES (render) | VERIFIED (Digest A) |
| StartProduction | 0x004C9C70 | Start fresh Object (create via type vtable+0x8C, set Owner +0x21C, Balance = full GetCost, mirror to Object+0x300) OR append to queue (cap vs Rules+0xF0) | YES | VERIFIED (`decompile 0x004C9C70`, V1) |
| Suspend(bool) | 0x004C9E60 | If running: set IsManual=arg, IsSuspended=1, zero Duration(+0x38), clear timer | YES | VERIFIED (Digest A) |
| SetRate(bool) | 0x004C9EA0 | Resume; rate = `(Object? GetBuildStepTime():0)/0x36` clamp [1,255]; write to +0x34 **and** +0x38; affordability check on next step | YES | VERIFIED (Digest A, magic ÷54) |
| CalcRate | 0x004C9FB0 | Read-only rate query (`GetBuildStepTime/0x36`, clamp [1,255]) | YES | VERIFIED (Digest A) |
| AbandonProduction (cancel/refund) | **0x004C9FF0** (body 0x004C9FF0-0x004CA11C) | Refund `GetCost(Owner) − Balance` via Add_Credits; reset; destroy partial Object; clear AI tracking @Owner +0x564C/50/54/58 | YES | VERIFIED-LIVE v2 (`disassemble 0x004C9FF0`; refund calc 0x004CA037 `CALL [+0x84]`(Owner) − [ESI+0x60]Balance → `Add_Credits 0x004F9950`; ECX=Owner@+0x6c). (v1 listed 0x004CA0E0 as an alias; v2 REFUTED → 0x004CA0E0 is an INTERIOR address (AI-tracking-clear block), not a second entry, evidence `get_function_by_address 0x004CA0E0`→Entry 0x004C9FF0.) |
| GetProgress | 0x004CA120 | `return Production_Value` (+0x24) | YES | VERIFIED (`decompile`, V1) |
| IsComplete | 0x004CA130 | `(Object && PV==0x36) \|\| (SpecialItem!=−1 && PV==0x36)` | YES | VERIFIED (`decompile`, V1) |
| GetObject | 0x004CA160 | `return Object` (+0x58) | YES | VERIFIED (Digest A) |
| CompletedProduction | 0x004CA1A0 | On PV==0x36: clear Object, IsSuspended=1, IsDifferent=1, PV=0, zero timer; **does NOT start next**. Predicate asymmetry: AI/SetRate/CompletedProduction/AbandonProduction use `SpecialItem==0`='none'; only ctor-init and IsComplete use `-1`. **NOT confirmed dead** — the SpecialItem (+0x68) writer (SW/special-begin path) was not located, so value 0 cannot be proven unreachable. DRIFT/UNCHECKED — do NOT collapse 0 and -1 in the port. | YES | VERIFIED-LIVE v2 (the convention); writer UNVERIFIABLE. (v1 framed the asymmetry as safe-to-collapse dead code; v2 REFUTED → cannot prove value 0 unreachable.) |
| StartNextQueued | 0x004CA5A0 | Requires queue non-empty, Object null, (Duration==0 \|\| IsSuspended); pop **front** (shift down), re-Begin_Production(resume=1) | YES | VERIFIED (Digest C) |
| RemoveFromQueue | 0x004CA620 | Remove **first** matching index (front-to-back), shift down | YES | VERIFIED (Digest C) |
| RecalcAllRates(house) | 0x004CA6E0 | For each same-house factory rewrite Duration(+0x38 only) = `(Object? GetBuildStepTime():0)/0x36` clamp [1,255] | YES | VERIFIED (Digest A) |
| GetBuildStepTime | 0x006F47A0 | Per-step time, `this` = the **object under construction** (NOT the factory). Order: `s1=trunc(HouseBuildTimeBonus×Cost)` → `s2=trunc(s1×Type+0x608 BuildTimeMultiplier)` → `s3=trunc(s2 / clamp(1−(1−ratio)×LPPM[+0x578], Min[+0x570], Max[+0x574]))` (Max clamp only when ratio<1.0; divisor floored to 0.01 if ≤0) → MultipleFactory loop `trunc(acc×Rules+0x57c)` repeated (n−1)× **with per-iteration truncation**, gated `Rules+0x57c > 0.0` → wall branch `trunc(s4×Rules+0x758 BuildSpeed[double])` ONLY if RTTI==6 && object+0x520→+0x1571≠0. ÷54 + clamp[1,255] happens in **callers**, not here. **NO ×0.9, NO universal BuildSpeed.** | YES | **VERIFIED-LIVE v2** `disassemble_function 0x006F47A0` (R1). (v1 said base = `trunc(Cost×BuildSpeed×0.9)`; v2 REFUTED → base = `trunc(BuildTimeBonus×Cost)`, no ×0.9 — `0x007e2ac8` is **1.0f**, the `1.0` in the low-power `1−x` clamp, not a 0.9 factor.) |
| ScalarDeletingDestructor | 0x004CA790 | Unregister from `g_FactoryClass_Array` (find-index, dec count, shift-left); AbandonProduction if g_GameActive; free queue | YES | VERIFIED (Digest A) |

**FactoryClass struct field map (size 0x74, live `get_struct_layout` this session):**

| Off | Field | Off | Field |
|---|---|---|---|
| +0x00 | vtable | +0x4C | QueuedObjects_IsInit |
| +0x04/08/0C | IPersist/IRTTIInfo/INoticeSink vtables | +0x4D | QueuedObjects_IsAlloc |
| +0x24 | Production_Value (0→54) | +0x50 | QueuedObjects_Count |
| +0x28 | Production_HasChanged | +0x54 | QueuedObjects_CapIncr (10) |
| +0x2C | Timer_StartTime | +0x58 | Object (TechnoClass*) |
| +0x30 | Timer_pad (dead) | +0x5C | OnHold |
| +0x34 | Timer_TimeLeft (CDTimer dur) | +0x5D | IsDifferent |
| +0x38 | Production_Timer_Duration (the rate; RecalcAllRates target) | +0x60 | Balance (remaining-to-pay) |
| +0x3C | Production_Step (1) | +0x64 | OriginalBalance |
| +0x40 | QueuedObjects_vtable | +0x68 | SpecialItem (−1=none) |
| +0x44 | QueuedObjects_Items | +0x6C | Owner (HouseClass*) |
| +0x48 | QueuedObjects_Capacity | +0x70 | IsSuspended |
| | | +0x71 | IsManual (default 1) |

### 2b. HouseClass methods relevant to production/economy/lifecycle

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| Constructor | 0x004F54A0 | `new(0x160B8)`; assign HouseIndex(+0x30), Type(+0x34), SideIndex(+0x1E8); zero defeat/result flags; self-set +0x1D8 editor mask; subscribe to 5 global removal-listener vectors + bidirectionally cross-register 2 per-peer-house score/flag vectors (O(N²)) + g_HouseClass_Array | YES | VERIFIED (Digest F); cross-registration VERIFIED-LIVE v2 (E2) |
| Update (per-frame tick) | 0x004F8440 — **vtable +0x5C, slot 23** | Power/radar recheck, SW-ready, defeat detect, AI choosers (8-frame), +0x1FC **superweapon** manage/resume tail. Does NOT step factories. | YES | VERIFIED (`decompile`, V2/V3; +0x1FC=SW per Digest D) |
| Begin_Production | 0x004FA350 | Resolve factory pointer by BuildCat (naval split via type+0xE08==5); prereq gate (`type vtable+0x94`, 2-arg form, resume-retry); lazy-alloc FactoryClass; StartProduction; SetRate; sidebar tab refresh; queued-no-start return | YES | VERIFIED (`decompile 0x004FA350` this session) |
| Place_Production | 0x004FB0E0 | Delivery commit (sole caller EventClass::Execute): building Unlimbo path OR vehicle ExitObject auto-exit path; CompletedProduction + FUN_004FAA10 + Record_Last_Built on success | YES | VERIFIED (Digest C) |
| Add_Credits | 0x004F9950 | `[+0x30C] += amount` | YES | VERIFIED (`decompile 0x004F9950`, V2) |
| Spend_Money | 0x004F9790 | Deduct from Balance(+0x30C); if short, drain building ore storage; track SpentCredits(+0x2DC); no per-step rollback (that's FactoryClass) | YES | VERIFIED (Digest E) |
| Add_Tiberium_Credits | 0x004F9610 | `Balance += trunc(TibValue×IncomeMult×amount)`; `HarvestedCredits(+0x54E8) += trunc(amount×5.0)` | YES | VERIFIED (Digest E). VERIFIED-LIVE v2: `[+0x54E8] = ftol(amount×5.0 + …)` (const 5.0 @0x007eaa00), Balance += `ftol(TibValue[type+0xb8] × IncomeMult[HouseTypeClass+0x148] × amount)` (`disassemble 0x004F9610`, R4). |
| DepositOreFromStorage | 0x00522D50 | Per non-empty slot: purifier bonus = `OrePurifierCount(house+0x538C) × PurifierBonus(Rules+0xf3c) × amount` (+ AIVirtualPurifiers[house+0x184] for AI); each of base-removed AND bonus separately re-multiplied by TibValue×IncomeMult inside Add_Tiberium_Credits; **no cap on credit conversion** | YES | VERIFIED-LIVE v2 (`decompile 0x00522D50`, R4). (v1 said base = `StorageCapacity(+0x538C bales)`; v2 REFUTED → OrePurifier building count.) |
| DepositWeedCredits | 0x004F9700 | WEED path; caps weed storage at Rules+0x17D0 TiberiumStorageLimit | YES (weed only) | VERIFIED (Digest E A5) |
| CanBuild | 0x004F7870 | Prereq tokens −1..−6, TechLevel, RequiredHouses/ForbiddenHouses bitmask, stolen-tech, BuildLimit (−1 greyed); AI campaign prereq shortcut | YES (AI shortcut deferred) | VERIFIED (Digest E) |
| GetPowerRatio | 0x004FCE30 | `output<drain && drain!=0 ? output/drain : 1.0` (output==0 → 0.0); reads +0x53A4/+0x53A8 | YES | VERIFIED (`disassemble 0x004FCE30` this session) |
| AI_AssessPower | 0x00508C30 | Recompute PowerOutput/Drain over buildings; blackout/occupied-reactor zeroing (zeroes output past blackout expiry for garrisoned plant); RecalcAllRates; AI_ManageProduction on transition; set RecheckRadar | YES | VERIFIED (`decompile 0x00508C30` this session) |
| GetFactoryCount | 0x00500910 | Per-RTTI factory count (reads +0x5378..+0x5388); feeds MultipleFactory | YES | VERIFIED (V4) |
| Find_Factory | 0x004F83C0 | Scan g_FactoryClass_Array for factory whose produced-object RTTI matches; pure lookup | YES | VERIFIED (Digest B; was mislabeled AI_Tick) |
| Set_Credits_And_Color | 0x004FCE00 | **Skirmish/MP credit-init path** (sole caller Create_Houses). 3-arg (RET 0xc): arg1→Type+0xC0 & +0x16054 (color, +0x16054 later overwritten by PriorityToColorScheme), arg3→+0x1dc & +0x30c (raw lobby credits, no ×100; both writes share one EAX asm 0x004fce07/0x004fce0d). arg2 ('country') never read. | YES | VERIFIED-LIVE v2 (`disassemble 0x004FCE00`, R3/E1) |
| Read_Scenario_INI | 0x00500B40 | **Campaign only**: per-house INI. Campaign path writes +0x1dc = `Credits*100` then derives Balance(+0x30c) with [Difficulty] bonus (Rules+0xdfc/+0xe00 by DAT_00a8eb64) + negative-clamp. (Also TechLevel, PlayerControl, IQ, Edge, Color, Allies.) | YES (campaign) | VERIFIED-LIVE v2 (`disassemble 0x00500B40`, R3/E1) |
| IsAlliedWith | 0x004F9A50 | One-directional read of `Allies`(+0x5788); self/same-index short-circuit true | YES | VERIFIED (Digest F) |
| MakeAlly | 0x004F9B70 | `this->Allies \|= 1<<other.idx` (directional; mutual = 2 calls) | YES | VERIFIED (Digest F) |
| BreakAlliance | 0x004F9F90 | Clear this→other bit; if mutual, clear other→this too | YES | VERIFIED (Digest F) |
| MPlayer_Defeated | 0x004FC0B0 | Ordered effects: IsDefeated(+0x1F5)=1 → Recalculate_Alliances (MP, AI gate) → Clear_Rally (Scenario&0x10) → **destroy owned `g_UnitClass_Array` units ONLY** (not infantry/aircraft/buildings) via vtable+0xF8, gated `g_GameMode!=0 && g_ScenarioClass&0x800` → local-player reveal+sidebar-collapse+EVA → game-completion scan (reads +0x5788 Allies directionally) → Flag_To_Win/Lose | YES | VERIFIED-LIVE v2 (`decompile 0x004FC0B0`, E2). (v1 said "destroy-all-owned"; v2 REFUTED → units-array only.) |
| Flag_To_Win | 0x004FC9E0 | Set HasWon(+0x1F7); borrowed-time only when `g_GameMode ∉ {0 campaign, 5 skirmish}` (LAN/WOL/MP); writes timer block +0x298/+0x29c/+0x2a0; victory EVA | YES | VERIFIED-LIVE v2 (`decompile 0x004FC9E0`, E2) |
| Flag_To_Lose | 0x004FCBD0 | Set HasLost(+0x1F8); borrowed-time only when `g_GameMode ∉ {0 campaign, 5 skirmish}` (LAN/WOL/MP); writes timer block +0x298/+0x29c/+0x2a0; defeat EVA | YES | VERIFIED-LIVE v2 (`decompile 0x004FCBD0`, E2) |
| AI_ManageProduction / AI_ResumeProduction | 0x0050AF10 / 0x0050B1D0 | **SUPERWEAPON** grant/suspend/resume/cameo (NOT the factory step, NOT the production queue) — runs in the +0x1FC tail. Belongs to the superweapon sub-system, not this substrate. | YES (SW) | DOC-ONLY (CALL placement verified V3; bodies not re-decompiled) |

**Relevant HouseClass fields (offset → field):**

| Off | Field | Off | Field |
|---|---|---|---|
| +0x30 | HouseIndex | +0x30C | Balance/AvailableCredits (×100) |
| +0x34 | Type (HouseTypeClass*) | +0x53A4 | PowerOutput |
| +0x1D4 | TechLevel | +0x53A8 | PowerDrain |
| +0x1DC | StartingCredits seed: campaign=Credits×100, MP=raw slot value; INERT after init (seeds +0x30c Balance; save-serialized only). RESOLVED v2 (R3/E1) | +0x53AC..+0x53CC | 6 Primary_For* factory pointers |
| +0x1EC | IsHuman | +0x538C | **OrePurifierCount** (±1 per OrePurifier building via OnConstructionComplete 0x00445F80 / Limbo 0x00445880, gated BuildingTypeClass+0x16cc='OrePurifier'; purifier-bonus base) — v1 said StorageCapacity, REFUTED v2 (R4/G1) |
| +0x1ED | PlayerControl | +0x54E8 | HarvestedCredits (×5.0, statistics) |
| +0x1E8 | SideIndex | +0x2DC | SpentCredits |
| +0x1F5 | IsDefeated | +0x6C/+0x78 | OwnedObjects array / count |
| +0x1F6 | scatter-pending | +0x2F0 | OwnedBuildings |
| +0x1F7 | HasWon | +0x1FC | superweapon-manage dirty flag |
| +0x1F8 | HasLost | +0x5788 | Allies (directional bitmask, 32-bit `1<<(idx&0x1f)`) |

**Primary_For* offset ↔ category map — RESOLVED v2 (binary-verified across 4 byte-identical dispatch tables in Begin_Production + FUN_004FAA10 + Place_Production, keyed by RTTI_To_TypeArray 0x0048DCD0; corroborated by `get_struct_layout HouseClass`).** Dispatch is keyed by RTTI (the in_stack_00000004 param), NOT a separate BuildCat; secondary keys are the naval flag byte and BuildingTypeClass+0xE08==5. +0x53C0/+0x53C4/+0x53C8 are NOT factory pointers (untouched by any production function). (v1 attached RTTI 15,16 to +0x53AC and RTTI 2,3 to +0x53B0; v2 REFUTED → RTTI travels WITH category: +0x53AC=Aircraft RTTI 2,3 and +0x53B0=Infantry RTTI 0xf,0x10, evidence R2/E2.)

| Offset | Field (VERIFIED-LIVE v2) | RTTI / split |
|---|---|---|
| +0x53AC | Primary_ForAircraft | RTTI 2,3 (→ g_AircraftTypeClass_Array) |
| +0x53B0 | Primary_ForInfantry | RTTI 0xf,0x10 (→ g_InfantryTypeClass_Array) |
| +0x53B4 | Primary_ForVehicles | RTTI 1,0x28, naval-flag([ESP+0x30])==0 (→ g_UnitTypeClass_Array) |
| +0x53B8 | Primary_ForShips | RTTI 1,0x28, naval-flag!=0 |
| +0x53BC | Primary_ForBuildings | RTTI 6,7, BuildingTypeClass+0xE08 != 5 (→ g_BuildingTypeClass_Array) |
| +0x53CC | Primary_ForDefenses | RTTI 6,7, BuildingTypeClass+0xE08 == 5 |

#### 2b.1 House-creation INI → offset map (VERIFIED-LIVE v2, E1)

**Caller gating:** Full_Init 0x00686b20 routes `g_GameMode==0` → FUN_005009b0 (campaign, per-`[Houses]` Read_Scenario_INI) and `g_GameMode!=0` → Create_Houses (MP/skirmish).

**Campaign (Read_Scenario_INI 0x00500B40):**

| INI key | string addr | offset | value |
|---|---|---|---|
| TechLevel | 0x824e40 | +0x1D4 | ReadInt(default Scenario+0x1254) |
| Credits | 0x824e38 | +0x1DC | ReadInt×100 (raw; Balance derived separately) |
| PlayerControl | 0x824e28 | +0x1ED | ReadBool |
| IQ | 0x824dd8 | +0x1D0 AND +0x24C | ReadInt; if >Rules+0x1434 → clamp 1 |
| Edge | 0x824dd0 | +0x1E0 | FUN_00475980(default -1) |
| Color | 0x81b138 | +0x16110 | FUN_00474a90(default Type+0xC0); <0→5 |
| Allies | 0x824d5c | +0x5788 | self-ally first, then MakeAlly per set bit |
| (Side from Type) | — | +0x1E8 | Type+0xBC; ==-1 → 0 (Allied) |
| (Balance) | — | +0x30C | =+0x1DC + [Difficulty] bonus (Rules+0xDFC/+0xE00 by DAT_00a8eb64) when PlayerControl && GameMode==0, clamp≥0; else =+0x1DC |

HouseTypeClass source (ReadINI 0x00511850): Side @+0xBC (str 0x817334), Color @+0xC0 (str 0x81b138).

**MP (Create_Houses 0x00687F10 → Set_Credits_And_Color 0x004FCE00)** — sources are lobby globals, not per-house INI: Type/Country +0x34 (g_HouseTypeClass_Array[node+0x4b]); TechLevel +0x1D4 ← DAT_00822cf4; Credits +0x1DC & +0x30C ← DAT_00a8b25c (raw, no ×100); Color Type+0xC0 & +0x16054 ← arg1 then PriorityToColorScheme; IsHuman +0x1EC =1 humans/=0 AI; PlayerControl +0x1ED =1 + g_PlayerPtr only for local; HouseIndex +0x30 = g_HouseClass_Array_Count.

### 2c. Global helpers & free functions

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| LogicClass::PerTickUpdate | 0x0055AFB0 | Tick spine: walks g_FactoryClass_Array (FactoryClass::AI) then g_HouseClass_Array (HouseClass::Update) | YES | VERIFIED (`disassemble`, V3) |
| EventClass::Execute | 0x004C6CB0 | Lockstep command dispatch: 0x0E Begin, 0x0F Suspend, 0x10 Cancel-one, 0x2E Cancel-all, 0x0B Place. **Sole caller of Place_Production**. The 0x0B event carries event+0xb's heapId (auto-deliveries from StripClass carry −1; player building placement carries the real value). The −1-vs-≥0 distinction lives in **FUN_004faa10** (heapId<0 ⇒ delivery: AbandonProduction-noop + StartNextQueued; heapId≥0 ⇒ cancel-one RemoveFromQueue) — do NOT state '0x0B always = heapId −1'. | YES | VERIFIED (Digest C); heapId routing VERIFIED-LIVE v2 (`decompile 0x004C6CB0`/`0x004FAA10`; V5) |
| FUN_004FAA10 (queue restart / cancel) | 0x004FAA10 | `(house, rtti, heapId, naval, removeAll)`: heapId≥0 → RemoveFromQueue; heapId=−1 → AbandonProduction(no-op after completion) + StartNextQueued | YES | VERIFIED (Digest C) |
| StripClass::AI (delivery/flash) | 0x006A8B30 | Per cameo: HasChanged+IsComplete+GetObject; **unit/aircraft/infantry (RTTI 1/2/0xf) → auto-emit 0x0B Place (heapId=−1) via FUN_004c6ae0**; **building (RTTI 6) → PlayEVA + FUN_00734250 (set pending-BUILDING placement ghost)**; drives progress bar + flash | YES (sidebar) | VERIFIED-LIVE v2 (`decompile 0x006A8B30` switch on produced vtable+0x2c; V5). (v1 had building→0x0B and vehicle→FUN_00734250; v2 REFUTED → inverted: unit/aircraft/infantry→0x0B, building→FUN_00734250.) |
| FUN_00734250 (pending-building ghost setter) | 0x00734250 | **Pending-BUILDING placement-ghost setter** (StripClass case RTTI 6 only): defense (`Type+0xe08==5`)→DAT_00B0FE60 else regular building→DAT_00B0FE5C. NOT vehicle-related. | YES | VERIFIED-LIVE v2 (`decompile 0x00734250`; V5). (v1 said "pending-vehicle"; v2 REFUTED → pending-building.) |
| Prereq revalidation ("UpdateRadar" misnomer) | 0x00509140 | Queued items: single gate `(1,0,1,house)`, **dropped if permanently unbuildable** (back-to-front compaction), never individually suspended. Active object: gate `(1,0,1)` fail → **AbandonProduction + StartNextQueued** (permanent); else gate `(1,1,1)` fail → **Suspend(false)** (temporary, +sidebar if local); else **SetRate-resume** if `IsSuspended && !IsManual`. Empty factory (no Object, no queue) → **self-delete** via `vtable+0x20` (0x004CA770). Callers: GoOnline/GoOffline/Limbo/Unlimbo/ReadFromINI (capture flows through Limbo/Unlimbo — NOT a distinct caller). | YES | VERIFIED-LIVE v2 (`disassemble 0x00509140`, `get_function_callers 0x00509140`, `read_memory 0x007E88F0`; V4) |
| RTTI_To_TypeArray | 0x0048DCD0 | RTTI → type-array index used by Begin/Place to resolve category | YES | VERIFIED (referenced in 0x004FA350 this session) |
| FUN_005007a0 | 0x005007A0 | Naval-split / category normalization helper called by Begin_Production | YES | VERIFIED (call seen this session) |
| Record_Last_Built | (in Place_Production tail) | Records last-built type after successful delivery | YES | DOC-ONLY |
| Main_Tick | 0x0055D360 | Caller of PerTickUpdate; late `g_CurrentFrameCounter++` (pause/desync-gated) | YES | VERIFIED (V3) |

### 2d. Singleton / global state

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| g_FactoryClass_Array | 0x00A83E34 | Pointer-to-pointer array of all FactoryClass instances | YES | VERIFIED (V1/V3) |
| g_FactoryClass_Array_Count | 0x00A83E40 | Count for above; reloaded each loop iter (tolerates mid-loop change) | YES | VERIFIED (V1/V3) |
| (factory vector control block) | 0x00A83E30 | DynamicVector control (grow vtbl+8, find-index vtbl+0x10), cap +0x08, IsAlloc +0x0D, CapIncr +0x14 | YES | VERIFIED (Digest A) |
| (second live-objects vector) | 0x00B0F720 / 724 / 730 | Parallel all-objects vector also registered in ctor; role INFERRED | YES | INFERRED (Digest A) |
| g_HouseClass_Array | 0x00A8022C | Array of all HouseClass instances | YES | VERIFIED (V3/Digest D) |
| g_HouseClass_Array_Count | 0x00A80238 | Count for above | YES | VERIFIED (V3/Digest D) |
| Pending building (regular) | 0x00B0FE5C | pending **regular-building** placement-ghost. NOT a vehicle. (v1 said pending land vehicle; v2 REFUTED, V5) | YES | VERIFIED-LIVE v2 (V5) |
| Pending building (defense) | 0x00B0FE60 | pending **defense-building** (Type+0xe08==5) placement-ghost. NOT a vehicle. (v1 said pending naval vehicle; v2 REFUTED, V5) | YES | VERIFIED-LIVE v2 (V5) |
| g_CurrentFrameCounter | 0x00A8ED84 | Authoritative gameplay frame counter; advanced late in Main_Tick | YES | VERIFIED (V3) |
| g_PlayerPtr | (global) | Local player's HouseClass*; gates sidebar/EVA/UI side effects | YES | VERIFIED (referenced 0x004FA350 this session) |
| g_GameMode | (global) | 0=campaign, 5=skirmish, else MP; gates AI shortcut, borrowed-time, defeat | YES | VERIFIED (Digest E/F) |

### 2e. Registries (registration/removal mechanisms)

| Registry | Owner | Register on | Unregister on | Evidence |
|---|---|---|---|---|
| g_FactoryClass_Array | global | FactoryClass ctor (append at Count×4, Count++) | ScalarDeletingDestructor (find-index, dec, shift-left) | VERIFIED (Digest A) |
| g_HouseClass_Array | global | HouseClass ctor (ArrayIndex = Count, then Count++ & append) | (house dtor) | VERIFIED (Digest F) |
| Primary_For* factory slots | HouseClass +0x53AC..+0x53CC | Begin_Production lazy-alloc stores factory ptr | FUN_004FAA10 / prereq-revalidation null the slot when queue+object empty | VERIFIED (Digest C / this session) |
| 5 global removal subscriptions + 2 per-peer-house score vectors | global removal lists + HouseClass +0x5604..+0x5618 / +0x561C..+0x5630 | ctor subscribes to 5 global removal listeners + bidirectionally cross-registers the 2 per-peer score/flag vectors with every existing house (O(N²)) | dtor decrements lists | VERIFIED (Digest F); split VERIFIED-LIVE v2 (E2) |

### 2f. Static tables / data

| Table | Address / location | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| BuildCat→factory-handler jump table | 0x004FA890 | Begin_Production switch dispatch | YES | VERIFIED (V2) |
| RTTI→handler byte-index table | 0x004FA8A4 | `[0,1,1,4,4,2,2,…,3,3,…]` selecting the handler | YES | VERIFIED (V2) |
| Prereq token table | CanBuild switch −1..−6 → Rules offsets | POWER +0x35C/+0x368, FACTORY +0x378/+0x384, BARRACKS +0x394/+0x3A0, RADAR +0x3B0/+0x3BC, TECH +0x3CC/+0x3D8, PROC +0x3E8/+0x3F4 (+SMIN alt @+0x400 on +0xDF8) | YES | VERIFIED (Digest E) |
| AIVirtualPurifiers | Rules +0x1324 (ptr to int array, **offset VERIFIED-LIVE v2**), indexed by **house+0x184 (AI-difficulty field, identity UNVERIFIED this session)** | Adds virtual purifier count to AI ore-bonus base | YES (AI) | offset VERIFIED-LIVE (`decompile 0x00522D50`, R4); index-field identity still OPEN |
| SetRate ÷54 magic | `0x4BDA12F7` + `SAR 4` | Signed division by 54 for per-step rate | YES | VERIFIED (Digest A) |
| PriorityToColorScheme table | 0x0083ED14 (9 bytes) + 0x0083ED1C | Color-priority → scheme index | YES | DOC-ONLY (values not re-read) |
| Build-time Rules floats | Rules +0x570 (lower/Min clamp) / +0x574 (upper/Max clamp, gated ratio<1) / +0x578 (LowPowerPenaltyModifier) / +0x57C (MultipleFactory, loop gate `>0.0`) / +0x758 (BuildSpeed **double**, applied only in the RTTI==6 wall branch). Plus Type+0x608 (BuildTimeMultiplier, per-type, FMUL'd at T2). | YES | VERIFIED-LIVE v2 by usage (`disassemble 0x006F47A0`, R1). INI key-name mapping = INFERRED (not string-confirmed). |

### 2g. Vtable / COM slots used

| Slot | Class | Target | Role | Evidence |
|---|---|---|---|---|
| +0x5C | FactoryClass (vtable 0x007E88D0) | FactoryClass::AI @0x004C9B20 | Per-tick production step (dispatched by PerTickUpdate over the factory array) | VERIFIED (read_memory 0x007E892C, V1/V3) |
| +0x5C (slot 23) | HouseClass (vtable 0x007EA8A0) | HouseClass::Update @0x004F8440 | Per-frame house tick | VERIFIED (read_memory 0x007EA8FC, V2/V3) |
| +0x20 | FactoryClass | scalar-deleting destructor | Factory delete (called by Begin_Production rollback, FUN_004FAA10) | VERIFIED (Digest C) |
| +0x18 | HouseClass +0x24 credit sub-object | GetAvailableCredits | FactoryClass::AI affordability query (`[Owner+0x24] vtbl+0x18`). Spend_Money WRITES `[Owner+0x30C]`; the affordability READ goes via this +0x24 sub-object slot+0x18. That both reference the same wallet word is asserted (H1) but the +0x18 slot target was not decompiled — **read==write-wallet equivalence UNCHECKED.** | VERIFIED (V1) |
| +0x04/+0x08/+0x0C | FactoryClass / HouseClass | IPersist/IRTTIInfo/INoticeSink (OLE save/load) | COM persistence subobjects — not gameplay | VERIFIED (struct layout) |
| type +0x84 / +0x88 | TechnoTypeClass | GetCost / cost chain | Balance init + per-step cost basis | VERIFIED (Digest A/C) |
| type +0x8C | TechnoTypeClass | Create/Create_One | Produced-object creation in StartProduction | VERIFIED (V1) |
| type +0x94 | TechnoTypeClass | CanBuild gate | Begin_Production + prereq revalidation (2-arg form, resume-retry) | VERIFIED (this session 0x004FA350) |
| produced +0xD8 / +0x100 / +0x190 | TechnoClass / BuildingClass | Unlimbo (+0xD8) / ExitObject (+0x100) / **exit-target/cell resolver (+0x190)** (called `(0,0)`/`(0,1)` returning the exit helper; NOT a stored anchor) | Place_Production delivery | VERIFIED-LIVE v2 (`disassemble 0x004FB0E0`; V5). (v1 labeled +0x190 "anchor"; v2 REFUTED → exit-target/cell resolver.) |

### 2h. Legacy / dormant TS paths in this surface

| Item | Status | Evidence |
|---|---|---|
| Tunnel / subterranean coupling | **None** in any production/house/economy path | VERIFIED (V4: clean in FactoryClass::AI, StripClass::AI, GetBuildStepTime; Rust grep clean) |
| Fog-of-war / shroud-regrow gates (`*Scenario&0x1000`, Rules+0x17F0) | OFF by stock-YR default; not a production field | VERIFIED gate code (V3); INI defaults DOC-ONLY |
| +0x1D8 editor ally mask | Active only under `g_MapEditorMode`; **N/A for skirmish/gameplay** (the real Allies mask is +0x5788) | VERIFIED (Digest F) |
| Production_Timer_pad (+0x30) | Dead/scratch field (reads uninitialized in ctor) | VERIFIED (Digest A) |
| AI campaign prereq shortcut (CanBuild) | Reachable only for AI/non-player in campaign | VERIFIED (Digest E) — see Deferred bucket |

---

## 3. Active vs inactive/legacy/deferred split

### ACTIVE-YR — must be reproduced; the substrate's player-observable contract

| Item | One-line rationale |
|---|---|
| FactoryClass::AI 54-step progress + pay-as-you-go charging + OnHold stall | The build cadence and the "production halts when credits run out mid-build" behavior the player sees every match. |
| SetRate / GetBuildStepTime / RecalcAllRates rate model (÷54, clamp [1,255]) | Defines build timing; low-power and multi-factory speed both flow through it. |
| Begin/Place_Production + StartProduction + queue (FIFO, RemoveFromQueue front, StartNextQueued front) | The full begin→step→complete→deliver→exit pipeline and queue ordering. |
| AbandonProduction refund (`GetCost − Balance`, spent-only) | Cancel refund amount; visible on every cancel. |
| Pay-as-you-go + silo-drain Spend_Money + ore→credit deposit (TibValue×IncomeMult, purifier bonus = OrePurifierCount×PurifierBonus, no cap on conversion, +x5.0 HarvestedCredits) | The credit balance the player watches; bonus base is OrePurifier building count (v1 'silo capacity' REFUTED, R4/G1). |
| Power sums (+0x53A4/+0x53A8), GetPowerRatio, AI_AssessPower (incl. occupied-reactor zeroing), low-power slowdown, multiple-factory speedup | Power→production coupling; fires on every power flip and every multi-factory base. |
| CanBuild prereq/tech/owner-bitmask/BuildLimit gating | What is buildable / greyed in the sidebar. |
| Global tick order: factories-before-houses (PerTickUpdate), late frame-counter increment | Determines simultaneous-completion ordering and per-frame cadence (lockstep-relevant). |
| Pending-vehicle delivery globals + StripClass::AI delivery dispatch | Completed-vehicle placement-ghost / auto-exit; the unit-ready→appear behavior. |
| Prereq revalidation 3-way (drop/suspend/resume) on building add/remove/power | Queue reacts correctly when a prereq building is sold/destroyed/captured/powered-down. |
| HouseClass identity/diplomacy (directional Allies +0x5788, MakeAlly/BreakAlliance) | Alliance toggles, shared vision/build-off-ally, FFA team setup — every MP match. |
| Win/loss/defeat lifecycle (MPlayer_Defeated, Flag_To_Win/Lose, Update detection) | Match-end behavior, scatter, reveal, EVA cues. |
| House creation pipeline (Create_Houses, Neutral/Special, PlayerPtr, color, credits) | Per-player setup at match start. |

### DEFERRED-AI — active in YR but out of scope now (leave a clean seam, do not design internals)

| Item | One-line rationale |
|---|---|
| AI base-plan FSM: AI_Manage_Build_Queue (0x004FDD10) ← AI_Building_Strategy (0x004FD500) | AI-house only (gated `cVar2==0 && Type[+0x1A6]==0` in Update); human play never reaches it. |
| AI_Choose_Building/Unit/Aircraft/Infantry (8-frame cadence in Update) | AI production chooser; skipped for the human player. |
| AI prerequisite shortcut in CanBuild (campaign, non-player) | AI-player convenience path; not the human gate. |
| AI virtual purifiers (Rules+0x1324, AIVirtualPurifiers[diff]) | Economy bonus only for AI houses; the formula seam exists but the AI-side input is deferred. |
| AI delivery headstart precompute (Begin_Production non-player Production_Value precompute) | AI-only build-progress jumpstart. |
| Recalculate_Alliances (max-IQ AI re-ally on defeat) | AI/max-IQ-only (`Type[0x1a6]` gate + Rules+0x14b5) → DEFERRED-AI. NOTE: runtime *player-facing* alliance mutation is TriggerAction__Execute-driven (map triggers), ACTIVE, belonging in the lifecycle sub-program seam, not DEFERRED-AI (E2). |
| AI house production tracking fields (Owner +0x564C/50/54/58 cleared by Abandon) | Bookkeeping consumed only by AI choosers. |
| Superweapon manage/resume (AI_ManageProduction/AI_ResumeProduction, +0x1FC tail) | Belongs to the superweapon sub-system, NOT this production substrate; do not wire SW manage/resume into the production queue. |

### TS-LEGACY / DORMANT — do NOT implement as default

| Item | One-line rationale |
|---|---|
| Tunnel / subterranean | TS legacy; absent from every production/house/economy path (verified clean). |
| Fog-of-war "previously-seen" darkening (`*Scenario&0x1000`) | OFF by stock-YR default; not a production field. |
| Shroud-regrow gate (Rules+0x17F0) | OFF by stock-YR default. |
| +0x1D8 editor ally mask | Touched only under map-editor mode; the gameplay ally mask is +0x5788. |
| Production_Timer_pad (+0x30) | Dead scratch field; never read meaningfully. |
| CompletedProduction `SpecialItem==0` vs IsComplete `!=−1` convention | **NOT confirmed dead** (v2): the SpecialItem (+0x68) writer was not located, so value 0 cannot be proven unreachable — do NOT collapse 0 and −1 in the Rust port; keep both as distinct (V2, UNCHECKED writer). |

---

## 4. Comparison against the current Rust architecture

The current production/economy code is a set of **owner-keyed `BTreeMap`s manipulated by module-level free functions** (digest G). There is no per-(house,category) authority object, no global factory registry, no per-step charge, no central revalidation. The producing building is decoupled from its queue; "which factory" is answered only by `active_producer_by_owner` (a `u64` used for sidebar focus + spawn rotation), not by a binding. This is the opposite of both gamemd's per-factory authority **and** the project's own substrate pattern (`src/sim/world/techno_ai.rs` Structure arm — line 107 no-op — is the intended home).

### 4.1 Structural map

| Concern | gamemd authority (catalog) | Current Rust | Verdict |
|---|---|---|---|
| Production state machine | `FactoryClass` per (house, category), `g_FactoryClass_Array` registry | loose `queues_by_owner: BTreeMap<owner, BTreeMap<category, VecDeque>>` + free fns (`production_types.rs:198`) | **DRIFT (structural)** — no Factory object, no registry |
| Per-step progress | `Production_Value` 0→54, step at timer expiry (F2) | `remaining_base_frames` + `progress_carry` PPM integration (`production_queue.rs:881`) | **DRIFT** — different timer model; equivalence unproven |
| Wallet | `HouseClass.Balance` +0x30C, ×100 scale (H1) | `HouseState.credits: i32` (`house_state.rs:28`) | OK (clean Rust analog); scale convention is a DRIFT to confirm |
| Factory↔building binding | factory pointer per category in house | `active_producer_by_owner` u64 (sidebar focus only) | **DRIFT** — binding is cosmetic, not authoritative |
| Multi-factory speed | read from `GetFactoryCount` (H5) | full-store rescan each tick (`production_tech.rs:518`) | OK on output (formula matches per V4), DRIFT on structure/scale |

### 4.2 Behavior table (default DRIFT; from digests C/E/F/G)

| # | gamemd behavior (verified) | Current Rust | Verdict | Player-visible? | Trigger frequency |
|---|---|---|---|---|---|
| 1 | **Pay-as-you-go**: full cost charged incrementally over 54 steps; insufficient funds → `OnHold`, rewind 1 step (no net advance) (F4, digest C/B3) | full cost deducted upfront at enqueue (`production_queue.rs:~218`) | **DRIFT.** VERIFIED-LIVE v2 (V1, single remainder charge — see C3 precision note; G1 confirms upfront charge at `production_queue.rs:218`) | YES — a build halts mid-progress when credits run out; Rust never halts because it pre-paid | every match with tight economy / multiple concurrent builds |
| 2 | **Cancel refund = already-paid only** (`GetCost − Balance`) (F9, digest C/B10) | full-cost refund on cancel | **DRIFT (paired with #1).** VERIFIED-LIVE v2 (refund=`GetCost(Owner)−Balance` via Add_Credits 0x004F9950, V2; Rust full-cost refund at 783/837/876, G1) | YES — refund amount differs the moment #1 is fixed; double-refund risk if fixed alone | every cancel |
| 3 | **Cancel-one removes FIRST matching queued type** (RemoveFromQueue front-to-back, digest C/D3) | `cancel_by_type_for_owner` removes LAST (`.rev()`, `production_queue.rs:811`) | **DRIFT.** VERIFIED-LIVE v2 (`decompile 0x004CA620`, V2 — RemoveFromQueue Find vtable+0x10 returns first match; Rust `.rev()`/last is wrong; G1 confirms line 811 `.rev()`) | YES — which instance leaves, and resulting order, differ when ≥2 of a type queued | any queue with duplicates + cancel |
| 4 | **Factory-complete ≠ delivery**: completed object stays attached + suspended; delivery is a separate command (0x0B); queue does not advance until ExitObject succeeds (F7, digest C/B6) | spawn/pop in the same tick pass; only blocked-vehicle case holds (`production_queue.rs:~551`) | **DRIFT.** VERIFIED-LIVE v2 (CompletedProduction 0x004CA1A0 only clears, no begin/next call; queue advance in FUN_004FAA10, V2/V5) | YES — "unit ready, awaiting bay" cadence; placement-ghost timing | every vehicle/building completion |
| 5 | **Queue restart is command-bound** (StartNextQueued inside successful Place_Production, digest C/B9) | pop-front then next becomes front next tick | **DRIFT (cadence).** VERIFIED-LIVE v2 (StartNextQueued front-pop + re-Begin_Production synchronous inside the cancel/delivery command, V2) | borderline — both deterministic; shifts when next item's money is charged (that IS sim state) | every multi-item queue completion |
| 6 | **Global factory step BEFORE house tick**, two sequential loops, late frame counter (G2/G4, digest D, V3) | production in Phase 7 (post-combat); AI/defeat split into Phase 8/8.5 | **DRIFT (ordering).** VERIFIED-LIVE v2 (PerTickUpdate factory loop 0x55b66a-b68b before house loop 0x55b68d-b6b3; house loop null-checks each slot, factory loop does not, V3) | borderline — observable on same-frame completion + same-frame credit/power interaction | every tick with ≥1 completion |
| 7 | **Prereq revalidation 3-way** (drop permanently-unbuildable / suspend temporarily-unbuildable / resume) on building add/remove/power (digest C/B11, §2c) | UNCHECKED in `production_queue.rs`; lives in `production_tech` or absent | **MISSING / UNCHECKED** | YES — queue reaction when a prereq building is sold/destroyed/captured/powered-down | every base under attack / power loss |
| 8 | **Low-power slowdown** continuous: `÷ clamp(1−(1−ratio)×LPPM, Min, Max)` (H4, V4/R1) | `production_tech.rs:457-473` integer port (445-456 is doc comment) | **OK** (V4 confirms Rust matches; POWER_SYSTEM doc is the stale one). Note Max clamp is ratio<1-conditional, divisor floored 0.01 (R1) | YES | every low-power state |
| 9 | **Multiple-factory speedup** loop `trunc(acc×MF)` per extra factory, per-iteration truncation (H5, R1) | `apply_multiple_factory_scaling_ppm` (def `production_tech.rs:429-443`, call site 407) | **OK** on output; rescan structure is a scale DRIFT; per-iteration vs single-truncate DRIFT to verify (C11/R1) | YES | every multi-factory base |
| 10 | **Purifier bonus base = OrePurifier building COUNT (house+0x538C) × PurifierBonus(Rules+0xf3c) × amount** (+AIVirtualPurifiers[house+0x184] for AI); bonus then run through Add_Tiberium_Credits (×TibValue×IncomeMult) (R4/G1) | bonus = purifier building count × pct (`miner_dock_sequence.rs:1162-1166`, dup `slave_miner.rs:338-342`) | **OK on base term** (count model CORRECT; REFUTES prior C14 — v1 said StorageCapacity bales, v2 REFUTED → building count). Residual DRIFT (LOW): bonus must be converted via TibValue×IncomeMult, not added as raw credits; AIVirtualPurifiers term missing | YES (residual) | every ore deposit |
| 11 | **IncomeMult applied** (`TibValue × IncomeMult × amount`, IncomeMult from country HouseTypeClass+0x148, H2, digest E/A1) (applies to BOTH the base deposit AND the purifier bonus — two Add_Tiberium_Credits calls) | deposit uses raw bale value, no IncomeMult; no country/type handle in deposit path | **DRIFT (LOW).** VERIFIED-LIVE v2 (`disassemble 0x004F9610`, `read_memory 0x00511cfb`, R4) | only with modded IncomeMult (stock = 1.0 → identical) | rare (stock parity holds) |
| 12 | **MissionCom/Radio exit handshake** at war-factory exit (establish radio **0x02**/+0x18/0x09; 0x0C is the building-online handshake, not the exit unlink) | `tick_war_factory_exit_contacts` breaks contact when vehicle clears footprint (`war_factory_exit.rs:28`) | **OK on behavior**, but the code label is wrong: the Rust comment `war_factory_exit.rs:67` ('models 0x08 → 0x19 → 0x03') and the doc's '0x0C' both mislabel — actual establish is 0x02. The break code that fires on footprint-clear was not isolated (UNCHECKED). | YES | every vehicle exit |
| 13 | **Defeat lifecycle** (MPlayer_Defeated effects: scatter, reveal, sidebar collapse, destroy owned **units only** (g_UnitClass_Array) under SpecialFlags 0x800 + GameMode!=0, borrowed-time when GameMode∉{0,5}) (H9, digest F, E2) | `check_defeat` sets bool + has_won; no effects | **DRIFT.** VERIFIED-LIVE v2 (units-only destroy + ordered effects, E2) | YES — match-end scatter/reveal/EVA | every defeat |
| 14 | **Directional alliance bitmask** mutated at runtime by MakeAlly/BreakAlliance (H10, digest F) | static symmetric `HouseAllianceMap` from map `[Houses]` | **DRIFT.** DRIFT confirmed VERIFIED-LIVE v2 (Rust static symmetric HouseAllianceMap cannot represent directional or runtime-mutated alliances; E2) | YES — runtime ally toggle, asymmetric alliance, build-off-ally | allied-start every allied game (Post_Map_Init); runtime change via ally/unally trigger actions or AI Recalculate_Alliances |
| 15 | **Occupied-reactor power zeroing**: gamemd zeroes a garrisoned power plant's output **even after blackout expiry** (`local_d` in AI_AssessPower, digest E/D8) | Rust only zeroes while `power_blackout_remaining>0` | **DRIFT** | YES (Yuri occupy-reactor) | uncommon but every Yuri-occupied-plant match |
| 16 | **Spawn-cell lookup is read-only** | `find_spawn_selection…` silently writes `active_producer_by_owner` during a FIND (`production_spawn.rs:111-117`) | **DRIFT (determinism).** VERIFIED-LIVE v2 (silent write at `production_spawn.rs:111-117` into hashed active_producer_by_owner, G1) | not directly visible but hash-relevant | every spawn with unset active producer |

### 4.3 What is MISSING outright

- **No `FactoryClass` analog and no `g_FactoryClass_Array` analog** — production has no per-(house,category) authority object and no deterministic registry. (digest G §2)
- **No per-step charge + OnHold rewind** (#1). The single largest gameplay-visible economy gap.
- **No factory-complete vs delivery split** (#4) — no "produced-but-not-delivered" state tied to a producing structure.
- **No central prereq revalidation** triggered by building lifecycle/power (#7).
- **No HarvestedCredits accumulator** — gamemd writes `HarvestedCredits += trunc(amount×5.0)` on every deposit (statistics, serialized/score state); Rust has none (digest E/D4). MISSING.
- **No runtime diplomacy** and **no MPlayer_Defeated effects** (#13/#14) — out of this substrate's core scope (house lifecycle) but flagged for the seam.
- **Hashed-state-fabricating getter**: `credits_entry_for_owner` (`production_queue.rs:74-92`) auto-creates a house with `is_human=true` on a miss, mutating the hashed `houses` map from a *getter* (digest G smell #4). Any stray call perturbs `state_hash` — a live hash-determinism hazard, not just a cleanup target. The substrate must forbid house creation in the hot path.

### 4.4 Hash holes (digest G §4) — surfaced, default DRIFT, not triaged

`airfield_docks`, `slave_bindings`, `depot_dock_reservations` (`production_types.rs:237/213/235`), house `country` (`house_state.rs:24`), and `waypoint_edge` (`house_state.rs:48`) are serialized but **absent from `state_hash`** (`src/sim/world/world_hash.rs`; sub-funcs: hash_houses 157-184, hash_production 187-271, hash_power_states 274-282). **Each is UNCHECKED for derive-vs-store equivalence across a load boundary; the default verdict is desync-hole until proven derived** — not "either derived or static" (that phrasing was a soft downgrade). `slave_bindings` (master↔slave binding) and `country` (identity) are gameplay/identity state, not obviously derivable. **`is_low_power` is excluded from the hash but is a pure derivation of the hashed total_output/total_drain (`power_system.rs:127` `is_low_power = produced < drained`) — DERIVED, not a hole** (v1 listed it as a desync hole; v2 downgraded → derived, G1). The remaining five — airfield_docks, slave_bindings, depot_dock_reservations, country, waypoint_edge — stay default-DRIFT. Confirmed in-tree this session: `country` and `waypoint_edge` are real serialized HouseState fields absent from the §4 hashed list. The new substrate must close these (see §6.6, §8).

### 4.5 Hardcoded economy constants (digest G smell #9) — DRIFT, surfaced not triaged

`REPAIR_HP_PER_TICK=4`, `REPAIR_COST_PERCENT=25`, `SELL_REFUND_PERCENT=50` are hardcoded where gamemd reads INI / per-side fields. §7 does not list repair/sell; these are player-visible (repair speed, sell refund). **DRIFT** — deferred only if the user signs off; per the parity bar these cannot be triaged out silently. Surfaced here explicitly.

### 4.6 HouseState field comparison (E1) — house-creation fields missing/drifting in Rust

| gamemd field (offset) | Rust HouseState | Verdict |
|---|---|---|
| PlayerControl (+0x1ED) | absent | **MISSING** — local-viewport / g_PlayerPtr-owner flag, distinct from IsHuman (+0x1EC) |
| HouseIndex (+0x30) | Rust keys by `InternedId` | **DRIFT** — the registration-order index used for `1<<HouseIndex` Allies-bit positioning is absent; needed for directional alliance under the keyed replacement |
| IQ (+0x1D0 AND +0x24C) | absent | **MISSING** — read twice, clamped vs Rules+0x1434; DEFERRED-AI input but seeded at creation |
| credits (+0x1DC vs +0x30C) | single `credits` field | **DRIFT** — Rust collapses raw(+0x1DC) and Balance(+0x30C); campaign starting-credit difficulty bonus + ≥0 clamp + ×100 scale unmodeled (fires **every campaign mission**) |
| waypoint_edge | `waypoint_edge` (`house_state.rs:48`) | **mechanism-DRIFT** — gamemd campaign authoritative source is INI `Edge=` (+0x1E0), not a closest-edge computation |

---

## 5. gamemd-native behavior contract (testable statements)

Each is a TESTABLE invariant the substrate must satisfy, with catalog/digest evidence. These are the acceptance-test targets of §8.

**C1 — Ordering: all factories step before any house tick.** In one logic pass, every factory's progress advances before any house's per-frame update runs; the two are sequential global loops, factories first. *(G2, V3: PerTickUpdate factory loop `0x55b66a` strictly precedes house loop `0x55b68d`.)*

**C2 — 54-step completion.** A build completes at exactly 54 progress steps (`Production_Value == 0x36`), advancing by step=1 per timer expiry. *(F2, V1.)*

**C3 — Per-step credit charge.** Each step charges `⌊Balance/(54−Value)⌋` (signed-trunc); on the final step (`54−Value==0`) the IDIV is skipped (divide-by-zero guard) and the charge is the **entire remaining Balance, charged once**. The completion-block `Spend_Money` (F5) then runs as `Spend_Money(0)` because the pay branch already zeroed Balance — the remainder is charged **once, not twice** (a Rust port that charges remainder at step 54 AND at completion would double-spend). The house pays exactly the original full cost across the build. VERIFIED-LIVE v2 (V1). *(F4/F5, V1.)*

**C4 — Rollback on no-funds.** If the house cannot afford a step's charge, the factory sets `OnHold` and rewinds progress by 1 (net-zero advance, no money spent that step). *(F4, digest C/B3, V1.)*

**C5 — SetRate = total / 54, truncated, clamped.** Per-step frame rate = `GetBuildStepTime() / 54`, signed-truncated, clamped to `[1, 255]`. A factory with no Object yields rate 0 (`(Object?GetBuildStepTime():0)/54`). *(F3, digest A §5: magic `0x4BDA12F7 + SAR 4 = ÷54`.)* No 661 constant exists; 661 is one example total (`rate 12 × 54`). ÷54 magic re-confirmed: `MOV EAX,0x4BDA12F7; IMUL; SAR EDX,4; EDX+=(EDX>>31)` then clamp[1,255] (R1). Every truncation in the pipeline rounds toward zero — FPU control word @0x00822d80 = 0x0E7F, RC=0b11 (truncate); for non-negative costs = floor; no special negative/max-cost behavior (R1). VERIFIED-LIVE v2.

**C6 — FIFO queue.** Queue is FIFO; `StartNextQueued` pops the front; `RemoveFromQueue` (cancel-one) removes the **first** matching type front-to-back. *(F6, digest C/B9/D3.)* VERIFIED-LIVE v2: StartNextQueued 0x004CA5A0 front-pop+shift; RemoveFromQueue 0x004CA620 Find vtable+0x10 first-match; StartProduction 0x004C9C70 appends at tail (FIFO enqueue). (V2)

**C7 — Restart after delivery.** A queued item starts only after the current object is delivered (queue advance is bound to the successful delivery command, not the completion tick). *(digest C/B8/B9.)* VERIFIED-LIVE v2: CompletedProduction 0x004CA1A0 has no begin/next call; queue advance is FUN_004FAA10's post-AbandonProduction StartNextQueued — same path fires on delivery AND on cancel-with-remaining-queue (V2/V5).

**C8 — Partial refund on cancel.** Cancel refunds only the amount already paid (`GetCost − Balance`), not the full cost. *(F9, digest C/B10.)*

**C9 — Prereq revalidation, 3-way.** On a building add/remove/power flip for a house: permanently-unbuildable queued items are dropped (walked back-to-front); a temporarily-unbuildable active item is suspended; a now-buildable suspended item resumes. *(digest C/B11.)* Correction: a **permanently**-unbuildable active item is **AbandonProduction + StartNextQueued** (not suspended); queued items are only ever dropped (permanent), never individually suspended; resume applies only to `IsSuspended && !IsManual`; an empty factory (no Object, no queue) **self-deletes** via `vtable+0x20` (0x004CA770), nulling the Primary_For* slot. VERIFIED-LIVE v2 (`disassemble 0x00509140`, `read_memory 0x007E88F0`; V4).

**C10 — Low-power slowdown (continuous).** Build-step time divides by `clamp(1 − (1 − ratio) × LowPowerPenaltyModifier, Min, Max)`, where the **Max clamp applies only when `ratio < 1.0`**, and the divisor is floored to **0.01** if it computes ≤0. `ratio` from GetPowerRatio 0x004FCE30 (`output/drain`; 0.0 if output==0&low; 1.0 if output≥drain). VERIFIED-LIVE v2 (`disassemble 0x006F47A0`, R1). *(H3/H4, V4.)*

**C11 — Multiple-factory speedup.** Step time runs a **loop** multiplying by `Rules+0x57c` (MultipleFactory) with **truncation after each iteration** (n−1 `trunc(acc×MF)` steps), NOT `step × MF^(n−1)` with a single truncate. Loop gated `Rules+0x57c > 0.0` strictly (skips on ≤0 or NaN). The Rust `apply_multiple_factory_scaling_ppm` (`production_tech.rs:429-443`) must truncate each step, not once — flag as a DRIFT to verify. VERIFIED-LIVE v2 (R1). *(H5, V4.)*

**C12 — Factory-complete vs delivery split.** Completion sets the factory suspended with the object still attached (`Balance = 0`); the object remains pending until a separate delivery succeeds; only then does `CompletedProduction` clear it and the queue advance. *(F5/F7, digest C/B6/B7.)* VERIFIED-LIVE v2 (`decompile 0x004CA1A0/0x004FB0E0/0x004FAA10/0x004CA5A0`; object held +0x58 from create through factory-complete until delivery clears it; V5).

**C13a — Building-online handshake (radio 0x0C).** On successful building placement (Unlimbo), Place_Production sends radio `0x0C` to the just-placed **building** (Receive_Radio 0x0043c2d0 case 0xc → set mission 5/Guard + grand-opening anim). NOT a vehicle footprint-clear unlink. **C13b — War-factory exit radio link.** The vehicle-exit path (ExitObject_Main 0x00443c60) has the producing building establish a radio link to the exiting vehicle via vtable+0x278(2,…)(+0x18/0x09), broken later when the vehicle clears the footprint — the mechanism `war_factory_exit.rs` models. Codes are **0x02/0x18/0x09**, NOT 0x0C and NOT '0x08→0x19→0x03'. The break code was not isolated (UNCHECKED). (v1 framed C13 as a single Radio-0x0C exit-unlink; v2 REFUTED → 0x0C is building-online, exit establish is 0x02.) VERIFIED-LIVE v2 (`disasm 0x004fb2a9`, `decompile 0x0043c2d0`, `decompile 0x00443c60`; V5).

**C14 — Purifier bonus base [CORRECTED v2].** Ore-deposit bonus base = **OrePurifier building COUNT** at house+0x538C (±1 per OrePurifier building via OnConstructionComplete 0x00445F80 / Limbo 0x00445880, gated on BuildingTypeClass+0x16cc = INI 'OrePurifier'), NOT silo StorageCapacity in bales. bonus = count × PurifierBonus(Rules+0xf3c) × amount, +AIVirtualPurifiers[house+0x184] for AI houses. **The current Rust purifier-COUNT model is CORRECT on this term; the prior 'StorageCapacity (bales)' claim is REFUTED** (v1 said StorageCapacity bales; v2 REFUTED → building count). VERIFIED-LIVE v2 (`decompile 0x00522D50/0x00445F80/0x00445880`; `read_memory 0x004604d8`; R4/G1).

**C15 — Exact-cost conservation.** Over a full build with no cancel, total credits removed from the house equal the type's full `GetCost`; with a mid-build cancel, total removed equals the spent portion and the refund returns it. *(F5/F9, derived from C3+C8.)* Proven by telescoping: Σ per-step `cost_k` = B₀ − B_final; completion adds B_final → total = B₀ = full GetCost, independent of per-step rounding. The remainder is charged exactly once (final pay step); completion `Spend_Money` is `Spend_Money(0)`. Conservation exact. VERIFIED-LIVE v2 (V1).

**C16 — No cap on ore→credit conversion.** Ore deposited beyond silo storage capacity still converts fully to credits (no overflow loss on the main path); only the WEED path caps weed storage at `TiberiumStorageLimit`. *(digest E A5/E; `DepositOreFromStorage 0x00522D50` drains the whole slot unconditionally.)* — guards against a Rust author adding a wrong storage cap to the main deposit path.

**C17 — HarvestedCredits statistics accumulator.** Every ore deposit also accumulates `HarvestedCredits += trunc(amount × 5.0)` (statistics / score state; independent of the credit award). *(H2, digest E A2/D4; Rust currently has NO such accumulator — see §4.3 MISSING.)* VERIFIED-LIVE v2: const 5.0 @0x007eaa00, `[house+0x54E8]=ftol(amount×5.0+…)` (R4).

**C18 — IncomeMult source.** Deposit credit = `trunc(TibValue × IncomeMult × amount)`, with IncomeMult read from the **house's country type** (HouseTypeClass+0x148; stock 1.0). The deposit path must carry a country/type handle to evaluate it. *(H2, digest E A1/A2.)* VERIFIED-LIVE v2: IncomeMult @ HouseTypeClass+0x148 (`disassemble 0x004F9610`, `read_memory 0x00511cfb`, R4); applies to base AND purifier-bonus credits.

**C19 — Prereq gate, three argument triples.** The type prereq gate (`type->vtable+0x94`, ECX=type, args `(aiMode, countExisting, expend, house)`, routing into HouseClass::CanBuild 0x004F7870) is called with three distinct triples. Begin_Production: `(0,1,1,house)` ('buildable now'), retried `(1,0,1,house)` only when the resume flag is set, else returns 3. **Revalidation (0x00509140) uses a DIFFERENT pair**: queued items tested once with `(1,0,1,house)` and **dropped if 0** (permanent); the active object tested `(1,0,1)` → **AbandonProduction + StartNextQueued if 0** (permanent), else `(1,1,1)` → **Suspend if 0** (temporary), else **SetRate-resume if `IsSuspended && !IsManual`**. The 3-state `BuildEligibility::{Buildable, TemporarilyBlocked, PermanentlyBlocked}` maps: Buildable = (1,0,1) pass AND (1,1,1) pass; TemporarilyBlocked = (1,0,1) pass BUT (1,1,1) fail; PermanentlyBlocked = (1,0,1) fail. (v1 said revalidation mirrors Begin_Production's pair; v2 REFUTED → revalidation uses (1,0,1)/(1,1,1), a different pair.) VERIFIED-LIVE v2 (`disassemble 0x004FA350` 0x4fa438/0x4fa45b; `disassemble 0x00509140` 0x5091bf/0x509210/0x509240; `decompile 0x004F7870`; V4).

**C20 — Begin-vs-enqueue decision + sidebar tab switch.** When the queue is non-empty AND not-resume AND not-already-producing-this type, Begin_Production switches the sidebar tab and returns 0 (queued, no start) rather than starting; otherwise it starts a fresh object. *(digest C/B1; observable cadence — the "click adds to queue vs starts" decision and the tab switch.)*

---

## 6. Rust-native replacement boundary

A cohesive **production+economy substrate** that mirrors gamemd authority with clean Rust. It owns three things the current code scatters: the **wallet** (per house), the **factory state machines** (per house, per category, in a deterministic registry), and the **per-tick step service**. It exposes read-only views to the sidebar (render layer) and a clean, empty seam for AI (deferred).

### 6.1 Ownership / module diagram

```
Simulation (src/sim/world/mod.rs)
├── houses: BTreeMap<InternedId, HouseState>        // wallet lives here (credits), unchanged container
│      └── HouseState.economy: Economy              // NEW: wallet+storage value-type (shadow→authoritative)
│
├── substrate: ObjectSubstrate                       // existing: entities, logic order, occupancy
│      └── techno_ai.rs Structure arm  ──────────────┐  (seam: per-object BuildingClass::Update bracket)
│                                                     │
└── production: ProductionState                       │
       └── factories: FactoryRegistry  ◄──────────────┘  // NEW authority — analog of g_FactoryClass_Array
              │  BTreeMap<(InternedId house, ProductionCategory), Factory>
              │  + deterministic registration order (insertion_seq) for same-frame completion order
              │
              ├── Factory (value-type)                    // analog of FactoryClass (0x74)
              │     progress 0..=54, step_rate_frames, step_timer,
              │     balance, original_balance, object: Option<PendingObject>,
              │     on_hold, suspended, manual, queue: VecDeque<InternedId>
              │
              └── (existing economy buckets stay: resource_nodes, ore_growth, docks, terrain… )

         ┌─────────────── per-tick step service (NEW) ───────────────┐
         │ ProductionSubstrate::step_factories(&mut sim)             │
         │   for (key, factory) in factories  (registry order)       │
         │     factory.advance_one_step(&mut economy_of(house), rules)│  ← C2/C3/C4/C5/C12
         └────────────────────────────────────────────────────────────┘

  Render seam (NO sim dep):  sidebar reads FactoryView { progress, on_hold, queue, ready } (read-only)
  AI seam (deferred):        FactoryRegistry::begin_for_ai(house, cat, type) — empty trait-free hook,
                             never called by human path; documented "S?: AI build chooser"
```

**Determinism:** `FactoryRegistry` is keyed by `(InternedId, ProductionCategory)` in a `BTreeMap` → sorted iteration (replay/lockstep). For **same-frame completion order** (the gamemd `g_FactoryClass_Array` insertion order, digest D §E UNCHECKED), the registry carries a monotonic `insertion_seq: u64` per factory and the step service iterates by `(insertion_seq)` — reproducing native array order, not map order. **30-player scale:** `(InternedId, …)` keying has no 8-player array; no fixed bitmask. The alliance/defeat lifecycle (digest F) keeps using `InternedId`-keyed structures, never `1<<(idx&0x1f)`.

**`insertion_seq` serde discipline:** the registry's `next_insertion_seq` counter MUST itself be serialized and hashed — otherwise two peers can assign different seqs to factories created after a load → desync at scale. It is therefore part of the §6.4 hashed/serialized field set (P5 test `registry_next_insertion_seq_is_serialized_and_hashed`).

### 6.2 Key types (Rust-native, fixed-point, no addresses in comments)

```rust
//! Production+economy substrate: per-house wallet/storage and per-(house,category)
//! factory state machines, stepped once per tick before the house tail. Mirrors
//! the engine's production authority with clean Rust. Depends only on rules data
//! and the entity store; never on render/ui/sidebar/audio/net.

/// 54 progress steps to completion (the engine's build-step count).
pub const PRODUCTION_STEPS: u16 = 54;
/// Per-step frame-rate clamp.
pub const STEP_RATE_MIN: u16 = 1;
pub const STEP_RATE_MAX: u16 = 255;

/// Per-house wallet + ore storage. Credits are the spendable balance; the engine
/// keeps an internal x100 scale — we keep the same scale so build/deposit math is
/// bit-identical to the source. The purifier deposit bonus base is the per-house
/// OrePurifier building count, not silo capacity (REFUTES prior C14). IncomeMult is
/// NOT stored here — it is read per-deposit from the depositing house's country
/// type (stock 1.0).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Economy {
    pub credits: i32,            // spendable balance (x100 internal scale)
    pub spent_credits: i32,      // running total spent (statistics-relevant)
    pub harvested_credits: i32,  // statistics accumulator (deposit x5.0)
    pub purifier_count: i32,     // OrePurifier building count; purifier-bonus base
}

impl Economy {
    /// Add credits to the balance (deposit, refund, grant).
    pub fn add_credits(&mut self, amount: i32);
    /// Spend up to `amount`; returns the amount actually paid. Drains stored ore
    /// to cover any shortfall (engine silo-drain fallback). Never goes negative.
    pub fn spend(&mut self, amount: i32) -> i32;
    /// Continuous power ratio for the build-speed penalty: output/drain when
    /// output < drain (and drain != 0), else 1.0 — read from PowerState.
    pub fn available(&self) -> i32 { self.credits }
}
```

The deposit path must read IncomeMult from the house's country type (C18) — `Economy` does not store it, so the deposit caller passes the resolved IncomeMult and the per-house `purifier_count` is the purifier-bonus base (C14); both base-removed and bonus convert via TibValue×IncomeMult (C18); ore→credit conversion is uncapped (C16) and accumulates `harvested_credits += trunc(amount×5.0)` (C17).

```rust
/// The object a factory holds from start through delivery. Bound to a produced
/// entity once it exists; before that the type id alone is held in the queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingObject {
    pub type_id: InternedId,
    /// stable id of the produced entity once created (engine attaches the object
    /// at start; we create lazily at first step or at completion — slice choice).
    pub entity_id: Option<u64>,
}

/// One production state machine: per (house, category). Value-type, owned by the
/// FactoryRegistry. Mirrors the engine's FactoryClass behavior contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Factory {
    pub owner: InternedId,
    pub category: ProductionCategory,
    /// 0..=54. Completion at PRODUCTION_STEPS.
    pub progress: u16,
    /// Per-step frame rate = GetBuildStepTime()/54, clamped [1,255].
    pub step_rate_frames: u16,
    /// Frames remaining in the current step (counts down to 0 = advance).
    pub step_timer: u16,
    /// Remaining cost still owed (charged down per step). Full cost at start.
    pub balance: i32,
    /// Full-cost snapshot at start, for exact-cost conservation.
    pub original_balance: i32,
    /// In-flight object (None when idle/queued-only).
    pub object: Option<PendingObject>,
    /// Set when a step could not be afforded (UI "On Hold"); does not advance.
    pub on_hold: bool,
    /// Complete-but-not-delivered, or paused: not stepping.
    pub suspended: bool,
    /// User-vs-system pause distinction.
    pub manual: bool,
    /// FIFO build queue of type ids waiting behind the active object.
    pub queue: VecDeque<InternedId>,
    /// Deterministic registration order — reproduces native array order for
    /// same-frame completion sequencing.
    pub insertion_seq: u64,
}

/// Deterministic registry of all factories. Analog of the engine's global factory
/// array, but keyed (no fixed-size player array) for the 30-player scale target.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactoryRegistry {
    factories: BTreeMap<(InternedId, ProductionCategory), Factory>,
    next_insertion_seq: u64,   // serialized + hashed (see §6.1, §6.4)
}

impl FactoryRegistry {
    /// Step every factory once, in native registration order, charging its owner.
    /// This is the substrate's per-tick service (engine: global FactoryClass loop).
    pub fn step_all(&mut self, economies: &mut BTreeMap<InternedId, Economy>, rules: &Rules);
    /// Begin production / enqueue for a (house, category); lazily creates the
    /// factory. Resume flag mirrors the engine's StartProduction resume path.
    pub fn begin(&mut self, owner: InternedId, category: ProductionCategory, type_id: InternedId);
    /// Cancel-one: remove the FIRST matching queued type (front-to-back), or
    /// abandon the active object with partial refund.
    pub fn cancel_one(&mut self, owner: InternedId, category: ProductionCategory,
                      type_id: InternedId, economy: &mut Economy);
    /// 3-way prerequisite revalidation for one house, called on building
    /// add/remove/power change: drop permanently-unbuildable queued items,
    /// suspend a temporarily-unbuildable active item, resume a now-buildable one.
    pub fn revalidate(&mut self, owner: InternedId, can_build: &dyn Fn(InternedId) -> BuildEligibility);
    /// Read-only sidebar view (render layer): never mutates.
    pub fn view(&self, owner: InternedId, category: ProductionCategory) -> Option<FactoryView<'_>>;
}
```

```rust
impl Factory {
    /// Advance one tick: if armed and the step timer elapsed, take one step —
    /// charge balance/(54-progress) (or the remainder on the last step), and on
    /// no-funds set on_hold + rewind one step. On reaching 54, suspend with the
    /// object attached and balance zeroed (delivery is a separate command).
    fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome { /* C2/C3/C4/C12 */ }
}

/// Build eligibility result for revalidation (engine's 3-way distinction; the
/// active object runs BOTH `(1,0,1)` and `(1,1,1)` gates, queued items only
/// `(1,0,1)`): Buildable = (1,0,1) pass AND (1,1,1) pass; TemporarilyBlocked =
/// (1,0,1) pass BUT (1,1,1) fail -> suspend; PermanentlyBlocked = (1,0,1) fail ->
/// abandon active / drop queued.
pub enum BuildEligibility { Buildable, TemporarilyBlocked, PermanentlyBlocked }

pub enum StepOutcome { Idle, Stepped, Stalled, Completed }
```

### 6.3 Where it sits in `advance_tick`

gamemd order is **global factory step → house tick** with a **late frame counter** (G2/G4). The Rust phase pipeline already runs `power_system::tick_power_states` in Phase 4 (so build speed reads fresh power, matching the contract) and production in Phase 7. The substrate keeps that placement but tightens the internal order:

```
advance_tick:
  … commands → ground move → air/special → vision → power (Phase 4) …
  object_ai_stage()                 // existing substrate spine (no-op arms today)
  Phase 7 (Scatter+Production+…):
    ┌── FactoryRegistry::step_all() ──┐   ← NEW: the global factory step, BEFORE house tail (C1)
    │     (charges economies)         │
    └─────────────────────────────────┘
    tick_resource_economy (harvest)   // deposits feed Economy (C14/C16/C17/C18)
    delivery commit (command-bound)   // C7/C12: queue advances on successful delivery
    repairs, docks, ore growth, terrain spawners
  Phase 8 (AI) — deferred seam
  Phase 8.5 (defeat) → house lifecycle effects (digest F, separate sub-program)
  refresh_mission_shadow → state_hash
```

**FIT correction (do NOT ship the equivalence as fact).** The whole substrate program's invariant is that per-object behavior dispatches through `object_ai_stage()` in **LogicVector** order (`src/sim/world/techno_ai.rs` Structure arm, S8 marker `src/sim/world/techno_ai.rs:107`). Running `step_all` as a standalone Phase-7 registry sweep "until that arm is non-no-op" is exactly the per-owner free-function-scan anti-pattern Digest G #1 flags as architecturally divergent. The justification "insertion_seq order equals the eventual per-building order" is an **UNPROVEN equivalence claim** — the eventual order is LogicVector (reveal/conceal) order, NOT factory-insertion order: a building revealed→concealed→revealed gets a new LogicVector position but keeps its factory `insertion_seq`. Resolution, pick one before P5:
- **(a) Preferred:** drive `step_all` from the **Structure arm of `object_ai_stage()`** (`src/sim/world/techno_ai.rs:162` unit_ai_shadow_step is the shadow counterpart) from P2 onward — each building steps its own factory in live-object order — making the registry a *lookup*, not an iteration owner.
- **(b)** explicitly mark "insertion_seq order == LogicVector order" as an UNPROVEN assumption and add a blocking test `factory_step_order_matches_logic_vector_order` before P5.

The end-state (option a) is the alignment target; until the Structure arm is non-no-op, the registry sweep runs in `insertion_seq` order with the equivalence flagged UNPROVEN, never asserted.

### 6.4 Serialization + hashing

- `Economy`, `Factory`, `FactoryRegistry` all derive `Serialize/Deserialize` and are included in `state_hash` once authoritative. **Hash field set for `Factory`:** owner, category, progress, step_rate_frames, step_timer, balance, original_balance, object (type_id + entity_id), on_hold, suspended, manual, queue (type ids in order), insertion_seq. **For `FactoryRegistry`:** the factory map **and** `next_insertion_seq` (the counter must round-trip and hash — §6.1). **For `Economy`:** credits, spent_credits, harvested_credits, purifier_count.
- The hash addition is **gated behind a `SNAPSHOT_VERSION` bump** (current 17 → 18) at the authority-flip slice, exactly as Mission/Radio did (bump 16→17). Until then the new fields are `#[serde(skip)]` shadow state and excluded from the hash.
- The migration also **closes the existing hash holes** (digest G §4, §4.4): when the registry becomes authoritative, the old `active_producer_by_owner` u64 and the per-item `remaining_base_frames/progress_carry` leave the hash and are replaced by `Factory` fields; `slave_bindings`/`airfield_docks`/`depot_dock_reservations`/`country`/`waypoint_edge` are folded into the hashed set or proven derived (separate verification, tracked in §8).

### 6.5 Shadow-mode rollout (mirrors Mission/Radio)

1. **Shadow:** `FactoryRegistry` built each tick from the existing queues as `#[serde(skip)]` derived state; a read-only debug assert proves the shadow factory's `progress/balance` track the legacy item's `remaining_base_frames`-derived progress and the legacy upfront-charge balance. Divergence is **surfaced** (tick + owner + category), never equalized — the same discipline as `unit_ai_shadow_step` (`src/sim/world/techno_ai.rs:162`).
2. **Invert:** flip the hash-invariant test — assert the registry is the source the legacy view derives from, not vice versa.
3. **Drop shadow asserts.**
4. **Authoritative:** legacy `queues_by_owner`/`active_producer_by_owner` become thin derivations of the registry (or are retired, §7); `state_hash` hashes the registry; **bump `SNAPSHOT_VERSION` 17→18.**
5. **Parity harness:** deterministic replay over a recorded command stream produces a bit-identical per-tick hash sequence vs the baseline (§8).

### 6.6 Seams

- **AI (deferred):** `FactoryRegistry::begin` is the only entry; an AI chooser would call it with a chosen type. No AI internals are designed; the hook is documented and never called by the human path (digest A/E AI-shortcut + headstart precompute are explicitly out of scope, classified DEFERRED-AI in §3).
- **Sidebar (render):** `FactoryView<'_>` is a borrow-only projection (progress %, on_hold, queue contents, ready list). The `IsDifferent`/`HasChanged` dirty flag is **render-only and not hashed** (F11, digest C §E) — it stays in the render layer, computed from a per-tick change set, never in `Factory`.
- **House-lifecycle / diplomacy (deferred sub-program) — binding scale constraint.** **Scale-blocker inventory (VERIFIED-LIVE v2, E2):** (a) **+0x5788 Allies** 32-bit `1<<(idx&0x1f)` directional — hard 32-slot cap, zero headroom for 30 players → replace with `InternedId`-keyed directional set; (b) **+0x1D8** editor self-mask, same 32-cap (DORMANT in skirmish); (c) **+0x5600 EnemyHouseIndex** single grudge index (-1=none) → `Option<InternedId>` (NOT a bitmask); (d) **two per-peer-house cross-registered DynamicVectors** at +0x5604..+0x5618 and +0x561C..+0x5630 (8-byte ptr+score/flag records), bidirectionally registered in the ctor → **O(N²) house-creation cross-registration**, replace with on-demand keyed maps; (e) **5 global type-removal listener subscriptions** (objects/factory/tag/etc.) — global, not per-peer, lower priority. **No separate radar-share or enemies bitmask exists** — enmity = not-allied + single grudge index. Only +0x5788/+0x1D8 are the hard 32-caps. (v1 said "radar-share, grudge, and the 5 listener arrays use the same [bitmask] pattern"; v2 REFUTED → grudge is a single index, the per-peer vectors are DynamicVectors not bitmasks, no radar-share bitmask exists.) **Constraint for the deferred house-lifecycle sub-program: the directional `Allies` mask and ALL `1<<(idx&0x1f)` peer bitmasks MUST be replaced with `InternedId`-keyed sets — a 32-bit bitmask is a hard 30-player blocker, not optional cleanup.** This requirement is recorded here so it binds that future sub-program, not just the digests.
- **Superweapon:** AI_ManageProduction/AI_ResumeProduction (+0x1FC tail) belong to the superweapon sub-system (`superweapon/mod.rs`), NOT this substrate; do not wire SW manage/resume into the production queue.

---

## 7. Old ad hoc Rust logic to retire

Concrete `file:symbol` targets from digest G, each mapped to its substrate replacement. Retire only at/after the authority-flip slice; until then they coexist as the shadow-derived-from source.

| Current Rust (file:symbol) | Smell | Replaced by |
|---|---|---|
| `production_types.rs:198` `queues_by_owner` | loose per-owner map, no factory authority | `FactoryRegistry.factories[(house,cat)].queue` |
| `production_types.rs:201` `active_producer_by_owner` | cosmetic u64 binding, hashed but only sidebar-meaningful | `Factory` is the authoritative binding; view derives the focus id |
| `production_queue.rs:881-898` PPM `progress_carry`/`remaining_base_frames` integration | timer model divergent from 54-step counter | `Factory.progress (0..=54)` + `step_rate_frames`/`step_timer` (C2/C5) |
| `production_queue.rs:~218` upfront `*credits -= obj.cost` | pre-pay (no per-step charge, no stall) | `Factory::advance_one_step` per-step charge + `on_hold` rewind (C3/C4) |
| `production_queue.rs:~783/837/876` full-cost refund | over-refund once charging is per-step | `FactoryRegistry::cancel_one` partial refund `original_balance − balance` (C8) |
| `production_queue.rs:~811` `cancel_by_type_for_owner` `.rev()` | removes LAST matching (wrong end) | front-to-back first-match removal (C6) |
| `production_queue.rs:74-92` `credits_entry_for_owner` (auto-creates `is_human=true` house) | side-effecting getter mutating hashed state (§4.3 hazard) | `Economy` accessor on an explicitly-created house; no fabrication in hot path |
| `production_tech.rs:518-534` `matching_factory_count_for_owner` (full-store rescan) | O(N) rescan per owner/cat/tick at 20k scale | `FactoryRegistry` count by key (output formula unchanged, V4) |
| `production_tech.rs` pervasive `owner: &str` + `eq_ignore_ascii_case` scans | intern/resolve churn | `InternedId`-keyed registry/economy API |
| `production_spawn.rs:111-117` `find_spawn_selection…` silent `active_producer` write | find-fn with hashed side effect (§4.2 #16) | spawn reads `Factory.object.entity_id`; no hidden write |
| `production_tech.rs:48-54,18,139` hardcoded stolen-tech block / `MATCH_TECH_LEVEL=10` / disabled prototype | approximations, not the mechanism | revalidation `can_build` closure reads real prereq/tech gates (C9/C19; CanBuild semantics from digest E §B) |

**Note:** the `Defense` `ProductionCategory` split (`production_types.rs:138`) is a sidebar concern leaking into sim keys (digest G §6). The engine has one building factory; defenses route to the same building factory with a sub-category at the render layer. The registry **keeps `Defense` as a key for now** (changing it is a hash-set change with its own slice) but documents it as a render-origin split to revisit.

---

## 8. Migration slices + acceptance tests

Dependency-ordered, shadow-first, in the Mission/Radio rhythm: **shadow → invert hash-invariant test → drop shadow asserts → make authoritative → bump SNAPSHOT_VERSION → parity harness.** Every test runs as `cargo test -p vera20k` and is deterministic; hash-relevant ones gate on `state_hash()`.

### Slice P0 — research gate (BLOCKING; no Rust)
**Goal:** re-decompile the still-DOC-ONLY math before any charge/deposit code becomes authoritative. **Most P0 targets are now VERIFIED-LIVE v2 and no longer block.** SETTLED: GetBuildStepTime full truncation order (no ×0.9 — REFUTED; per-iteration MultipleFactory truncation; BuildSpeed double in wall branch only; Math__ftol RC=truncate); Primary_For* +0x53AC=Aircraft/+0x53B0=Infantry binding; +0x1DC StartingCredits (campaign ×100 / MP raw). **STILL OPEN (keep BLOCKING for the dependent test only):** AIVirtualPurifiers index-field identity (offset 0x1324 verified, the `house+0x184` AI-difficulty index identity UNVERIFIED). P3 is UNBLOCKED (charge math verified). P7's `ai_virtual_purifier_*` test remains gated on the index-identity check; the count-base and IncomeMult parts are unblocked.
**Targets (residual):** AIVirtualPurifiers index-field identity (digest E LBC #6). **Settled this pass (no longer targets):** `GetBuildStepTime 0x006F47A0` full truncation order / the `×0.9` (REFUTED — does not exist) / MultipleFactory float-compare / BuildSpeed double; `Math__ftol` RC=truncate; SetRate ÷54 magic; the `Primary_For*` +0x53AC/+0x53B0 category binding; +0x1DC StartingCredits. **Output:** verified verdicts folded into §9.1; no slice past P7 ships its AIVirtualPurifiers term on the open index identity.

### Slice P1 — `Economy` value-type (shadow)
**Goal:** introduce `Economy` (wallet + storage + statistics) as `#[serde(skip)]` shadow alongside `HouseState.credits`; prove it tracks the legacy credits exactly.
**Files:** `src/sim/economy.rs` (new), `src/sim/house_state.rs`, `src/sim/production/production_economy.rs`.
**Tests:**
- `economy_shadow_tracks_legacy_credits` — after each deposit/spend, `Economy.credits == HouseState.credits`.
- `economy_shadow_does_not_change_state_hash` — building the shadow leaves `state_hash()` bit-identical (mirrors `techno_ai_shell_is_passthrough_no_hash_change`).
- `economy_spend_silo_drain_matches_engine` — `spend()` with insufficient credits drains stored ore, returns exact paid amount, never negative (C-derived from H1/A4).
- `economy_ore_deposit_has_no_credit_cap` — depositing amount beyond any silo storage limit still converts the full amount to credits (C16).
- `economy_harvested_credits_accumulates_x5` — after deposit of N bales, `Economy.harvested_credits == trunc(N × 5.0)`, independent of the credit award (C17).

### Slice P2 — `Factory` + `FactoryRegistry` (shadow, derived)
**Goal:** build the registry each tick from `queues_by_owner` as derived shadow; assert the shadow factory's progress/balance track the legacy item.
**Files:** `src/sim/production/factory.rs` (new), `src/sim/production/factory_registry.rs` (new), `production_queue.rs`. (If FIT option (a) is taken — §6.3 — the per-building step dispatch is wired into the Structure arm of `object_ai_stage()` from here.)
**Tests:**
- `factory_shadow_progress_tracks_legacy_remaining` — shadow `progress (0..=54)` maps monotonically to legacy `remaining_base_frames`; divergence surfaced (tick+owner+cat), never equalized.
- `factory_registry_iteration_is_insertion_ordered` — `step_all` visits factories in `insertion_seq`, not map order, with a fixture of 3 owners × 2 categories.
- `factory_registry_shadow_no_hash_change`.
- (FIT option b only) `factory_step_order_matches_logic_vector_order` — UNPROVEN-assumption guard before P5.

### Slice P3 — per-step charge + rollback (shadow assert against an oracle; P0 charge-math verified — UNBLOCKED)
**Goal:** implement `advance_one_step` (C2/C3/C4/C5) and assert it conserves exact cost. P3 runs against a **CLONED/throwaway economy (oracle)** — it does NOT call the real `economy.spend` on the hashed wallet; the legacy upfront-charge remains authoritative and the `#[serde(skip)]`/no-hash guarantee holds until P5.
**Files:** `factory.rs`.
**Tests:**
- `factory_54_steps_to_complete` — from start to `Completed`, exactly 54 `Stepped` outcomes (C2).
- `factory_exact_cost_conservation` — sum of oracle-economy spend over a full build == type full cost, for costs {1, 25, 700, 99991} (boundary set, C3/C15).
- `factory_last_step_charges_full_remainder` — at progress 53→54 the charge equals the entire remaining balance, with no division-by-zero at `stepsLeft==0` AND assert the remainder is charged exactly once (no second full-Balance charge at completion — completion is `Spend_Money(0)`; double-charge = bug) (C3/C5; V1 DRIFT-A; digest A §3 guard).
- `factory_stall_on_no_funds_rewinds` — with the oracle economy one below a step charge, the factory sets `on_hold` and `progress` is unchanged after the tick (C4).
- `set_rate_total_over_54_truncates_clamps` — `step_rate_frames == clamp(total/54, 1, 255)` for totals {0, 53, 54, 661, 14000} (C5; 661→12).
- `set_rate_zero_when_no_object` — a factory with no Object yields `step_rate_frames == 0` and a suspended/queued-only factory does not step (C5).
- `factory_advance_step_does_not_change_state_hash` — P3's stepping against the oracle leaves `state_hash()` bit-identical (no-hash guarantee until P5).

### Slice P4 — FIFO queue + cancel semantics (shadow→assert)
**Goal:** registry queue (FIFO front pop) + `cancel_one` first-match + partial refund.
**Files:** `factory_registry.rs`, `factory.rs`.
**Tests:**
- `cancel_one_removes_first_matching` — queue `[A,B,A,C]`, cancel A → `[B,A,C]` (C6, the digest-C/D3 DRIFT fix; current Rust removes the last A).
- `cancel_active_refunds_spent_only` — cancel an active build at progress 20 refunds `original_balance − balance` (the spent portion), and total house credits return to pre-build value (C8/C15).
- `queue_advances_only_after_delivery` — completion suspends with object attached; queue front does not advance until the delivery commit (C7/C12).

### Slice P5 — invert hash-invariant + make authoritative; bump SNAPSHOT_VERSION (+ ordering lock decision)
**Goal:** flip the registry to authoritative; legacy `queues_by_owner`/`active_producer_by_owner` become derived (or retired, §7); hash the registry; **bump 17→18.**
**Ordering note (FIT):** making the registry authoritative bakes the same-frame completion ordering into the hash. The C1 factory-before-house ordering lock (P8) **changes** that ordering, which is hashed state. **Decision required:** either fold the C1 ordering lock into P5 (one combined authority+ordering flip, one version bump), OR accept that P8 needs a **second** `SNAPSHOT_VERSION` bump (18→19). Default recommendation: fold C1 into P5 to avoid two version-affecting changes.
**Files:** `world_hash.rs`, `snapshot.rs`, `production_queue.rs`, `factory_registry.rs`.
**Tests:**
- `production_authoritative_hash_includes_factory_fields` — mutating a `Factory.progress`/`balance`/`queue` changes `state_hash()`; mutating the (now-derived) legacy mirror does not (the inversion test).
- `snapshot_version_is_18` and `snapshot_roundtrip_factory_registry` — serialize→deserialize→`state_hash()` equal; assert `SNAPSHOT_VERSION == 18`.
- `legacy_active_producer_removed_from_hash` — the retired field no longer perturbs the hash.
- `legacy_progress_carry_removed_from_hash` — mutating the retired `remaining_base_frames`/`progress_carry` PPM-carry field does not perturb `state_hash` after the flip.
- `registry_next_insertion_seq_is_serialized_and_hashed` — `next_insertion_seq` round-trips through serde and is part of the hash (§6.1 desync guard).

### Slice P6 — prereq revalidation 3-way (authoritative)
**Goal:** wire `FactoryRegistry::revalidate` into building add/remove/power transitions; implement drop/suspend/resume (C9), with the 3-state eligibility mapping the two prereq-gate forms (C19).
**Files:** `factory_registry.rs`, building lifecycle (`production_sell.rs`/place path), `power_system.rs` transition hook.
**Tests:**
- `revalidate_drops_permanently_unbuildable_queued` — queue an item, remove its prereq permanently → item dropped from queue.
- `revalidate_drops_all_permanently_unbuildable_back_to_front` — queue with two unbuildable items → both removed, survivors keep order (C9 back-to-front walk).
- `revalidate_suspends_then_resumes_active_on_power` — active build, power lost making it temporarily blocked → `suspended`; power restored → resumes (C9/C19).
- `revalidate_abandons_active_when_permanently_unbuildable` — active build whose prereq is permanently removed (fails `(1,0,1)`) → AbandonProduction (partial refund) + StartNextQueued, NOT suspend (C9/C19, V4).
- `revalidate_no_hash_change_when_nothing_blocked` — a revalidate pass with all-buildable leaves the hash unchanged.

### Slice P7 — purifier-bonus economy fix (authoritative, economy; count-base + IncomeMult unblocked; AIVirtualPurifiers index identity still gated)
**Goal:** (a) **keep** the count-based base (already CORRECT per R4/G1 — REFUTES prior C14, NOT a HIGH-magnitude fix); (b) add the AIVirtualPurifiers[house+0x184] term for AI houses; (c) route the purifier bonus credit through TibValue×IncomeMult (the residual DRIFT — currently added raw); (d) add HarvestedCredits += trunc(amount×5.0). Apply IncomeMult from country type (C18) to both base and bonus.
**Files:** `economy.rs`, `production_economy.rs`, miner deposit path (`miner_dock_sequence.rs` + dup `slave_miner.rs:338-342`).
**Tests:**
- `deposit_bonus_uses_purifier_count` — bonus == count × PurifierBonus × N, independent of silo storage (C14).
- `deposit_bonus_zero_when_no_purifier` — no OrePurifier building → no bonus.
- `purifier_bonus_runs_through_income_mult_and_tibvalue` — bonus credit == trunc(count × PurifierBonus × TibValue×IncomeMult × N) (residual DRIFT fix; C18).
- `ai_virtual_purifier_adds_to_count_base` — AI house adds AIVirtualPurifiers[difficulty] to the *count* base (digest E/D2), human house does not — **gated on the open AIVirtualPurifiers index-field identity check (P0 residual)**.
- `deposit_applies_income_mult_from_country_type` — deposit credit == `trunc(TibValue × IncomeMult × amount)` with IncomeMult resolved from the depositing house's country type; stock 1.0 yields the raw value (C18).

### Slice P8 — ordering: factory step before house tail (authoritative; may fold into P5 per §8 P5 note)
**Goal:** assert and lock the C1 ordering — `step_all` runs before the house tick within the pass; same-frame completions charge in `insertion_seq` order.
**Files:** `world/mod.rs` (Phase 7 internal order), an ordering test module.
**Tests:**
- `factory_step_precedes_house_tail` — instrument a per-pass sequence counter (debug-only, like `ShellTrace`); assert every factory's step ordinal < the house-tail ordinal (C1).
- `same_frame_completions_charge_in_insertion_order` — two factories completing the same tick deduct credits in `insertion_seq` order; reordering the registry map (different `InternedId`s) does not change the deduction order.
- `completed_factory_holds_object_across_ticks_until_delivery` — a factory reaching 54 stays `suspended` with `object.is_some()` for ≥2 ticks with no delivery command, and the queue front is unchanged (C12 × C1 interaction).

### Slice P9 — global parity / replay harness (acceptance)
**Goal:** the required end-to-end determinism gate — a recorded command stream (begin, suspend, cancel-one, cancel-all, place) replayed twice and against the pre-migration baseline yields a bit-identical per-tick `state_hash()` sequence.
**Files:** `src/sim/production/factory_parity_tests.rs` (new), reusing the existing replay harness from Slice 8 T6 (the global parity harness in the recent commits).
**Tests:**
- `production_replay_is_bit_identical` — fixed seed + scripted command stream over ~600 ticks; `run()` twice → identical `Vec<hash>` (deterministic replay).
- `production_parity_vs_baseline_hash` — the recorded baseline hash sequence (captured at the authority-flip commit) equals the live sequence; a regression in any of C1–C20 flips a tick hash and fails this test.
- `economy_conservation_over_replay` — at the end of the scripted stream, total credits granted − total spent + total refunded == final balances summed across houses (C15, global invariant).
- `vehicle_exit_breaks_radio_contact_in_replay` — over the scripted stream, every delivered vehicle's factory contact is broken once it clears the footprint, deterministically (C13).

*Defeat/diplomacy effects and AI build choosers are explicit seams, not designed here (the house-lifecycle sub-program and DEFERRED-AI respectively).*

---

## 9. Sources & Verification Ledger

### 9.1 Ghidra addresses verified LIVE (v1 prior session + v2 this pass)

**v1 prior-session live (retained):**

| Address | Function | Verifying call | Used in |
|---|---|---|---|
| 0x004FCE30 | GetPowerRatio | `disassemble_function` → `MOV EAX,[ECX+0x53a4]`, `MOV ECX,[ECX+0x53a8]` | §0 power-offset adjudication, H3, C10 |
| 0x004FA350 | Begin_Production | `decompile_function` (category resolve, prereq gate `type vtable+0x94`, lazy-alloc, FUN_005007a0/RTTI_To_TypeArray refs) | §0 factory-pointer naming, F1, H7, C19/C20 |
| FactoryClass | struct layout (0x74) | `get_struct_layout FactoryClass` (+0x4C IsInit / +0x4D IsAlloc; full field map §2a) | §0 +0x4C/+0x4D adjudication, §2a field map |
| 0x00508C30 | AI_AssessPower | `decompile_function` (power recompute, occupied-reactor `local_d` zeroing, RecalcAllRates) | H3, §4.2 #15 |
| 0x004F9950 | Add_Credits | `decompile_function` (`[+0x30C]+=`) | H1, C15 |

**v2 verification pass (2026-06-04), VERIFIED-LIVE this pass:**

| Address | Function | Verifying call | Used in |
|---|---|---|---|
| 0x006F47A0 | GetBuildStepTime | `disassemble_function` (full order, no ×0.9, per-iter MF trunc, ftol RC=11) — R1 | H4/H5, C5/C10/C11, §2a, §8 P0 |
| 0x004C9B20 | FactoryClass::AI | `read_memory 0x004C9BD5/0x004C9BF1` + `disassemble` (per-step charge, single-remainder, OnHold rewind, completion settlement) — V1 | F4/F5, C3/C5/C15 |
| 0x004F9790 | Spend_Money | `decompile` (silo-drain, +0x30C) — V1 | H1 |
| 0x004CA130 / 0x004CA1A0 / 0x004CA5A0 / 0x004CA620 / 0x004C9FF0 | IsComplete / CompletedProduction / StartNextQueued / RemoveFromQueue / AbandonProduction | `decompile`/`disassemble` (queue+cancel+refund) — V2 | F9, C6/C7/C8/C12 |
| 0x0055AFB0 | PerTickUpdate | `disassemble` (factory-before-house, null-check asymmetry) — V3 | G2, C1, §4.2 #6 |
| 0x007E88D0 / 0x007EA8A0 (+0x5C @0x007E892C/0x007EA8FC) | Factory/House vtable bases | `read_memory` — V3 | §2g vtable slots |
| 0x00509140 | prereq-revalidation | `disassemble` + `get_function_callers` (3-triple gate, abandon, self-delete) — V4 | C9/C19, §2c |
| 0x004F7870 | CanBuild | `decompile` (token switch, TechLevel house+0x1d4/type+0x18d, Required/Forbidden 1<<Type[0xb8], BuildLimit) — V4 | H6, C19 |
| 0x004FB0E0 / 0x006A8B30 / 0x00734250 / 0x004C6CB0 / 0x00443c60 / 0x0043c2d0 | Place_Production / StripClass::AI / pending-building setter / EventClass::Execute / ExitObject_Main / Receive_Radio | `decompile` (delivery, RTTI mapping fix, radio codes) — V5 | C12/C13, §2c/§2d/§2g |
| 0x00522D50 / 0x00445F80 / 0x00445880 / 0x004F9610 | DepositOreFromStorage / OnConstructionComplete / Limbo / Add_Tiberium_Credits | `decompile`/`disassemble`/`read_memory 0x004604d8/0x00511cfb/0x0066fc60` (purifier COUNT base, PurifierBonus@Rules+0xf3c, IncomeMult@HouseTypeClass+0x148, OrePurifier flag@BuildingTypeClass+0x16cc, 5.0@0x007eaa00) — R4 | H2, C14/C16/C17/C18 |
| 0x00500B40 / 0x004FCE00 / 0x00687F10 / 0x00686b20 | Read_Scenario_INI / Set_Credits_And_Color / Create_Houses / Full_Init | `decompile`/`disassemble` (+0x1DC both paths, gating) — R3/E1 | §0 +0x1DC, H11, §2b.1 |
| 0x004FC0B0 / 0x004FC9E0 / 0x004FCBD0 / 0x004F9A50 / 0x004F9B70 / 0x004F9F90 / 0x004F54A0 / 0x00501640 + `get_struct_layout HouseClass` | MPlayer_Defeated / Flag_To_Win / Flag_To_Lose / IsAlliedWith / MakeAlly / BreakAlliance / ctor / Recalculate_Alliances | `decompile`/`get_struct_layout` (units-only destroy, GameMode gate, directional Allies, scale-blocker inventory) — E2 | H9/H10, §0 Ally, §6.6 |

### 9.2 Verified prior-session / cross-digest VERIFIED (NOT re-read; promoted items moved to §9.1 v2)

The FactoryClass AI/StartProduction/CompletedProduction/StartNextQueued/RemoveFromQueue/AbandonProduction methods; HouseClass ctor/Update/Place_Production/Spend_Money/Add_Tiberium_Credits/DepositOreFromStorage/CanBuild/IsAlliedWith/MakeAlly/BreakAlliance/MPlayer_Defeated/Flag_To_Win/Lose/Create_Houses; PerTickUpdate/EventClass/FUN_004FAA10/StripClass::AI/pending-building-setter/prereq-revalidation; and the FactoryClass/HouseClass vtable slots — are now **VERIFIED-LIVE v2** and live in §9.1. AbandonProduction's correct entry is **0x004C9FF0** (0x004CA0E0 is an interior address, not an alias).

Genuinely NOT re-read this pass (remain prior-session/cross-digest VERIFIED):

- FactoryClass: ctor 0x004C98F0, Suspend 0x004C9E60, SetRate 0x004C9EA0 (÷54 magic `0x4BDA12F7`), CalcRate 0x004C9FB0, RecalcAllRates 0x004CA6E0, dtor 0x004CA790 (Digests A/C/V1).
- HouseClass: DepositWeedCredits 0x004F9700, GetFactoryCount 0x00500910, Find_Factory 0x004F83C0 (Digests E/F).
- Globals: Main_Tick 0x0055D360; g_FactoryClass_Array @0x00A83E34/count 0x00A83E40, g_HouseClass_Array @0x00A8022C/count 0x00A80238, g_CurrentFrameCounter 0x00A8ED84 (V1/V3/Digests A/C/D).

### 9.3 DOC-ONLY (corroborated by a digest, NOT re-decompiled — re-verify before load-bearing)

REMOVED from this section (now §9.1 VERIFIED-LIVE v2): `GetBuildStepTime 0x006F47A0` full formula/truncation order, the `×0.9` (REFUTED), MultipleFactory float-compare direction, BuildSpeed double (→ R1); `Math__ftol` RC (now VERIFIED: control word 0x00822d80=0x0E7F, RC=0b11 truncate, R1).

Still DOC-ONLY:

- `AI_ManageProduction 0x0050AF10` / `AI_ResumeProduction 0x0050B1D0` bodies (SUPERWEAPON; only CALL placement verified V3).
- `Record_Last_Built` (Place_Production tail).
- `PriorityToColorScheme` table bytes @0x0083ED14 / 0x0083ED1C.
- AIVirtualPurifiers: Rules+0x1324 **offset VERIFIED-LIVE (R4)**; the **index-field identity (house+0x184 AI-difficulty) is still OPEN** (see §9.4).

### 9.4 REMAINING OPEN (still must re-decompile to settle; do NOT treat as fact)

Closed v2 (no longer open — see §9.1): `Primary_For*` +0x53AC/+0x53B0 binding (RESOLVED → Aircraft@+0x53AC, Infantry@+0x53B0, R2/E2); +0x1DC StartingCredits (RESOLVED → both paths write, R3/E1); Ally bitmask +0x5788 (VERIFIED-LIVE v2, E2).

Still open:

- **AIVirtualPurifiers index-field identity** — Rules+0x1324 offset VERIFIED-LIVE (R4); whether `house+0x184` is the AI-difficulty field is UNVERIFIED. Gates P7's `ai_virtual_purifier_*` test only.
- **SpecialItem (+0x68) writer / SW-begin path** — not located; cannot prove value 0 unreachable, so 0-vs-−1 must NOT be collapsed (V2).
- **War-factory exit-link BREAK radio code** — establish is 0x02; the break code that fires on footprint-clear was not isolated (V5, UNCHECKED).
- **read==write-wallet equivalence** for the +0x24/+0x18 affordability sub-object — Spend_Money writes +0x30C; the +0x18 read-slot target was not decompiled (V1, UNCHECKED).

### 9.5 Rust source consumed (in-tree; v1 + v2 G1 corrections)

- `src/sim/world/world_hash.rs` — hash bands 157-282: hash_houses 157-184, hash_production 187-271, hash_power_states 274-282 (§4.4).
- `src/sim/world/techno_ai.rs` — 107 Structure-arm no-op (S8 marker); 162 unit_ai_shadow_step (shadow counterpart).
- `src/sim/house_state.rs` — `country` (line 24) and `waypoint_edge` (line 48) are real serialized HouseState fields, both absent from the hashed set (§4.4).
- `src/sim/snapshot.rs:24` — `SNAPSHOT_VERSION = 17` confirmed (the 17→18 bump in §6.4/§8 P5 is correct).
- `power_system.rs:127` — `is_low_power = produced < drained` (derived; §4.4 downgrade).
- `production_tech.rs` — low-power port `457-473` (445-456 is doc comment); `apply_multiple_factory_scaling_ppm` def `429-443` (call site 407); stolen-tech/`MATCH_TECH_LEVEL=10`/disabled-prototype `48-54,18,139`; `matching_factory_count_for_owner` 518-534.
- `production_queue.rs` — `credits_entry_for_owner` 74-92; upfront charge 218; full-cost refund 783/837/876; `cancel_by_type_for_owner` `.rev()` 811; PPM `progress_carry`/`remaining_base_frames` 881-898.
- `miner_dock_sequence.rs:1162-1166` purifier-count bonus + duplicate `slave_miner.rs:338-342` — both multiply by purifier COUNT, no StorageCapacity term anywhere in src/sim (grep clean).
- `production_spawn.rs:111-117` silent `active_producer` write; `production_types.rs:198/201/138/237/213/235`; `production_tech.rs:18/48-54/139`; `war_factory_exit.rs:28/67`; `house_state.rs:24/48`.

### 9.6 Research docs / digests consumed

- Input digests A–G (parallel Ghidra decode + Rust-map lanes) and the adversarial completeness critic, reconciled here.
- House style mirrored from `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` and `docs/research/LOGICCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.
- Companion program: the core-engine-substrate plan + the mission/radio substrate plan (Slice 8 T2–T6 parity-harness rhythm and `SNAPSHOT_VERSION` 16→17 bump are the template for §8).

---

*End of study. The substrate is additive and shadow-first: nothing in P1–P4 changes a hashed bit; the authority flip and `SNAPSHOT_VERSION 17→18` land at P5 (with the C1 ordering lock folded in, or P8 takes a second bump); P6–P8 fix the verified DRIFTs (cancel-end, prereq revalidation 3-way, purifier base, IncomeMult, ordering) as authoritative behavior; P9 is the deterministic-replay acceptance gate. P0's charge/build-time/Primary_For*/+0x1DC targets are VERIFIED-LIVE v2 (P3 unblocked, P7 count-base + IncomeMult unblocked); only the AIVirtualPurifiers index-field identity remains open and gates P7's `ai_virtual_purifier_*` test alone. AI build choosers, superweapon manage/resume, and MPlayer_Defeated/diplomacy effects are explicit seams, not designed here.*
