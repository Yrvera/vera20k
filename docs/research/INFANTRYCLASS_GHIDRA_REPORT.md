# InfantryClass — Ghidra Research Report

**Primary addresses:** Constructor `0x00517A50`, AI `0x0051BAB0`, DoType sequencer `0x00520AE0`, Do_Action `0x0051D6F0`, Scatter `0x0051D0D0`
**Confidence:** HIGH (all findings verified from binary decompilation)
**Active in YR:** Yes — all systems documented here are active in standard YR skirmish

## 1. Overview

InfantryClass is the leaf class for foot soldiers, inheriting from FootClass (→ TechnoClass → ObjectClass → AbstractClass). It adds ~48 bytes (0x6C0–0x6E8) for infantry-specific state: the type pointer, animation sequence (DoType), fear level, prone/crawling flags, and sub-cell index. The class owns a complex animation state machine driven by DoType indices, a health-aware fear/panic system, and water/swim sequence remapping.

**Total instance size:** 0x6F0 (1776 bytes).

## 2. InfantryClass Struct Layout (0x6C0–0x6E8)

All offsets verified from constructor at `0x00517A50` (param_1 is `undefined4*`, so `param_1[N]` = byte offset N×4).

| Byte Offset | Size | Init Value | Field Name | Evidence / Purpose |
|-------------|------|------------|------------|--------------------|
| `0x6C0` | 4 | param (TypeClass*) | **InfantryTypeClass*** | Constructor stores second parameter; used everywhere as `this->Type` |
| `0x6C4` | 4 | -1 (0xFFFFFFFF) | **DoType / SequenceIndex** | Current animation sequence ID. -1 = none active. Compared extensively in AI, Scatter, Do_Action, sequencer |
| `0x6C8` | 4 | g_CurrentFrameCounter | **FearTimestamp** | Frame counter snapshot for fear timer. Compared against g_CurrentFrameCounter in AI fear decay logic |
| `0x6CC` | 4 | (implicit 0) | *(gap — part of 0x6C8 int or padding)* | Not explicitly initialized in constructor |
| `0x6D0` | 4 | 0 | **FearDuration** | Duration parameter for fear timer. Used in AI: `if (FearTimestamp != -1) { elapsed = g_Frame - FearTimestamp; remaining = Duration - elapsed; }` |
| `0x6D4` | 4 | 0 | **FearLevel** | Fear counter (0–300). Decrements by 1 per tick. Controls prone, crawl, and panic behavior. See §4 |
| `0x6D8` | 1 | 0 | **Unknown_6D8** | Byte flag, purpose not yet traced |
| `0x6D9` | 1 | 0 | **Unknown_6D9** | Byte flag, purpose not yet traced |
| `0x6DA` | 1 | 0 | **ShouldStandUp** | Checked in AI fear-timer block: when set and fear timer expires, cleared to 0. Related to timed prone state |
| `0x6DB` | 1 | 0 | **IsCrawling** | Set to 1 by Do_Action(5=Down). Cleared by Do_Action(7=Up) or Do_Action(0x1B). Controls sequence selection: crawl vs walk, fire-prone vs fire-standing |
| `0x6DC` | 1 | 0 | **Unknown_6DC** | Byte flag |
| `0x6DD` | 1 | 0 | **Unknown_6DD** | Byte flag |
| `0x6E0` | 4 | 0 | **Unknown_6E0** | Int field |
| `0x6E4` | 1 | 0 | **Unknown_6E4** | Byte flag |
| `0x6E8` | 4 | 2 | **SubCell** | Sub-cell index (0–4). Init to 2 (NE). Also reset to 2 in destructor. See INFANTRY_SUBCELL_POSITIONING.md |

### Related parent-class fields used by infantry logic

