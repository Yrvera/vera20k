# GSI-15.06 — EVA queueing and house voice (VoxClass) — Phase 7 disparity scan

Read-only scan, 2026-09-05. Worktree `clean-slate-system-impl-891469` @ `4b89ef52`
(branch `feature/phase-7-parity-close-284594`). Binary: gamemd.exe (Ghidra, base 0x400000).
Retail data: `ini/evamd.ini`, `ini/rulesmd.ini`.

Verdict first: the System Map DRIFT for GSI-15.06 is **confirmed and now demonstrated**.
VERA has no VoxClass model — the app hard-codes a per-event "queue vs standard" choice, keeps one
FIFO, drops STANDARD lines whenever the voice slot is busy, has no priority tiers, no
pending-immediate slot, no 500 ms inter-line gap, and its registry ignores `Type=`/`Priority=`.
On top of that the three "state" emitters the plan calls Phase-6 work (`EVA_UnitLost`, low power,
insufficient funds) are app-side edge detectors with predicates that differ from gamemd, and the
whole sidebar-click family of lines (`Building`/`Training`/`On hold`/`Canceled`/
`New construction options`/`Cannot deploy here` on a bad placement click) has no producer at all.

Legend for dispositions: MATCH (bounded), DRIFT (demonstrated), MISSING, EXCLUDED, UNRESOLVED.
"Frequency" is per ordinary 20–30 min stock skirmish for the local human player.

---

## 1. Scope enumeration — native mechanisms (bodies read)

### 1.1 VoxClass core (all bodies decompiled/disassembled this scan)

| Address | Identity (Ghidra label, verified against body) | Role |
|---|---|---|
| `0x00753000` | `VoxClass::ReadEVAINI` | Walks `[DialogList]`, allocates 0x54-byte entries, sets defaults: Volume `1.0f` (+0x28), `Priority = 1 (NORMAL)` (+0x48), `Type = 0 (STANDARD)` (+0x4C), state 2 (+0x50), then `ReadINI` per entry. Only `EVAMD.INI` is read (strings `0x825df0`, "Reading EVAMD.INI"); no `eva.ini` merge. |
| `0x00752DB0` | `VoxClass::ReadINI` | Reads `Volume=` (0x846568), `Type=` (0x824314) by `stricmp`: `QUEUE`(0x8467cc)→1, `STANDARD`(0x8467c0)→0, `INTERRUPT`(0x816120)→2, `QUEUED_INTERRUPT`(0x8467ac)→3; `Priority=` (0x84301c): `LOW`(0x8161dc)→0, `NORMAL`(0x8161d4)→1, `IMPORTANT`(0x8467a0)→2, `CRITICAL`(0x8161c0)→3; then `Yuri=`→+0x2C, `Russian=`→+0x35, `Allied=`→+0x3E (9-byte `strncpy`, NUL-terminated at +0x34/+0x3D/+0x46). String bytes confirmed by `read_memory 0x8467a0`, `0x8161c0`, `0x816120`, `0x824314`. |
| `0x00752700` | `VoxClass::PlayEVA(name, typeOverride)` | Linear `stricmp` scan of the array (`0xb1d4a4`/count `0xb1d4b0`); calls `QueueVoice(index, typeOverride, priority=-1)` — **priority is always the entry's own**; only the type can be overridden per call site (`0x0075273A OR ECX,-1` for the not-found case, `0x0075274F MOV ECX,ESI` for found). |
| `0x00752480` | `VoxClass::QueueVoice(index, type, priority)` | Guards: stream player exists (`0xb1d4cc`), `0 <= index < count`, suspend counter `0xb1d3d8 == 0`, entry **is not the one currently playing** (`0xb1d4c4`). `-1` type/priority → entry defaults. If `type == 2 (INTERRUPT)` **and something is playing**: pops/frees every node of the interrupt list, marks the current entry done, `StreamPlayer::Stop`, `ClearAllQueues`, and zeroes the 64-bit gap `0xb1d4d0/0xb1d4d4`. Then `FindInQueues(entry)`: if a node for this entry exists **with the same type** it is a duplicate and is dropped; otherwise `InsertIntoQueue`. Finally `PlayNextQueued`. |
| `0x00752590` | `VoxClass::InsertIntoQueue(entry, type, priority)` | Allocates a 0x20 node (`+0x0C entry, +0x14 priority, +0x18 type, +0x1C seq = counter%100`), sets entry state 1 (queued). Routing: `type==3` → interrupt list `0xb1d3c8`; `type==1 (QUEUE)` → priority list `0xb1d450 + priority*0xC` (LOW..CRITICAL); else `priority==3 (CRITICAL)` → critical list `0xb1d3f0`; else (STANDARD/INTERRUPT, non-critical) → the single **pending-immediate slot** `0xb1d4b8`, taken only if the interrupt and critical lists are empty and (slot empty or `slot.priority < priority`, strict); otherwise the **new node is discarded** (state 2). |
| `0x00752680` | `VoxClass::FindInQueues(entry)` | Searches interrupt list, the pending slot, the four priority lists (0xb1d474 down to 0xb1d450) and the critical list for a node whose entry matches. |
| `0x00752760` | `VoxClass::PlayNextQueued` | Whole body gated on: `[0xa8ed64]==0` (no-audio flag, written once at `0x0052fb0a`, also gates `ThemeClass`), sound system present (`FUN_00407000` = `[0x87e728]!=0`), stream player present, `StreamPlayer::IsPlaying()==0`, **`now_ms > StreamPlayer::GetEndTime() + gap64` (unsigned, `0x007527A1..0x007527CF`)**, pause counter `[0xb1d428]==0` (`0x007527D5`). Then: marks the current entry done; dequeue order **interrupt list (0xb1d3c8) → critical list (0xb1d3f0) → pending slot (0xb1d4b8) → priority lists 0xb1d474 (CRITICAL) … 0xb1d450 (LOW)**, FIFO inside each list; when an interrupt or critical node is taken **the pending slot is freed** (`0x00752824`/`0x00752855`). Column select by `[0xb1d4c8]`: `0`→+0x3E Allied, `1`→+0x35 Russian, **anything else**→+0x2C Yuri (`0x007528E8..0x007528FE`). Filename = column + `".WAV"` (`0x844768`), `StreamPlayer::PlayFile(name, 1)`; on success: **gap = 500 ms** (`0x0075296D MOV [0xb1d4d0],0x1F4`), current entry/type/priority stored at `0xb1d4c4/0xb1d3b8/0xb1d3e0`, entry state 0 (playing). |
| `0x007529E0` | `VoxClass::PumpAndCheckActive` | `PlayNextQueued` then "anything playing or queued?" — used by `HouseClass::Update`'s victory/defeat savour wait (`0x004F861C..0x004F8777`, 0x78 × 16 ms buckets) and by `GameExit::*`. |
| `0x00406FC1` | `AudioSystem::Pump` → `PlayNextQueued` | The per-service-loop pump (documented in `src/app/frame.rs:118-127`). |
| `0x007534E0` | `VoxClass::SetSide(side)` | `-1 → 0`, else stored to `0xb1d4c8`. Sole caller `InitSideMixFiles @ 0x00534FA0`, called from `ScenarioClass::Full_Init` (`0x0068781F`, `0x00687833`, `0x006878C5`) and `Load_Game_Content_From_Stream 0x0067E761` with **the local house's `HouseType+0xBC` (Side index)** (`0x00687801..0x00687807`: `Houses[PlayerIndex]->Type->+0xBC`). Note `InitSideMixFiles` maps side 2→1 **only for the MIX file names** after `SetSide` already stored 2, so Yuri keeps the Yuri voice column. |
| `0x007535B0/0x00753620` | `PauseEVA/UnpauseEVA` (`0xb1d428`) | Game pause; already modelled in `src/audio/sfx.rs:887-930` (`VoiceSuspend`). |
| `0x00753570/0x00753580` | `SuspendEVA/ResumeEVA` (`0xb1d3d8`) | Blocks *queueing*, not playback. Callers not enumerated this scan (UNRESOLVED, see §4). |
| `0x007535D0`, `0x00752370`, `0x00752A40` | `ResetAll`, `ClearAllQueues`, `RemoveFromQueues` | Session reset / explicit removal (used e.g. by `SuperClass::Launch` per PSYCHIC_DOMINATOR report). |
| `0x007533F0` | `VoxClass::LoadFromSave` | The queue **is persisted** in native saves. |

