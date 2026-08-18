# Core Service Profile — factory-house (FactoryClass / HouseClass)

**Service slug:** `factory-house`
**Source of truth:** `docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, v2 verification pass 2026-06-04, Ghidra-verified, addresses cited inline).
**One-line:** Per-house economy + power + prerequisites (HouseClass) and the per-(house,category) 54-step pay-as-you-go production state machine (FactoryClass); the production tick and the wallet.

---

## Purpose

The combined Factory + House substrate owns the **production pipeline and the per-player economy/power/diplomacy lifecycle**. FactoryClass is one production state machine per (house, build category), advancing a 0→54 progress counter and charging the owner incrementally; HouseClass is the wallet, the power accounting, the prerequisite/tech gate, the diplomacy bitmask, and the win/loss lifecycle. The two are walked by the tick spine in a fixed order: **all factories step, then all houses tick** (`LogicClass::PerTickUpdate 0x0055AFB0`).

The single defining behavior: gamemd charges a build's full cost **incrementally across 54 steps** and stalls (OnHold + rewind one step) when credits run out mid-build, settling exact cost on completion. Cancel refunds only the already-paid portion (`GetCost − Balance`).

---

## Owns (state / globals / structs)

### FactoryClass (struct size 0x74; per house, per category; lazily allocated)
- **Progress + timing:** `Production_Value` +0x24 (0→54, complete at 0x36); `Production_Step` +0x3C (=1 in YR); CDTimer block +0x2C..+0x34; `Production_Timer_Duration` +0x38 (the per-step rate, RecalcAllRates target).
- **Money:** `Balance` +0x60 (remaining-to-pay), `OriginalBalance` +0x64.
- **Object + queue:** `Object` +0x58 (TechnoClass* in flight); `QueuedObjects` DynamicVector +0x40.. (count +0x50, cap-incr 10 +0x54); `SpecialItem` +0x68 (−1 = none).
- **Flags:** `OnHold` +0x5C, `IsDifferent` +0x5D (render-only dirty), `IsSuspended` +0x70, `IsManual` +0x71 (default 1); `Owner` +0x6C (HouseClass*).

### HouseClass (struct size 0x160B8; one per player)
- **Wallet:** `Balance`/AvailableCredits +0x30C (internal ×100 scale); `SpentCredits` +0x2DC; `HarvestedCredits` +0x54E8 (statistics, += trunc(amount×5.0)); `StartingCredits` seed +0x1DC (campaign ×100 / MP raw; INERT after init).
- **Power:** `PowerOutput` +0x53A4, `PowerDrain` +0x53A8 (adjudicated this study — Digest B's +0x5384/+0x5388 read was OVERRULED; those are per-RTTI factory counts).
- **Factory pointers (the house→category→factory binding), +0x53AC..+0x53CC:** +0x53AC Aircraft, +0x53B0 Infantry, +0x53B4 Vehicles, +0x53B8 Ships, +0x53BC Buildings, +0x53CC Defenses (binding RESOLVED v2; old-doc Infantry@+0x53AC REFUTED).
- **Per-RTTI factory counts** +0x5378..+0x5388 (feeds MultipleFactory).
- **Purifier base:** `OrePurifierCount` +0x538C (±1 per OrePurifier building — v1 "StorageCapacity bales" REFUTED).
- **Identity/diplomacy:** HouseIndex +0x30, Type (HouseTypeClass*) +0x34, TechLevel +0x1D4, SideIndex +0x1E8, IsHuman +0x1EC, PlayerControl +0x1ED, `Allies` directional bitmask +0x5788 (`1<<(idx&0x1f)`); +0x1D8 = editor-only self-mask.
- **Lifecycle:** IsDefeated +0x1F5, scatter-pending +0x1F6, HasWon +0x1F7, HasLost +0x1F8; superweapon-manage dirty tail +0x1FC.
- **Owned-object accounting:** OwnedObjects +0x6C / count +0x78; OwnedBuildings +0x2F0.

### Globals owned/registered
- `g_FactoryClass_Array` @0x00A83E34 (count @0x00A83E40) — every FactoryClass self-registers in ctor, shift-left-removes in dtor. Iterated by PerTickUpdate and RecalcAllRates.
- `g_HouseClass_Array` @0x00A8022C (count @0x00A80238) — every HouseClass registers in ctor (HouseIndex = count at registration).
- Pending-building placement-ghost globals: `0x00B0FE5C` (regular building) / `0x00B0FE60` (defense). (v1 "pending land/naval vehicle" REFUTED — these are buildings, set by FUN_00734250 in StripClass case RTTI 6.)

---

## Key functions & globals (addresses)

### FactoryClass
- `FactoryClass::AI` (per-tick stepper) **0x004C9B20**, vtable +0x5C — advance by Step on timer expiry; per-step charge `⌊Balance/(54−Value)⌋`; OnHold + rewind on shortfall; settle on completion.
- Constructor **0x004C98F0**; ScalarDeletingDestructor **0x004CA790**.
- StartProduction **0x004C9C70** (create object via type vtable+0x8C, Balance=full GetCost; or append to queue capped at Rules+0xF0).
- Suspend **0x004C9E60**; SetRate **0x004C9EA0** (rate = GetBuildStepTime/0x36 [÷54 magic 0x4BDA12F7 + SAR 4], clamp [1,255]); CalcRate **0x004C9FB0**.
- AbandonProduction (cancel/refund) **0x004C9FF0** (refund `GetCost(Owner) − Balance`; 0x004CA0E0 is an interior address, NOT a second entry).
- GetProgress **0x004CA120**; IsComplete **0x004CA130**; GetObject **0x004CA160**; CompletedProduction **0x004CA1A0** (clears object, does NOT start next).
- StartNextQueued **0x004CA5A0** (front-pop); RemoveFromQueue **0x004CA620** (first-match front-to-back).
- RecalcAllRates **0x004CA6E0** (rewrites +0x38 for same-house factories on power flip).
- GetBuildStepTime **0x006F47A0** (`this` = object under construction; truncation order, no ×0.9, per-iteration MultipleFactory truncation, BuildSpeed double in RTTI==6 wall branch only; ÷54+clamp happens in callers).

### HouseClass
- Constructor **0x004F54A0**; Update (per-frame tick) **0x004F8440**, vtable +0x5C / slot 23.
- Begin_Production **0x004FA350**; Place_Production **0x004FB0E0** (delivery commit, sole caller EventClass::Execute).
- Add_Credits **0x004F9950** (`+0x30C +=`); Spend_Money **0x004F9790** (silo-drain fallback); Add_Tiberium_Credits **0x004F9610** (Balance += trunc(TibValue×IncomeMult×amount); HarvestedCredits += trunc(amount×5.0)).
- DepositOreFromStorage **0x00522D50**; DepositWeedCredits **0x004F9700** (weed only, caps at TiberiumStorageLimit).
- CanBuild **0x004F7870** (prereq tokens −1..−6, TechLevel, Required/Forbidden bitmask, BuildLimit).
- GetPowerRatio **0x004FCE30** (reads +0x53A4/+0x53A8); AI_AssessPower **0x00508C30** (recompute power, occupied-reactor zeroing, RecalcAllRates).
- GetFactoryCount **0x00500910**; Find_Factory **0x004F83C0**.
- Set_Credits_And_Color **0x004FCE00** (MP credit init); Read_Scenario_INI **0x00500B40** (campaign credit init).
- IsAlliedWith **0x004F9A50**; MakeAlly **0x004F9B70**; BreakAlliance **0x004F9F90**.
- MPlayer_Defeated **0x004FC0B0**; Flag_To_Win **0x004FC9E0**; Flag_To_Lose **0x004FCBD0**; Create_Houses **0x00687F10**.

### Global plumbing
- `LogicClass::PerTickUpdate` **0x0055AFB0** (tick spine; factory loop 0x55b66a..b68b before house loop 0x55b68d..b6b1).
- EventClass::Execute **0x004C6CB0** (lockstep command dispatch; sole caller of Place_Production); FUN_004FAA10 **0x004FAA10** (queue restart / cancel routing by heapId).
- StripClass::AI **0x006A8B30** (sidebar delivery/flash; unit/aircraft/infantry → 0x0B Place event, building → FUN_00734250).
- Prereq revalidation **0x00509140** (3-way drop/suspend/resume; callers GoOnline/GoOffline/Limbo/Unlimbo/ReadFromINI).
- RTTI_To_TypeArray **0x0048DCD0**; Main_Tick **0x0055D360** (late g_CurrentFrameCounter++ @0x00A8ED84).

---

## Tick / render position

In `LogicClass::PerTickUpdate` (the tick spine), this service occupies **two sequential global loops, factories first**:
1. Walk `g_FactoryClass_Array` → `FactoryClass::AI` (vtable +0x5C) on each — the production step (charge + advance + stall/settle). No null-check per slot.
2. Walk `g_HouseClass_Array` → `HouseClass::Update` (vtable +0x5C, slot 23) — power/radar recheck, SW-ready, defeat detect, AI choosers (8-frame), superweapon manage/resume tail. Null-checks each slot. Does **NOT** step factories.

`g_CurrentFrameCounter` advances **late**, in `Main_Tick` after the full logic pass (pause/desync-gated). Delivery (Place_Production) is **command-bound**, dispatched out of EventClass::Execute (event 0x0B), not the completion tick — the queue does not advance until delivery succeeds. The render/sidebar position is StripClass::AI (cameo flash, progress bar, auto-emit deliver event); `IsDifferent`/`HasChanged` is render-only, not hashed.

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| **logicclass** | `LogicClass::PerTickUpdate 0x0055AFB0` drives this service — walks `g_FactoryClass_Array` (FactoryClass::AI) then `g_HouseClass_Array` (HouseClass::Update). The factory step and house tick exist only because the LogicClass spine calls them in order. | G2/C1; `disassemble 0x0055AFB0` factory loop 0x55b66a..b68b before house loop 0x55b68d..b6b1 (V3). Late frame counter in Main_Tick 0x0055D360. |
| **abstract-object** | FactoryClass holds the produced `Object` +0x58 (a TechnoClass*); StartProduction creates it via the type's `Create` slot (type vtable+0x8C); Place_Production delivers via Unlimbo (+0xD8) / ExitObject (+0x100) / exit-target resolver (+0x190). Object lifecycle (create→limbo→unlimbo→delete) is the abstract/object layer. | F7; `decompile 0x004C9C70` (create), `disassemble 0x004FB0E0` (Unlimbo/ExitObject slots, V5). |
| **techno-foot** | The cost/create/CanBuild chain runs through `TechnoTypeClass` slots: GetCost (type +0x84/+0x88) seeds Balance and the per-step cost basis; Create (+0x8C) makes the produced object; the prereq gate is `type vtable+0x94`. GetBuildStepTime's `this` is the object-under-construction (a TechnoClass), reading per-type BuildTimeMultiplier (type+0x608). War-factory exit establishes a radio link from producing building to exiting vehicle (vehicle = FootClass). | §2g type slots; `decompile 0x004FA350` (gate +0x94); `disassemble 0x006F47A0` (type+0x608, R1); C13b exit link `0x00443c60` (V5). |
| **mission-radio** | Place_Production sends radio **0x0C** to the just-placed building (Receive_Radio 0x0043c2d0 case 0xc → set mission 5/Guard + grand-opening); war-factory exit establishes a radio link (0x02/+0x18/0x09) broken when the vehicle clears the footprint. Building-online sets MissionClass state. | C13a/C13b; `disasm 0x004fb2a9`, `decompile 0x0043c2d0` / `0x00443c60` (V5). |
| **rules-class** | Pervasive reads of parsed INI tunables: MaximumQueuedObjects (Rules+0xF0, queue cap), build-time floats Min/Max/LowPowerPenaltyModifier (+0x570/+0x574/+0x578), MultipleFactory (+0x57C), BuildSpeed double (+0x758), PurifierBonus (+0xf3c), AIVirtualPurifiers ptr (+0x1324), prereq token Rules offsets (+0x35C..+0x400), Difficulty starting-credit bonus (+0xDFC/+0xE00), TiberiumStorageLimit (+0x17D0). | H4/H5, C10/C11; `disassemble 0x006F47A0` (R1); CanBuild token table §2f; `decompile 0x00522D50` (PurifierBonus@Rules+0xf3c, R4). |
| **cell-map** | Place_Production resolves the delivery cell / exit target via the produced object's exit-resolver (+0x190) and Unlimbo onto the map; building factory category split tests BuildingTypeClass+0xE08 (defense). Delivered objects occupy cells; pending-building ghost globals stage placement. | §2g produced +0x190 exit-target/cell resolver (V5); §2b naval/defense split. |
| **ini-parsing** | House creation reads per-house values via CCINIClass accessors: Read_Scenario_INI (campaign) reads Credits/TechLevel/PlayerControl/IQ/Edge/Color/Allies through ReadInt/ReadBool; HouseTypeClass ReadINI (0x00511850) reads Side/Color. (MP path reads lobby globals, not INI.) | §2b.1 INI→offset map; `disassemble 0x00500B40` (R3/E1); ReadINI 0x00511850. |
| **random-scenario** | Lifecycle effects gate on ScenarioClass flags: Clear_Rally (Scenario&0x10), destroy-owned-units (g_ScenarioClass&0x800), borrowed-time/defeat gate on `g_GameMode`. Begin_Production reads g_PlayerPtr (player house) for sidebar side effects. | H9; `decompile 0x004FC0B0` (E2); §2d g_GameMode. |
| **drawing-helpers** | (render-only, indirect) StripClass::AI drives cameo flash / progress-bar draw and reads the factory's `IsDifferent`/HasChanged dirty flag; pending-building ghost placement is rendered. Not a sim edge — render layer reads FactoryView. | F11 (render-only); StripClass::AI 0x006A8B30 (V5). |

Notes:
- **cell-validation / bridge-helpers / pathfinding-helpers / target-scoring / damage-helpers / lookup-tables:** no direct edge found in the production/economy path (exit-object cell resolution is delegated through abstract-object / cell-map, not validated here).
- **gadget-dialog / shell-dialog:** the sidebar (StripClass) is the in-game gadget tree consumer; this service exposes read-only views to it (an *incoming* render dependency), not an outgoing call.

## Used-by (incoming edges)

| Source slug | Via symbol / field | Evidence |
|---|---|---|
| **logicclass** | PerTickUpdate is the caller: it dispatches FactoryClass::AI and HouseClass::Update every tick. (Reciprocal of the depends-on edge — LogicClass owns the order; this service supplies the two walked arrays.) | G2/C1, `disassemble 0x0055AFB0` (V3). |
| **techno-foot** | Buildings/units call into HouseClass for power accounting (AI_AssessPower sums PowerOutput/Drain over owned buildings), factory-count (GetFactoryCount), prereq revalidation on Limbo/Unlimbo/GoOnline/GoOffline (0x00509140 callers), and credit award on ore deposit (Add_Tiberium_Credits from the miner/harvester). OrePurifierCount ±1 on building OnConstructionComplete/Limbo. | H3; revalidation callers `get_function_callers 0x00509140` (V4); DepositOreFromStorage 0x00522D50 from miner path (R4). |
| **mission-radio** | Mission state machine consumes the building-online radio (0x0C) and exit-link (0x02) handshakes that Place_Production / ExitObject initiate; defeat lifecycle drives Clear_Rally. | C13a/C13b (V5). |
| **abstract-object** | The produced object's lifecycle (Unlimbo/ExitObject/delete) is invoked from Place_Production / AbandonProduction; the object is owned/attached by the factory until delivery clears it. | F7/C12 (V5). |
| **gadget-dialog** (sidebar / StripClass) | The in-game sidebar reads factory progress / on-hold / queue / ready-list (FactoryView), polls HasChanged (0x004C9C60) per cameo, and auto-emits the 0x0B Place command via EventClass. Command path: player clicks → EventClass::Execute → Begin/Suspend/Cancel/Place. | C20, StripClass::AI 0x006A8B30, EventClass::Execute 0x004C6CB0 (V5). |
| **random-scenario** | (weak) ScenarioClass-gated lifecycle effects read here; House creation pipeline (Create_Houses / Read_Scenario_INI) is invoked from Full_Init by g_GameMode routing. | H11; Full_Init 0x00686b20 routing (E1). |
| **frontier-ai** (deferred) | AI house production (AI_Manage_Build_Queue, AI choosers in Update's 8-frame cadence, AI virtual purifiers, AI build-progress headstart) calls FactoryRegistry::begin equivalents. Classified DEFERRED-AI; clean seam, not designed. | §3 DEFERRED-AI; Update 0x004F8440 AI gate. |

---

## Open / unverified edges

- **AIVirtualPurifiers index-field identity** — Rules+0x1324 offset VERIFIED-LIVE (R4); whether the index `house+0x184` is the AI-difficulty field is UNVERIFIED. Gates the AI economy edge (frontier-ai → factory-house) only.
- **SpecialItem (+0x68) writer / SW-begin path** — not located; value 0 cannot be proven unreachable, so the 0-vs-−1 convention must not be collapsed. Affects the superweapon edge (out of this substrate's core scope).
- **War-factory exit-link BREAK radio code** — establish is 0x02 (mission-radio edge confirmed); the break code firing on footprint-clear was not isolated (UNCHECKED).
- **read==write-wallet equivalence** — FactoryClass::AI affordability read goes via the +0x24 credit sub-object's vtable+0x18 slot; Spend_Money writes +0x30C. That both reference the same wallet word is asserted (H1) but the +0x18 read-slot target was not decompiled (UNCHECKED).
- **abstract-object exit-cell edge** — the produced +0x190 exit-target/cell resolver is confirmed called `(0,0)/(0,1)` but whether it touches cell-validation/pathfinding internally was not traced from this service.