| Offset | Class | Name | Notes |
|--------|-------|------|-------|
| `0x2A4` | TechnoClass | **IsProne** | Set to 1 when crawl-down anim (seq 0x1B) completes; cleared when stand-up anim (seq 0x1F) completes. Gated by `NOT DeployedCrushable` (type+0xEC9 == 0): IsProne is only updated when field `0xEC9` is zero. (corrected 2026-05-28: was "Gated by Crawls flag at type+0xEC9"; binary sequencer checks `*(char *)(type + 0xec9) == '\0'`; `0xEC9` is DeployedCrushable per §7, not Crawls; via `decompile_function 0x00520AE0` — RTTI_LABEL_DRIFT) |
| `0x68D` | FootClass | **ShouldFireFromProne** | Set by various systems; cleared in AI and fire-at-target. Controls whether to use prone fire sequences |
| `0x674` | FootClass | **ILocomotion*** | Locomotor COM pointer — infantry uses WalkLocomotionClass |

## 3. InfantryClass::AI (`0x0051BAB0`, 174 lines)

Called every tick. Sequence of operations:

### 3a. Early exit / limbo check
- If `FootClass::DriveTrackIndex` bit 7 is **clear** (byte at `0x684`): calls FUN_0051B350 (tube/tunnel movement handler), then calls `vtable+0x4A0` and returns. (corrected 2026-05-28: was "bit 7 is set"; binary shows `if (-1 < (char)param_1[0x1a1])` which is true when the signed-byte value ≥ 0, i.e. bit 7 = 0; via `decompile_function 0x0051BAB0` — OPERATOR_OR_ORDER_DRIFT)

### 3b. Falling / in-air handling
- If unit is falling (`vtable+0x1D4` or `vtable+0x1D8` returns true): every 24 frames, spawns a parachute anim from `RulesClass+0x344`.
- If unit has a transport target (`param_1[0x9E]` non-null): calls ILocomotion::Process.

### 3c. Death in-air
- If falling and has attached weapon target: clears it.
- If falling and NavCom is active: assigns mission and returns.

### 3d. Warping check
- If unit is warping in (`vtable+0x200`): handles mission transitions.

### 3e. Sequence sanity check
Critical block: if `Health < 1` (dying) AND current DoType is NOT one of the exempt sequences, forces DoType to 1:
```
Exempt sequences: 0x0B–0x0F (Die1–Die5), 0x22–0x24 (Deploy/Deployed),
                   0x14–0x15 (WetIdle/WetDie)
```

### 3f. FootClass::AI call
Calls parent `FootClass::AI()` which handles locomotor processing, movement counters, team AI, idle scatter, etc.

### 3g. Garrison building check
If unit is alive, not selected (`0x81 == 0`), and mission is Guard(5) or Sleep(0xB):
- Gets cell at unit location, checks for buildings.
- If building is CanBeOccupied and has room, and unit is not already garrisoning: triggers scatter away or enter.

### 3h. Prone continuation
If `ShouldFireFromProne` (0x68D) is set and animation counter is 0:
- Clears the flag.
- If in prone sequences (0x1B–0x1E): calls `Do_Action(0x1C, 0, 0)` (continue crawling).
- Else: calls `Do_Action(0, 0, 0)` (return to Ready/Stand).

### 3i. Passenger deploy check
If unit has passenger (`0x175`) and `0x3D5` flag is set: marks passenger byte `0x82` = 1.

### 3j. Landed unit mission check
If no passenger and mission is Guard(5): checks terrain passability and handles destruction if invalid.

### 3k. Capture mission
Calls `InfantryClass__Mission_Capture()`.

### 3l. Fear handler
Calls `FUN_005200b0()` — the fear decay/prone/panic handler (see §4).

### 3m. Fear timer check
Checks fields at 0x6DA (ShouldStandUp) and the fear timer (0x6C8 + 0x6D0). When timer expires, clears ShouldStandUp.

### 3n. Fire-at-target
Calls `FUN_005206b0()` — combat firing logic and target acquisition.

### 3o. Animation sequencer
Calls `FUN_00520AE0()` — the DoType state machine.

### 3p. Locomotion AI
Calls `FootClass__Locomotion_AI()`.

## 4. Fear System

### 4a. FearLevel field (0x6D4)
Integer value 0–300. Initialized to 0. Decrements by 1 each tick (unless Fearless). Maximum value clamped to 300.

### 4b. Fear application — who sets FearLevel

**Source 1: Damage/scatter events** (code at ~0x518C00, part of PerCellProcess or scatter chain)

Three paths based on unit type:

| Condition | FearLevel Set To |
|-----------|-----------------|
| **Fraidycat** (InfantryTypeClass+0xEBF) | 300 (instant max) |
| **Fearless** (InfantryTypeClass+0xEBC) OR weapon ability 0xD | Immune — no change |
| **Normal infantry** | Incremented by health-dependent amount (see below), clamped to 300 |

Normal infantry fear increment per damage event:

| Health State | Increment |
|-------------|-----------|
| Red (below ConditionRed threshold) | +50 |
| Yellow (between Red and Yellow thresholds) | +25 |
| Green (above ConditionYellow threshold) | +12 |

If the infantry successfully scattered and FearLevel was < 100: FearLevel is set to 100 instead of the incremental formula.

**Source 2: Fraidycat firing** (`0x0051DF70` — InfantryClass::Fire_At override)

After a Fraidycat unit fires its weapon: FearLevel = 300 **if** bullet spawned AND unit is not player-selected (`0x81 == 0`) AND runtime Sight field (`0x2FC`) is 0. (corrected 2026-05-28: was "FearLevel = 300 immediately" with no conditions; binary adds three guards before setting fear=300; via `decompile_function 0x0051DF70` — INFERENCE_HARDENED)

**Source 3: vtable+0x518 panic trigger** (`0x00521C10`)

Direct FearLevel = 300 if not Fearless and not weapon ability 0xD. Called via vtable dispatch.

### 4c. Fear decay — FUN_005200b0

Called every tick from InfantryClass::AI:

```
if FearLevel > 0:
    if NOT Fearless:
        FearLevel -= 1
    
    if FearLevel == 0 AND param_1[0x2FC] == 0:
        restore sight range to type default (type+0x684)
        // 0x2FC is the runtime Sight field, initialized from type+0x680 (or +0x684 if -1) in InitFromType
        // (corrected 2026-05-28: was "mission == 0"; binary: `if (param_1[0xbf] == 0)` where 0xbf*4=0x2FC; via `decompile_function 0x005200B0` — INFERENCE_HARDENED)
    
    if NOT IsCrawling (0x6DB == 0):
        if FearLevel > 49 AND NOT in prone seqs AND NOT Fraidycat:
            // Player-controlled: skip if moving or has NavCom
            Do_Action(5, 0, 0)   // Go prone (Down sequence)
        // Note: Fearless only gates the decrement above; the go-prone guard is NOT Fraidycat
        // (corrected 2026-05-28: was "NOT Fearless"; binary: `if (type+0xebf == '\0')` before Do_Action(5) where 0xEBF=Fraidycat; via `decompile_function 0x005200B0` — RTTI_LABEL_DRIFT)
    
    else (IsCrawling):
        if FearLevel < 50 AND NOT in prone seqs:
            Do_Action(7, 0, 0)   // Stand up (Up sequence)
    
    if Fraidycat AND FearLevel > 50 AND NOT in prone seqs:
        if NOT moving AND no NavCom:
            Scatter()             // Panic flee
```

### 4d. Key thresholds

| Threshold | Value | Behavior |
|-----------|-------|----------|
| Go prone | FearLevel > 49 | Infantry goes Down (sequence 5) |
| Stand up | FearLevel < 50 | Infantry goes Up (sequence 7) |
| Panic scatter | FearLevel > 50 | Fraidycat units flee |
| Panic walk anim | FearLevel > 199 | Walk sequence remapped to 0x25 (Panic) in Do_Action |

## 5. DoType / Animation Sequence System

### 5a. Sequence ID table

The InfantryTypeClass stores a pointer at `type+0xE3C` to a SequenceTypeClass, which is an array of 0x24-byte (36-byte) entries indexed by sequence ID.

Each entry contains:
| Entry Offset | Purpose |
|-------------|---------|
| +0x00 | Unknown |
| +0x04 | Frame count (total frames in sequence) |
| +0x0C | Facing direction value (for facing alignment) |
| +0x10 | Sound attachment count |
| +0x14 | Sound timing offset |
| +0x18+ | Sound data (stride 8 per sound) |

### 5b. Sequence IDs (verified from Do_Action and sequencer)