Retail `evamd.ini` (486 sections): `Type=` present on 60 entries, all `QUEUE`; **no stock entry uses
`INTERRUPT`/`QUEUED_INTERRUPT`** (those reach only through the per-call type override `2` at
`SelectClass::Action` OnHold/Canceled and `GameExit::BattleControlTerminated`). `Priority=`: 35 LOW,
7 NORMAL, 16 IMPORTANT, 9 CRITICAL, the rest default NORMAL. No stock `Volume=` line.

Stock rows the report leans on (all from `ini/evamd.ini`):

| Event | Type | Priority | Routing in gamemd |
|---|---|---|---|
| `EVA_UnitLost` | STANDARD | IMPORTANT (2) | pending slot, replaces a lower-priority pending line |
| `EVA_OurBaseIsUnderAttack` / `EVA_OreMinerUnderAttack` / `EVA_OurAllyIsUnderAttack` | STANDARD | NORMAL (default) | pending slot |
| `EVA_InsufficientFunds` | STANDARD | NORMAL | pending slot |
| `EVA_LowPower` | QUEUE | IMPORTANT | priority list [2] |
| `EVA_ConstructionComplete`, `EVA_UnitReady`, `EVA_CannotDeployHere`, `EVA_Building`, `EVA_Training`, `EVA_OnHold`, `EVA_Canceled`, `EVA_UnableToComply`, `EVA_SelectTarget`, `EVA_PrimaryBuildingSelected`, `EVA_Repairing`, `EVA_StructureSold`, `EVA_UnitSold`, `EVA_UnitPromoted`, `EVA_BridgeRepaired`, `EVA_NewRallyPointEstablished` | STANDARD | LOW | pending slot (lowest; discarded if any higher line is pending) |
| `EVA_NewConstructionOptions` | QUEUE | LOW | priority list [0] |
| `EVA_BuildingCaptured`, `EVA_StructureGarrisoned`, `EVA_StructureAbandoned`, `EVA_BuildingInfiltrated` | QUEUE | NORMAL | priority list [1] |
| `EVA_PlayerDefeated` | QUEUE | IMPORTANT | priority list [2] |
| `EVA_YouAreVictorious` (call-site type 0) / `EVA_YouHaveLost` | STANDARD | NORMAL | pending slot |
| `EVA_NuclearMissileLaunched`, `EVA_LightningStormCreated`, `EVA_IronCurtainActivated`, `EVA_ChronosphereActivated`, `EVA_BattleControlTerminated` (call-site type 2) | STANDARD | CRITICAL | critical list (never discarded, jumps the pending slot and frees it) |

### 1.2 Stock skirmish emitters (75 xrefs to `PlayEVA`; the ones a skirmish reaches)

| Site | Event | Gate established from the body |
|---|---|---|
| `0x004D98C0` (unlabeled, `TechnoClass::Death_Announcement` — vtable slot **+0x3B8**; referenced from the Aircraft/Infantry/Unit vtables `0x7e265c/0x7e904c/0x7eb410/0x7f6028`, UnitClass base `0x7f5c70`) | `EVA_UnitLost` | owner passes `0x0050B6F0` (**local-player** test: `this == PlayerPtr` in MP, `+0x1EC||+0x1ED` in campaign — the Ghidra label `IsHumanPlayer` is transposed, see the plate comment on `0x0050B730`); `TechnoType+0xD54 == 0` (`Spawned=` per `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md:536` — carrier Hornets, V3/Dreadnought missiles are silent); `CreateRadarEvent(7, cell)` returned 1 (type 7: dedup 8 cells, unique, vis 0 + blink 200 frames, per `RADAR_EVENT_CLASS_GHIDRA_REPORT.md:100`). Callers of slot +0x3B8: `AircraftClass::ReceiveDamage 0x00416613`, `0x005180F4` (Infantry damage path, unlabeled), `UnitClass::ReceiveDamage 0x00737DC9/0x00737E39/0x00737E68`, `TemporalClass::AI 0x00629C76`, `TemporalClass::Update 0x0071AA3A/0x0071AAB5`. |
| `HouseClass::NotifyUnderAttack 0x004F93E0` (disassembled) | `EVA_OreMinerUnderAttack` (`0x004F94FB`), `EVA_OurBaseIsUnderAttack` (`0x004F9556→0x004F95B3`), `EVA_OurAllyIsUnderAttack` (`0x004F95AE→0x004F95B3`) | Victim's `vtbl+0x80` false; "local" = `this == PlayerPtr` (MP); harvester branch when `TechnoType+0x408 (UndeploysInto) != 0 && TechnoType+0x5EC (ResourceGatherer) != 0`; **the only rate limit is `CreateRadarEvent(type 4 / 3 / 0x10, cell)`** returning 1 (types 3/4: dedup 8 cells, unique, live vis 200 + blink 600/400 frames). Base and ally lines are followed by `VocClass::PlayAtPos(Rules.BaseUnderAttackSound [+0x184], 1.0f, pan 0)` (`0x004F95B8..0x004F95CF`). Callers: `BuildingClass::ReceiveDamage 0x0044297B` (source non-null `0x00442942`, `[ESP+0x30]` non-zero, `BuildingType+0x232 Insignificant == 0`, victim `vtbl+0x80` false — **no attacker-house relation test in the inspected window**) and `TemporalClass::InitiateWarp 0x0071B0A7/0x0071B0CA`. |
| `UnitClass::ReceiveDamage 0x00738530` | `EVA_OreMinerUnderAttack` | `[ESP+0x60]` non-zero, `UnitType+0xE0E` set (Harvester, per RADAR_EVENT report OQ2), owner `== PlayerPtr` (`0x007384D3`), `CreateRadarEvent(4, cell)`. |
| `HouseClass::Update 0x004F8B3C..0x004F8BE1` | `EVA_InsufficientFunds` | `this == PlayerPtr`; timer `+0x57D4` expired; available credits (`(+0x24)->vtbl+0x18`) `< 100`; `GetFactoryCount` sum `+0x537C (infantry) + +0x5380 (vehicle) + +0x5384 (building) + +0x5388 (naval) > 0` (aircraft `+0x5378` excluded; identities from `HouseClass::GetFactoryCount 0x00500940`) — i.e. **the player owns any conyard/barracks/war factory/shipyard**, regardless of what is queued. Side effects: sidebar credits flash `FUN_006D0EC0`, timer re-armed to `SpeedNormalize(ftol(Rules.SpeakDelay[+0x16A8] × 792.25 [0x7E27F8]))` — `SpeakDelay=2` → 1584 → `(1584<<3)/(speedIndex+1)` frames (`FUN_005FB2E0`, receiver `0xA8EB60` = game-speed index 0..7). The silo block right after (`0x004F8BFF..0x004F8C53`: `capacity - stored < 30 && capacity > 50`) **re-arms the same timer without any EVA** — there is no `EVA_SilosNeeded` in YR (string absent; EXCLUDED). |
| `HouseClass::Update 0x004F8C56..0x004F8DAB` | `EVA_LowPower` | `this == PlayerPtr`; `PowerDrain > PowerOutput` **and** `PowerDrain != 0` **and** (`Output == 0` or `Output/Drain < 1.0`) — if power is fine the one-shot guard `[0xA8F040]` is cleared; then **requires `CountOwnedInstances` > 0 for one of `Rules.BuildPower[0..2]` (`+0x8B0`; stock `NAPOWR,GAPOWR,YAPOWR`)**, otherwise nothing happens and the guard is *not* cleared; then if the guard is 0: `PlayEVA`, chat message (string 0x949) for `Rules.MessageDelay[+0x14C0]×792.25` frames, guard = 1; timer `+0x57BC` re-armed to the SpeakDelay value (not consulted as a gate in this body). |
| `StripClass::AI 0x006A8E2F` | `EVA_ConstructionComplete` | Factory completed object switch (`vtbl+0x2C` RTTI jump table at `0x006A92C4`); building case. |
| `HouseClass::Place_Production 0x004FB644` | `EVA_UnitReady` | `CreateRadarEvent(6, cell)` returned 1 (type 6: dedup **2 cells**, unique, blink 200 frames). |
| `HouseClass::Place_Production 0x004FB377`, `UnitClass::Deploy 0x0073950A`, `DisplayClass::BandBox_LeftUp 0x004ABC80` | `EVA_CannotDeployHere` | placement failure for the local player (`0x004FB369 CMP EBP,[PlayerPtr]`); deploy failure (human && `type+0x5EC==0`, AMCV report); **invalid building-placement click**. |
| `SidebarClass::AddCameo 0x006A6415`, `StripClass::AddEntry 0x006A8837` | `EVA_NewConstructionOptions` | `[0xA8E7AC]==0` (scenario-init nesting counter, decremented at `0x006878E6`) and cameo RTTI `!= 0x1F` (SuperWeaponType). |
| `SelectClass::Action` `0x006AB498/0x006AB6C9` | `EVA_Building` / `EVA_Training` | `CMP EBP,0x10` (RTTI InfantryType) → `Training`, else `Building`. |
| `SelectClass::Action` `0x006AB007/0x006AB108`, `0x006AAE39`, `0x006AB3B1/0x006AB693`, `0x006AAFA7` | `EVA_OnHold` (type 2), `EVA_Canceled` (type 2), `EVA_UnableToComply`, `EVA_SelectTarget` | sidebar click outcomes (per EVA_SYSTEM_DEEP_DIVE §4; type overrides read from the call sites in that report, not re-disassembled here). |
| `0x00448226` (BuildingClass primary-factory setter) | `EVA_PrimaryBuildingSelected` | `[this+0x3D3]=1` then owner `0x0050B6F0`. |
| `BuildingClass::Sell 0x00449CE5/0x0044AB36`, `FootClass::OnSold 0x004D9F94` (+ `Rules.SellSound` PlayAtPos) | `EVA_StructureSold`, `EVA_UnitSold` | owner `0x0050B6F0`; sell of a `+0x408`-less type. |
| `0x004470B7`, `BuildingClass::MissionRepairAndProduce 0x0044C507/0x0044B973/0x0044BDC5/0x0044BFEA` | `EVA_Repairing`, `EVA_UnitRepaired`, `EVA_InsufficientFunds` (repair) | repair-facility flow. |
| `BuildingClass::ChangeOwner 0x0044848A/0x00448428` | `EVA_BuildingCaptured` (radar type 10) / `EVA_TechBuildingLost` | capture flow (`0x00448459` also calls `QueueVoice` directly). |
| `BuildingClass::OnSpyInfiltrate 0x00457288..0x0045758B` | infiltration family | spy. |
| `BuildingClass::OnConstructionComplete 0x00446995` (gated `[0xA8B538]==0`), `SuperClass::AI_Ready/AI_Charging`, `SuperClass::Launch` ×7 | `*Detected`, `*Ready`, `*Activated` | superweapons. `[0xA8B538]` = "local player defeated/spectating" (set 1 at `HouseClass::MPlayer_Defeated 0x004FC205`, cleared at session prepare `0x0052DA94`/`0x005C3ABE`). |
| `HouseClass::Flag_To_Win 0x004FCBA9` (type 0), `Flag_To_Lose 0x004FCDA1`, `MPlayer_Defeated 0x004FC2EA/0x004FC3BC`, `GameExit::BattleControlTerminated 0x00686616` (type 2) | outcome lines, `EVA_PlayerDefeated` | match end. |
| `BuildingClass::SetRallyPoint 0x00443A69`, `RadarClass::PlaceBeacon`, `MakeAlly/MakeEnemy`, `AddGarrisonOccupant 0x005229C1`, `CheckAutoSellOrCivilian 0x004582D8`, `InfantryClass::PerCellProcess 0x00519BC9` (bridge), `TechnoClass::AI_Update 0x006FA0CB/0x006FA139` (promotion), crate handlers `0x00482EBB/0x004830AA/0x0048328A` | misc | as labelled. |