| ID | Dec | Name | Notes |
|----|-----|------|-------|
| 0x00 | 0 | **Ready** | Idle standing |
| 0x01 | 1 | **Guard** | Guard/alert standing |
| 0x02 | 2 | **Prone** | Prone idle |
| 0x03 | 3 | **Walk** | Walking movement |
| 0x04 | 4 | **FireUp** | Standing fire (primary weapon) |
| 0x05 | 5 | **Down** | Transition: standing → prone. Sets IsCrawling=1 |
| 0x06 | 6 | **Crawl** | Crawling movement |
| 0x07 | 7 | **Up** | Transition: prone → standing. Clears IsCrawling=0 |
| 0x08 | 8 | **FireProne** | Prone fire (primary weapon) |
| 0x09 | 9 | **Idle1** | Random idle animation 1 |
| 0x0A | 10 | **Idle2** | Random idle animation 2 |
| 0x0B | 11 | **Die1** | Death animation 1 |
| 0x0C | 12 | **Die2** | Death animation 2 |
| 0x0D | 13 | **Die3** | Death animation 3 |
| 0x0E | 14 | **Die4** | Death animation 4 |
| 0x0F | 15 | **Die5** | Death animation 5 |
| 0x10 | 16 | **Tread** | Water idle (land→water transition) |
| 0x11 | 17 | **Swim** | Swimming movement |
| 0x12 | 18 | **WetAttack** | Fire while swimming |
| 0x13 | 19 | **WetIdle1** | Swim idle 1 |
| 0x14 | 20 | **WetIdle2** | Swim idle 2 |
| 0x15 | 21 | **WetDie1** | Death in water 1 |
| 0x16 | 22 | **WetDie2** | Death in water 2 |
| 0x17 | 23 | **Cheer/Paradrop** | Airborne→deployed or cheer |
| 0x1B | 27 | **CrawlDown** | Internal: crawl-down complete → switch to 0x1C; sets TechnoClass::IsProne=1 |
| 0x1C | 28 | **Crawling** | Active crawl/prone movement |
| 0x1D | 29 | **ProneFire** | Fire while in prone transitions |
| 0x1E | 30 | **ProneSecondary** | Alternate prone fire |
| 0x1F | 31 | **CrawlUp** | Internal: stand-up complete → switch to 0; clears TechnoClass::IsProne=0 |
| 0x20 | 32 | **Unknown_32** | Possibly deploy-related |
| 0x21 | 33 | **Paradrop** | Parachute descent |
| 0x22 | 34 | **Deploy** | Deploy transition → switch to 0x23 |
| 0x23 | 35 | **Deployed** | Deployed idle |
| 0x24 | 36 | **DeployedFire** | Fire while deployed (held) |
| 0x25 | 37 | **Panic** | Panic walk (FearLevel > 199) |
| 0x26 | 38 | **Undeploy** | Undeploy transition |
| 0x27 | 39 | **AltWalk** | Alternate walk (e.g., on bridge type 0x800) |
| 0x28 | 40 | **SecondaryFire** | Secondary weapon fire (standing) |
| 0x29 | 41 | **SecondaryProne** | Secondary weapon fire (prone) |

### 5c. Do_Action (`0x0051D6F0`, vtable+0x558)

**Signature:** `Do_Action(int sequenceId, bool force, bool randomStartFrame)`