`GoOnline/GoOffline 0x00452355/0x004523DE` (`EVA_BuildingOnLine/OffLine`) and `LostPoweredCenter`/robot-tank
lines were not gated-checked (UNRESOLVED, §4).

### 1.3 Current Rust owners (production path)

- Registry: `src/rules/sound_ini.rs:738-822` `EvaRegistry` (Allied/Russian/Yuri only), loaded by
  `src/app/loading/transitions.rs:593-618` — reads **`evamd.ini` then merges `eva.ini`**.
- Voice slot/queue: `src/audio/sfx.rs:1377-1500` (`queue_eva_sound`, `play_standard_eva_sound`,
  `interrupt_eva_sound`, `advance_voice_queue`), pumped from `SfxPlayer::pump` (`:1598-1607`) via
  `pump_audio_service` (`src/app/match_runtime/sim_tick.rs:737-747`) every app frame, and again at the
  head of `drain_sound_events`.
- Dispatch: `src/app/presentation/building_anim.rs:184-470` (`drain_sound_events`), producers in
  `src/app/match_runtime/sim_tick.rs` (`announce_local_state_evas :167-281`, the `SimSoundEvent`
  → `GameSoundEvent` translation `:1380-1910`), `src/app/in_game.rs:643-675` (battle-control
  terminated interrupt), outcome wait `sim_tick.rs:284-330`.
- Side key: `building_anim.rs:144-162` `eva_faction_key` (country-name match: `yuricountry` →
  Yuri, four Soviet country names → Russian, else Allied).
- Sim-side emitters: `src/sim/combat/mod.rs:4471-4480` (`UnderAttackEvent`), `src/sim/world/mod.rs:2997-3012`
  and `:7668-7690` (radar dedupe → `SimSoundEvent::UnderAttack{eva_allowed}`),
  `src/sim/production/production_queue.rs:513` (`BuildingComplete`), `:641` (`UnitComplete`),
  `src/sim/world/world_spawn.rs:1889/1899` (`CannotDeployHere`, MCV only), garrison
  (`src/sim/passenger.rs:802`), bridge repair (`world_orders.rs:447`), promotion
  (`techno_ai.rs:568`), outcomes (`world/mod.rs:5031-5081`), superweapons (`sim_tick.rs:1466-1503`).
- `src/audio/voice_queue.rs` is the **unit acknowledgement** latch (`TechnoClass::Queue_Voice`),
  not VoxClass; it is in scope only because both share `voice_player`.

---

## 2. Per-mechanism findings

### M1 — Registry data: `Type=`/`Priority=`/`Volume=` and file set — **DRIFT**
Native: `ReadINI` stores type and priority per entry (defaults STANDARD/NORMAL); only `EVAMD.INI`.
Rust: `EvaRegistry` keeps three sound ids per event and nothing else (`sound_ini.rs:740-744`); the
app decides "queue vs standard" per `GameSoundEvent` variant by hand (`building_anim.rs:259-270`,
`:441-464`; `sim_tick.rs:589-596` for superweapons). `load_eva_registry` also merges `eva.ini`
entries that `evamd.ini` lacks — gamemd never opens `eva.ini`, so an RA2-only section would be a
VERA-only line (no stock example found; stock `evamd.ini` is a superset). Player effect: none by
itself, but it is the prerequisite for M2. Frequency: every line.

### M2 — Queue model (pending slot, priority lists, critical, discard rules) — **DRIFT**
Native routing is in `InsertIntoQueue`/`PlayNextQueued` (§1.1). Rust has one `VecDeque`
(`queued_voice`) fed by `queue_eva_sound`, and `play_standard_eva_sound` which **returns false
(drops the line) whenever the slot is busy or the queue is non-empty** (`sfx.rs:1424-1432`).
Concrete divergences, all on stock data:

1. STANDARD line while a line is playing. Native parks it in the pending slot and plays it after
   the current one (+500 ms). Rust drops it. Example: "Construction complete" (LOW) playing,
   "Unit ready" (LOW) arrives → native: both; Rust: the second is silent. Also the **outcome
   lines**: `OutcomeEva` goes through `play_standard_eva_sound` (`building_anim.rs:260-265`), so a
   match that ends while "Unit lost" is still speaking loses "You have lost/You are victorious"
   entirely, and `drive_local_player_outcome_voice_wait` then finishes immediately. Frequency:
   several times per match for LOW/NORMAL STANDARD lines; match-end case occasional.
2. Priority replacement/discard in the pending slot. Native keeps **one** pending STANDARD line and
   replaces it only with a strictly higher priority (`0x0075260F`, `slot.priority < new`). Example:
   "Our base is under attack" (NORMAL) pending, "Unit lost" (IMPORTANT) arrives → native replaces
   (base line never plays); reverse order → the base line is discarded. Rust queues QUEUE-typed
   cues FIFO and drops STANDARD cues, so it neither replaces nor orders by priority.
3. Interleaving of QUEUE lines with the pending slot. Native plays the pending STANDARD line
   **before** any priority-list line, and CRITICAL/interrupt nodes free the pending slot. Rust: the
   app-routed QUEUE cues (`LowPower`, `InsufficientFunds`, `UnitLost`, under-attack, garrison) all
   sit in one FIFO in arrival order. Example: "Low power" (QUEUE IMPORTANT) queued, then "Unit
   lost" (STANDARD IMPORTANT) → native: Unit lost first; Rust: Low power first.
4. Duplicate rule. Native drops a duplicate only when an existing node has the **same type**
   (`0x00752566`); Rust drops any same-id (`sfx.rs:1391-1397`). Equivalent on stock data because
   every call site of a given event uses one type (bounded MATCH).
5. Unlimited FIFO depth in Rust vs at most one pending STANDARD line + per-list FIFOs natively —
   bursts of distinct STANDARD lines (sidebar spam) would back up in Rust once P2 emitters exist.

### M3 — 500 ms inter-line gap — **MISSING**
Native starts the next line only when `now > end_time + 500 ms` (`0x007527A1..0x007527CF`;
gap set at `0x0075296D`, zeroed on interrupt). Rust `advance_voice_queue` starts the next queued
line the moment `player.empty()` (`sfx.rs:1482-1500`). Player effect: back-to-back lines with no
breath between them whenever two cues are queued. Frequency: every queued pair.

### M4 — Interrupt (type 2) — MATCH bounded / minor DRIFT
Rust `interrupt_eva_sound` always clears the queue and stops the slot. Native does that **only when
something is playing** (`0x00752480`: `DAT_00b1d4c4 != 0 && type == 2`); with an idle slot a type-2
line is routed like STANDARD and existing queued nodes survive. Stock reach: `BattleControlTerminated`
(Rust `in_game.rs:643-675`, MATCH for the exit path), `OnHold`/`Canceled` (no Rust producer, see E9).

### M5 — Pause/suspend/reset — MATCH bounded
`VoiceSuspend` (`sfx.rs:887-930`) models `0xb1d428` and `ResetAll`; the pause-side ordering was
verified by the Phase 6 audio PRs (`f91a5547`, `d318c230`). The separate `SuspendEVA` counter
(`0xb1d3d8`, blocks queueing) has no Rust counterpart; its stock callers were not enumerated
(UNRESOLVED).

### M6 — Side/column selection — MATCH bounded (stock), DRIFT for data
Native: one side index per session from the local country's `Side=` (`HouseType+0xBC`);
`0`→Allied, `1`→Russian, **any other value**→Yuri. Rust: per-event lookup keyed by hard-coded
country names (`building_anim.rs:157-161`); unknown/missing → Allied. Equal for the nine stock
countries. A mod country on a third side, or a map `[Countries]` override, resolves differently
(low frequency). Rust also resolves per *owner* rather than per local player, but every producer
gates on the local owner first, so the column is the local side in practice.