Key behaviors:
- Returns 0 if sequence has 0 frame count (stub/unused).
- If current sequence is 0x21 (Paradrop) and dying: refuses to change.
- **Swim remapping:** When `InfantryTypeClass+0x5B4 == 3` (SpeedType = water-capable), sequences are remapped:
  - Walk(3)/Crawl(6) → Swim(0x11)
  - Ready(0)/Prone(2) → Tread(0x10)
  - Idle1(9) → WetIdle1(0x12); Idle2(10) → WetIdle2(0x13)
  - Die1(0xB) → WetDie1(0x14); Die2(0xC) → WetDie2(0x15)
  - FireUp(4)/FireProne(8) → WetAttack(0x16)
  - Plays splash sound when transitioning land ↔ water. The field at `0x6E8` (param_1[0x1BA]) is repurposed here as a land/water state tracker (0 = in water, 1 = on land); it is initialized to 2 (NE SubCell) and reset to 2 in the destructor, but during active swim transitions this field holds the water-state flag. (clarified 2026-05-28: was just "SubCell stored as 0/1 flag at 0x1BA" which conflicted with §2's SubCell label; via `decompile_function 0x0051D6F0` — MISLEADING)
- **Panic walk:** Walk(3) remapped to Panic(0x25) when FearLevel > 199.
- **Bridge/alt walk:** Walk(3) remapped to AltWalk(0x27) when on a bridge structure (0x800 flag).
- **Interruptibility check:** Uses `g_MissionScatterTable` at `0x7EAF7C` — per-sequence byte table determining if a sequence can be interrupted.
- **IsCrawling flag (0x6DB):**
  - Set to 1 when sequence = 5 (Down)
  - Cleared to 0 when sequence = 7 (Up) or 0x1B
- **Prone transition sounds:** Plays sounds from SequenceTypeClass offsets 0x56C (Down) and 0x570 (Up).
- **Random start frame:** If `randomStartFrame != 0`, picks a random frame within the sequence instead of starting at 0.

### 5d. Animation Sequencer (`0x00520AE0`)

Called every tick from AI. Drives the DoType state machine.

**Frame timing:** Checks `param_1[0x3E]` (current frame) against SequenceEntry+0x04 (total frames). When frame count is reached, the sequence transitions.

**Key transitions on completion:**

| Completing Sequence | Transition |
|---|---|
| 0x0B–0x0F (Die) | Plays death anim (random from type's lists), then calls `vtable+0xF8` (Destroy) |
| 0x1B (CrawlDown) | → Do_Action(0x1C, 1, 0) + sets IsProne=1 if **NOT** DeployedCrushable (type+0xEC9 == 0). (corrected 2026-05-28: was "if Crawls flag"; binary: `if (type+0xec9 == '\0')` sets IsProne; via `decompile_function 0x00520AE0` — RTTI_LABEL_DRIFT) |
| 0x1F (CrawlUp) | → Do_Action(0, 1, 0) + clears IsProne=0 if **NOT** DeployedCrushable (type+0xEC9 == 0). (corrected 2026-05-28: was "if Crawls flag"; same binary pattern; via `decompile_function 0x00520AE0` — RTTI_LABEL_DRIFT) |
| 0x22 (Deploy) | → Do_Action(0x23, 1, 0) (Deployed) |
| 0x26 (Undeploy) | If mission==Guard(10): re-trigger Undeploy |
| Default (moving) | If IsCrawling: Do_Action(6=Crawl); else Do_Action(3=Walk) |
| Default (stationary) | If IsCrawling: Do_Action(2=Prone); else Do_Action(0=Ready) |

**Attack sequence (0x28/0x29):** When unit has target and locomotor is not moving:
- Checks IsCrawling flag to choose between standing fire (0x28) and prone fire (0x29).

**Sound playback:** At the end, iterates the sequence entry's sound list: for each attached sound, if `(currentFrame % totalFrames) == soundTriggerFrame`, plays the sound at the unit's location.

## 6. Fire-At-Target Logic (`0x005206b0`)

Called from InfantryClass::AI when the unit has a Target.

### Weapon selection and sequence mapping

| Weapon | IsCrawling=0 | IsCrawling=1 |
|--------|-------------|-------------|
| Primary (weapon index 0) | Do_Action(4, 0) = FireUp | Do_Action(8, 0) = FireProne |
| Secondary (weapon index ≠0) | Do_Action(0x28, 0) = SecondaryFire | Do_Action(0x29, 0) = SecondaryProne |

Secondary weapon selection falls back to primary if the sequence data doesn't define secondary animations (checked via SequenceTypeClass offsets 0x5A4 and 0x5C8).

### ROF timing
Uses `InfantryTypeClass+0xE40/0xE44/0xE48/0xE4C` for weapon ROF values depending on weapon index and prone state. After firing completes, checks if another shot is needed.

### Post-fire scatter
If target is alive and weapon's minimum range (`WeaponTypeClass+0xA8`) < `RulesClass+0x16C0`: scatters objects at the target cell.

## 7. InfantryTypeClass Key Offsets

| Offset | Type | INI Key | Notes |
|--------|------|---------|-------|
| `0x5B4` | int | SpeedType | 3 = water-capable (swim sequences) |
| `0x678` | int | Speed | Movement speed |
| `0x680` | int | Sight (YR override) | -1 = use fallback at 0x684 |
| `0x684` | int | Sight (base) | Sight range in cells |
| `0x9C` | int | Armor | Armor type index |
| `0xA0` | int | Strength | Max HP |
| `0xC8E` | byte | Trainable | Vet eligible |
| `0xCD0` | byte | Unknown | Copied to TechnoClass+0x3D2 in InitFromType |
| `0xE04` | ptr | OccupyWeapon | Weapon when garrisoned |
| `0xE20` | ptr | EliteOccupyWeapon | Elite garrison weapon |
| `0xE3C` | ptr | **SequenceTypeClass*** | Pointer to sequence data table |
| `0xE40` | int | PrimaryROF (standing) | |
| `0xE44` | int | SecondaryROF (standing) | |
| `0xE48` | int | PrimaryROF (secondary weapon) | |
| `0xE4C` | int | SecondaryROF (prone secondary) | |
| `0xE54` | ptr | DeathAnims (array) | For prone death |
| `0xE60` | int | DeathAnims count | |
| `0xEAD` | byte | Unknown | Checked in death anim: if set, uses global death anims |
| `0xEB4` | byte | Occupier | Can garrison buildings |
| `0xEB5` | byte | Assaulter | Can assault enemy buildings |
| `0xEBC` | byte | **Fearless** | Immune to fear/panic |
| `0xEBD` | byte | Crawls | Has prone/crawl animations |
| `0xEBE` | byte | Infiltrate | |
| `0xEBF` | byte | **Fraidycat** | Panics easily; max fear on damage |
| `0xEC3` | byte | Unknown | Checked in scatter-fear: if set + AI-controlled + Guard/Sleep → assign Hunt mission |
| `0xEC9` | byte | DeployedCrushable | Also gates IsProne set/clear in sequencer |

## 8. Integration Points

### What calls InfantryClass::AI
- `LogicClass::PerTickUpdate @ 0x0055AFB0` contains the per-object active-vector loop; it iterates the LogicClass-owned object vector forward and calls vtable+0x5C, re-reading count after each call. `LogicClass::AI` is the input/event dispatcher, not this object-AI loop.

### What InfantryClass::AI calls
1. FUN_0051B350 — tube/tunnel movement
2. FootClass::AI — parent class tick (locomotor, movement, team, idle scatter)
3. FUN_005200b0 — fear decay/prone/panic handler
4. FUN_005206b0 — fire-at-target acquisition and attack sequencing
5. FUN_00520AE0 — DoType animation sequencer
6. InfantryClass__Mission_Capture — capture building logic
7. FootClass__Locomotion_AI — additional locomotor processing

### When fear is applied
- From damage/scatter pipeline (called during Apply_area_damage → ReceiveDamage chain)
- From Fraidycat firing (InfantryClass::Fire_At override at 0x0051DF70)
- From vtable+0x518 direct panic trigger (0x00521C10)

## 9. Current Rust Implementation Status

### Implemented
- **InfantrySequenceRegistry**: Full INI sequence parsing (`src/rules/infantry_sequence.rs`)
- **SequenceKind enum**: All 33 sequence variants defined (`src/sim/animation.rs`)
- **Animation state machine**: Frame advancement, auto-transitions Stand↔Walk, Attack↔Stand, death sequences
- **Sub-cell positioning**: Full 5-cell system with preference tables (`src/util/lepton.rs`, `src/sim/movement/bump_crush.rs`)
- **Occupancy grid**: Infantry sub-cell tracking (`src/sim/occupancy.rs`)
- **Garrison INI parsing**: OccupyWeapon, Occupier, Assaulter parsed
- **InfDeath → Die sequence mapping**: `death_sequence_for_inf_death()` in animation.rs

### NOT Implemented
- **Fear/Panic system**: No FearLevel field, no prone trigger from damage, no Fraidycat/Fearless behavior, no fear decay. `SequenceKind::Panic` exists but is never entered.
- **Prone/Crawl entry from gameplay**: Sequences are defined but sim never enters Down/Crawl from live events (marked TODO(RE) in animation.rs).
- **IsCrawling flag**: No equivalent of the 0x6DB flag to choose between standing/prone variants.
- **Swim/water sequence remapping**: No Do_Action equivalent that remaps Walk→Swim based on terrain.
- **Panic walk (FearLevel > 199)**: Walk→Panic remap not implemented.
- **Secondary weapon fire sequences**: SecondaryFire/SecondaryProne not wired.
- **Garrison combat**: Parsed but not firing (P0 gap per GARRISON_IMPLEMENTATION_PLAN.md).
- **WalkLocomotionClass 7-state machine**: Infantry shares generic GroundMovePhase with vehicles.

## 10. Open Questions

1. **Fields 0x6D8, 0x6D9, 0x6DC, 0x6DD, 0x6E0, 0x6E4**: Initialized to 0 but not yet traced to specific behavior. May be related to deploy state, swim state, or internal flags.
2. **InfantryClass::Draw_It**: Not decompiled. The vtable slot for Draw_It was not resolved. Need to read the vtable at the correct offset to find the draw function.
3. **InfantryClass::PerCellProcess**: Documented as `0x54C550` in subcell report but not decompiled in this session. Contains drowning, crate pickup, and scatter logic. The fear-setting code at ~0x518C00 may be part of or called from this function.
4. **Weapon ability 0xD**: Grants fear immunity. Need to identify which INI ability flag this maps to.
5. **SequenceTypeClass full layout**: Only offsets +0x04 (frame count), +0x0C (facing), +0x10 (sound count), +0x18 (sound data) are confirmed. The full 0x24-byte structure has gaps.
6. **InfantryClass::Load/Save** (`0x0051FB00`): Serialization format for sub-cell and DoType state not decompiled.
7. **g_MissionScatterTable** (`0x7EAF7C`): Per-sequence interruptibility table — contents not dumped.

## Sources

### Ghidra addresses decompiled
- `0x00517A50` — InfantryClass::Constructor (struct layout)
- `0x00517D90` — InfantryClass::Destructor (field resets)
- `0x00517CC0` — InfantryClass::InitFromType (type init)
- `0x0051BAB0` — InfantryClass::AI (full, 174 lines)
- `0x00520AE0` — DoType animation sequencer (full, 187 lines)
- `0x0051D6F0` — Do_Action (sequence switching, swim remap, fear/panic)
- `0x005200B0` — Fear decay/prone/panic handler
- `0x005206B0` — Fire-at-target handler (weapon selection, attack sequences)
- `0x0051CBA0` — InfantryClass::IdleDispatch
- `0x0051CDB0` — InfantryClass::UpdateIdleAction
- `0x0051D0D0` — InfantryClass::Scatter (full, 247 lines)
- `0x00521C10` — vtable+0x518 panic trigger (FearLevel = 300)
- `~0x00518C00` — Fear application (damage/scatter path, health-dependent increment)
- `0x0051DF70` — InfantryClass::Fire_At override (Fraidycat post-fire fear)
- `0x004D7330` — FootClass::ReceiveDamage (calls TechnoClass::ReceiveDamage)
- `0x00701900` — TechnoClass::ReceiveDamage (full, 682 lines)
- `0x005F5390` — ObjectClass::ReceiveDamage (full, 188 lines)
- `0x00489280` — Apply_area_damage (full, 529 lines)
- `0x00481670` — CellClass::Scatter_Objects

### Doc files referenced
- `INFANTRY_SUBCELL_POSITIONING.md` — sub-cell system (verified, extended)
- `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` — parent class layout
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` — parent class layout
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md` — InfantryClass::Scatter details
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` — garrison occupant mechanics
- `READINI_FIELD_MAPS.md` — InfantryTypeClass offsets

### INI files checked
- `ini/rulesmd.ini` — Fearless, Fraidycat, Occupier, InfDeath, ProneDamage
- `ini/artmd.ini` — Sequence definitions ([*Sequence] sections)