### M7 — Audio file lookup — bounded MATCH, not deeply checked
Native builds `<column>.WAV` and lets `StreamPlayer::PlayFile` open a raw file first, then the
MIX/bag path (DEEP_DIVE §6). Rust resolves the id through `audio.bag` indices then the sound
registry (`sfx.rs:2082-2100`); the retail EVA samples resolve either way. The raw-file override
(a loose `ceva048.wav` next to the exe) is not modelled — EXCLUDED for stock installs.

### M8 — Queue persistence across save/load — DRIFT (minor)
Native `VoxClass::LoadFromSave` restores queued lines; VERA's queue is app state and is not in the
snapshot (`src/sim/snapshot.rs:4394-4448` deliberately covers only the outcome edge). Player
effect: a line queued at the moment of saving is lost on load. Frequency: rare.

### M9 — Pump cadence and savour wait — MATCH bounded
`SfxPlayer::pump` advances the voice queue every app frame outside the sim gate (`frame.rs:118-127`),
matching `AudioSystem::Pump`. The victory/defeat wait (`HouseClass::Update 0x004F861C..`) is
modelled by `drive_local_player_outcome_voice_wait` — but see M2.1 for the dropped line.

### E1 — `EVA_UnitLost` — **DRIFT** (and the plan's claim is only half true)
Production owner is **not** Phase 6 combat: it is the app-side edge detector
`announce_local_state_evas` (`sim_tick.rs:167-281`) that scans local non-structure entities with
`dying == true` each sim tick and queues `EVA_UnitLost` once per newly-dying id, via
`queue_eva_sound` (FIFO). The sim emits no unit-lost event. Differences vs `Death_Announcement`:

- Gate: native suppresses `Spawned=yes` types (`TechnoType+0xD54`) and applies the radar type-7
  dedupe (**8 cells, 200 frames**): a second local death within 8 cells while the previous type-7
  diamond is alive produces no second line even after the first finishes. Rust announces every
  dying unit (only the voice-queue same-id dedupe suppresses), so a squad wiped over ~15 s yields
  more "Unit lost" repeats than native.
- Trigger point: native fires at the damage-kill sites (`ReceiveDamage` ×3 classes) and at
  temporal erase; Rust fires only for entities that enter the `dying` state — instant removals and
  (per the code comment `sim_tick.rs:200-203`) crush deaths are missed; chrono-erase not checked.
- Routing: native STANDARD IMPORTANT (pending slot, replaces lower pending lines); Rust FIFO.
Frequency: 20–100 deaths per skirmish; player-visible as both extra repeats and missed lines.

### E2 — Base / miner / ally under attack — **DRIFT** (Phase 6 wiring confirmed, cadence differs)
The sim path exists and is production-reached: `combat/mod.rs:4471-4480` pushes an
`UnderAttackEvent` for a Structure or miner damaged (`damage > 0`) by a **different house**;
`world/mod.rs:2997-3012`/`:7668-7690` runs the radar dedupe (`radar_events.push_owned`) and
carries its result as `eva_allowed` — this is the native coupling. Then the app adds
**`EVA_UNDER_ATTACK_COOLDOWN_TICKS = 450` (~30 s) across both cue kinds**
(`sim_tick.rs:16-18`, `:1875-1892`), documented as "native delay UNVERIFIED". Native has no such
timer: `NotifyUnderAttack` and `UnitClass::ReceiveDamage` are gated **only** by `CreateRadarEvent`
(8-cell dedupe against live events; a type-3 diamond lives 800 frames, type-4 600). Demonstrated
consequences: two bases (or a base and a miner) attacked >8 cells apart within 30 s → native plays
both lines, Rust one; a miner hit 8+ cells from the earlier ping → native repeats after the
diamond expires (~40 s), Rust after 30 s. Also:
- `EVA_OurAllyIsUnderAttack` (radar type 0x10) — MISSING (no ally relation in the emitter).
- Native's `BuildingClass::ReceiveDamage` window shows no attacker-relation test; Rust requires
  `attacker_owner != target.owner` — UNRESOLVED whether own-fire (force-fire on your own
  building) announces natively.
- Native skips `Insignificant=yes` buildings; Rust does not check it (stock `Insignificant=yes`
  buildings are civilian props, so low frequency).
Frequency: 5–30 attacks per skirmish; the 30 s clamp is player-visible in multi-front fights.

### E3 — `EVA_LowPower` — **DRIFT**
Rust: edge on `PowerState.is_low_power` (`produced < drained`, `power_system.rs:133`) via
`eva_low_power_active` (`sim_tick.rs:225-228`), queued (Type=QUEUE — correct). Native additionally
requires the house to own at least one `Rules.BuildPower` type (`0x004F8C97..0x004F8CFC`) before
announcing, and leaves the guard set while power is short. Demonstrated: a player who places a
barracks/refinery before any power plant hears "Low power" in VERA and nothing in gamemd; frequency
depends on build order (common opening in some styles). The one-shot guard/clear-on-recovery
semantics otherwise match (MBLK-005 already verifies the ordering). The accompanying chat message
(string 0x949, `MessageDelay`) is a GSI-13 surface, not checked here.

### E4 — `EVA_InsufficientFunds` — **DRIFT** (predicate and cadence)
Rust: edge on "any local factory `on_hold` with an object" (`sim_tick.rs:187-193`,
`eva_funds_stalled`), once per edge, queued FIFO. Native (`0x004F8B3C..0x004F8BE1`): while
`credits < 100` **and the player owns any conyard/barracks/war factory/shipyard**, the line (and a
sidebar credits flash) repeats every `SpeedNormalize(ftol(SpeakDelay × 792.25))` frames — with
`SpeakDelay=2` that is 1584 frames scaled by the game-speed table (e.g. 1811 frames at speed index
6, 1584 at index 7), and the silo-nearly-full branch pushes the next nag out by the same amount.
Nothing needs to be queued or stalled. Demonstrated: a broke player with an idle barracks hears
the nag every ~1.8–2 min in gamemd and never in VERA; a player at $150 with a stalled build hears
it in VERA and not in gamemd. Frequency: several per skirmish for most players. Routing: native
STANDARD NORMAL (pending slot), Rust FIFO.

### E5 — `EVA_ConstructionComplete` — MATCH bounded, M2 applies
Rust emits on the first completion of a building item (`production_queue.rs:505-513`), the same
moment `StripClass::AI` sees the completed factory object. Local-owner gate matches. Routed as
STANDARD in Rust → dropped when the slot is busy (M2.1).

### E6 — `EVA_UnitReady` — DRIFT (cadence)
Native gates on `CreateRadarEvent(6, exit cell)`: dedup **2 cells**, unique, 200 frames — two units
leaving the same factory within 200 frames give one line. Rust emits per unit
(`production_queue.rs:641`) as STANDARD (dropped if busy). Frequency: 30–100 per skirmish; net
effect is fewer lines than native in busy audio and more in quiet audio.

### E7 — `EVA_CannotDeployHere` — partly MISSING
Rust covers the blocked MCV deploy only (`world_spawn.rs:1889/1899`, deploy tests). Native also
plays it for **an invalid building-placement click** (`DisplayClass::BandBox_LeftUp 0x004ABC80`)
and for a failed `Place_Production` (`0x004FB377`); VERA's placement path produces only the cursor
feedback (`src/app/input/cursor.rs:72`, no sound event). Frequency: 5–20 misclicks per skirmish.
Native suppresses the deploy line for `ResourceGatherer` types (chrono-miner undeploy) — not modelled.

### E8 — `EVA_NewConstructionOptions` — **MISSING**
No producer (grep of `EVA_` names in `src/`: only `sound_ini.rs`, `sim_tick.rs`, `in_game.rs`).
Native fires it from `SidebarClass::AddCameo` for every new non-superweapon cameo once scenario
init has finished. Type=QUEUE LOW, so it also never interrupts. Frequency: 10–30 per skirmish.

### E9 — Sidebar click lines (`Building`/`Training`/`OnHold`/`Canceled`/`UnableToComply`/`SelectTarget`) — **MISSING**
Nothing in `src/sidebar` or `src/app` references these events or their sample ids. These are the
most frequent EVA lines in a skirmish (every queue click, every hold/cancel).

### E10 — `EVA_PrimaryBuildingSelected`, `EVA_NewRallyPointEstablished`, `EVA_StructureSold`, `EVA_UnitSold`, `EVA_Repairing`/`EVA_UnitRepaired` — **MISSING**
No producers (grep). Primary-factory selection has no Rust concept at all (only render-side
`is_primary` refinery anim flags). Sell is implemented sim-side (`WallSold` SFX) without the line.

### E11 — Capture/infiltration/superweapon Detected+Ready/`EVA_PlayerDefeated`/beacon/alliance — **MISSING**
Only the seven `SuperClass::Launch` "Activated" lines and the two outcome lines exist. Superweapon
`*Detected` (on the enemy's construction, gated on the spectator flag `0xA8B538`) and `*Ready`, and
`EVA_PlayerDefeated` (each AI opponent's defeat, QUEUE IMPORTANT) are stock-skirmish reachable.

### E12 — Garrison / abandon / bridge repair / promotion — MATCH bounded, one routing DRIFT
Garrison lines are queued (correct type). Bridge repair uses the radar dedupe result as the gate
(correct). `EVA_UnitPromoted` is a `GameSoundEvent::UnitPromotedEva` that `drain_sound_events`
does **not** match explicitly, so it falls into the spatial `_` arm (`building_anim.rs:461-467`),
`source()` returns `None` (`events.rs:419-443`) and it plays through **`play_sound_spatial` on the
16-channel SFX pool at Sound volume** — it overlaps other EVA lines and ignores the Voice slider.
Native: `VoxClass::PlayEVA` STANDARD LOW. Frequency: every promotion of a local unit.

### E13 — Spectator suppression `[0xA8B538]` — MISSING (post-defeat only)
After the local player is defeated the superweapon-detected line and radar recheck are muted
natively; VERA has no flag (residual already noted in `events.rs:602`). Frequency: only when the
player keeps watching after losing.

---

## 3. Ranked implementation candidates (grouped by shared prerequisite)

**P0 — VoxClass queue model (prerequisite for every cadence claim above).** Size: M (≈700 lines
with tests). Parse `Type=`/`Priority=`/`Volume=` in `src/rules/sound_ini.rs` and drop the `eva.ini`
merge in `src/app/loading/transitions.rs:593-618`; carry the **event name** (not a resolved sample)
in `GameSoundEvent` (`src/audio/events.rs`) plus an optional type override; replace
`queue_eva_sound`/`play_standard_eva_sound`/`interrupt_eva_sound` in `src/audio/sfx.rs:1377-1500`
with `QueueVoice`/`InsertIntoQueue`/`PlayNextQueued` semantics (pending slot with strict-priority
replacement, four priority FIFOs, critical + interrupt lists, same-type duplicate rule, 500 ms gap
after `end_time`, interrupt-only-when-playing); resolve the side column once per match from the
local country's `Side=` in `src/app/presentation/building_anim.rs:144-162`; route
`UnitPromotedEva` and `OutcomeEva` through it. Consumers to touch: `building_anim.rs:184-470`,
`sim_tick.rs` producers, `in_game.rs:643-675`, `quit_cascade.rs` (`voices_active`). Optional in the
same PR: persist the queue in the app-side save (M8).

**P1 — Native emitter predicates for the three "state" lines + under-attack cadence** (depends on
P0 only for routing; can land first). Size: M.
- `EVA_InsufficientFunds`: move to a sim-owned house timer (`src/sim/house_state.rs` or
  `power_system.rs` sibling) with the `credits < 100 && factory_count > 0` predicate, the
  `SpeakDelay × 792.25` re-arm, the silo re-arm quirk, and the game-speed normaliser (needs the
  speed index; `src/app/match_runtime/frame_pacer.rs` owns the VERA notion). Delete the
  `eva_funds_stalled` edge in `sim_tick.rs:187-193,229-232`.
- `EVA_LowPower`: add the `BuildPower`-ownership gate (`rules.build_power_types` already parsed at
  `src/rules/ruleset.rs:2592`) and keep the guard set while short — `sim_tick.rs:225-228` or a
  sim event from `power_system.rs:183`.
- `EVA_UnitLost`: emit from the death sites in `src/sim/combat/mod.rs` (damage kill) and the
  temporal erase path, gated on `Spawned` and a new radar type 7 in `src/sim/radar.rs`; delete the
  `dying` scan (`sim_tick.rs:167-281`, `match_audio.rs:22-24`).
- Under attack: remove `EVA_UNDER_ATTACK_COOLDOWN_TICKS` (`sim_tick.rs:16-18,1875-1892`,
  `match_audio.rs:25-28`); add the ally branch (radar type 0x10) and the `Insignificant` skip in
  `combat/mod.rs:4471-4480`; decide the own-fire question with one more `BuildingClass::ReceiveDamage`
  read (§4).

**P2 — Sidebar and placement lines** (independent of P0/P1; needs the sidebar command owner).
Size: S–M. `EVA_Building`/`Training`/`OnHold`/`Canceled`/`UnableToComply`/`SelectTarget` from the
sidebar click handlers (`src/app/input/dispatch.rs`, `src/app/presentation/sidebar_build.rs`,
`src/sidebar/`), `EVA_NewConstructionOptions` from cameo insertion (`src/sidebar/sidebar_view.rs`
+ the scenario-init nesting gate), `EVA_CannotDeployHere` from the invalid placement click
(`src/app/input/cursor.rs`/placement dispatch) and failed placement, `EVA_UnitReady` behind a radar
type 6 (2-cell) dedupe (`src/sim/radar.rs`, `production_queue.rs:641`),
`EVA_NewRallyPointEstablished` from the rally-point command, `EVA_PrimaryBuildingSelected` once a
primary-factory concept exists, `EVA_StructureSold`/`EVA_UnitSold` from the sell path.

**P3 — Remaining stock-reachable lines.** Size: S each. Superweapon Detected (with the `0xA8B538`
spectator flag) and Ready, `EVA_PlayerDefeated`, `EVA_BuildingCaptured`/`TechBuildingLost` (+ radar
type 10), infiltration family, `Repairing`/`UnitRepaired`, beacon and alliance lines.

---

## 4. Uninspected / unresolved

- `SuspendEVA/ResumeEVA` (`0x753570/0x753580`) stock callers (radar movie `0x006579BB` region?) —
  whether any skirmish path suspends queueing.
- `BuildingClass::ReceiveDamage` predicates `[ESP+0x30]` (likely damage result) and victim
  `vtbl+0x80` at `0x0044296A`; whether an attacker-relation test exists earlier than the inspected
  110-instruction window; `UnitClass::ReceiveDamage` `[ESP+0x60]`.
- Which death paths reach vtable `+0x3B8` besides the three `ReceiveDamage` bodies and the temporal
  erase (crush, sinking, self-destruct); the `0x005180F4` infantry site is in an unlabeled function.
- `GoOnline/GoOffline` (`EVA_BuildingOnLine/OffLine`) gating and stock reachability (EMP/blackout).
- `StreamPlayer::PlayFile` raw-file-before-MIX order and `VoxClass::SetGlobalVolume 0x752AB0`
  (options Voice slider mapping) vs `SfxPlayer::voice_volume`.
- Mapping of the game-speed normaliser table (`0x00832D0C`, `FUN_005FB2E0`) onto VERA's fixed
  67 ms tick for the `SpeakDelay` re-arm.
- The Rust radar type-config values in `src/sim/radar.rs` were not re-read against the native
  table (`0x007F0998`); `RADAR_EVENT_CLASS_GHIDRA_REPORT.md:330-345` lists earlier gaps
  (types 6–16 absent) that the code has since partly addressed.
- Campaign-only paths (mission dialog entries, `EVA_MissionAccomplished/Failed`, taunts) are out of
  the stock-skirmish scope and were not examined.

Evidence provenance: all addresses in §1 were read from the binary this scan
(`decompile_function`/`disassemble_function`/`disassemble_bytes`/`get_assembly_context`/
`read_memory`/`search_instructions`); prior reports (`EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md`,
`RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`) were used for the
radar type table, the `SelectClass::Action` type overrides and the `+0xD54/+0x5EC/+0x408/+0x232`
field names, each spot-checked where load-bearing. The `792.25` constant corrects the deep-dive
report's "900 frames per minute" assumption (`AUDIT_LOG.md:99` had already recorded it). Nothing
here is parity-demonstrated; every MATCH is bounded to the inputs named.
