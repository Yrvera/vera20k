# Multiplayer Defeat/Victory System — Ghidra Report

Source: `D:\ra2mdpost\House.CPP` (confirmed from debug strings in binary)
Binary: `gamemd.exe` (Yuri's Revenge)

---

## Table of Contents

1. [MPlayer_Defeated (0x4FC0B0)](#1-mplayer_defeated-0x4fc0b0)
2. [Destroy_All_Owned (0x4FB920)](#2-destroy_all_owned-0x4fb920)
3. [ScatterAllUnits (0x4FC6D0)](#3-scatterallunits-0x4fc6d0)
4. [Flag_To_Win_Check (0x4FC980)](#4-flag_to_win_check-0x4fc980)
5. [Flag_To_Win (0x4FC9E0)](#5-flag_to_win-0x4fc9e0)
6. [Flag_To_Lose (0x4FCBD0)](#6-flag_to_lose-0x4fcbd0)
7. [Check_Win_Condition (0x4FCDC0)](#7-check_win_condition-0x4fcdc0)
8. [Alliance AI Rearrangement (0x501640)](#8-alliance-ai-rearrangement-0x501640)
9. [Key Globals and Offsets](#9-key-globals-and-offsets)
10. [Implementation Notes](#10-implementation-notes)

---

## 1. MPlayer_Defeated (0x4FC0B0)

**Size:** 1559 bytes
**Signature:** `void __fastcall HouseClass::MPlayer_Defeated(HouseClass *this)`
**Source:** House.CPP

### Who calls it?

The function header in the static export has no "Called by" section, meaning it is called
via **indirect dispatch** (vtable or function pointer). Based on cross-references, the
callers are the game's defeat detection logic in `HouseClass::Update` (FUN_006dd8b0, the
9530-byte house update tick), which checks defeat conditions each frame and calls
MPlayer_Defeated when a house meets them. The defeat detection in Update checks whether
the house has lost all its buildings and units (or all its buildings of qualifying types),
then invokes MPlayer_Defeated.

### Complete flow, branch by branch

#### STEP 1: Set defeated flag

```c
this->IsDefeated = 1;   // offset +0x1F5
```

This is the very first thing. Unconditional. Once set, the house is marked as defeated
for all subsequent checks.

#### STEP 2: AI alliance rearrangement (conditional)

```c
if (this->IQLevel == Rules->MaxIQLevels) {  // +0x24C == Rules+0x1434
    // This house had max IQ (was a full AI player)
    bool isHumanLike;
    if (GameMode == 0) {  // campaign
        isHumanLike = (this->IsHuman || this->IsSpectator);  // +0x1EC || +0x1ED
    } else {
        isHumanLike = this->IsHuman;  // +0x1EC
    }

    if (!isHumanLike && Rules->HasSidebarPanels && GameMode != 0) {
        // +0x17E0 on Rules, DAT_00a8b238 != 0
        FUN_00501640();  // Rearrange_Alliances_For_AI_Defeat
    }
}
```

`FUN_00501640` (271 bytes) is the **AI alliance rearrangement** function. When a max-IQ
AI player is defeated in multiplayer, this function iterates all surviving non-defeated
houses. For each surviving AI house (IsHuman==false), it makes that AI ally with all
other surviving AI houses (`Make_Ally`) and break alliance with all surviving human
houses (`Break_Alliance`). This ensures the remaining AIs team up against humans after
one AI falls.

#### STEP 3: Clear rally point (SpecialFlags bit 4)

```c
if (*SpecialFlags & 0x10) {  // DAT_00a8b230, bit 4
    // Clear this house's rally point
    if (this->RallyPointObject == NULL) {  // +0x53DC
        if (this->RallyPointCell != InvalidCell) {  // +0x53E0 != DAT_00a8ef98
            cell_obj = CellClass::Get(this->RallyPointCell);  // +0x53E0
        } else {
            goto skip_rally_clear;
        }
    }
    HouseClass::Clear_Rally_Point_Object(this, cell_obj, 1);  // FUN_004fbe40
}
```

Bit 4 of SpecialFlags (0x10) controls this behavior. If set, the defeated house's rally
point is cleared.

#### STEP 4: Destroy garrisoned buildings (multiplayer + bit 11)

```c
if (GameMode != 0 && (*SpecialFlags & 0x800)) {  // HarvesterImmune flag? Actually bit 11
    for (int i = 0; i < BuildingClass::Array.Count; i++) {
        BuildingClass *bld = BuildingClass::Array[i];  // DAT_008b410c
        if (bld->Owner == this && bld->IsOccupied) {   // [0x87]==this, [0x24]!=0
            bld->vtable->EjectOccupants();  // vtable+0xF8
        }
    }
}
```

If the multiplayer game has SpecialFlags bit 11 (0x800) set, all buildings owned by the
defeated house that have garrison occupants get their occupants ejected.

#### STEP 5: Local player defeated path

```c
if (PlayerPtr == this) {  // DAT_00a83d4c == param_1
```

**This is the "I am defeated" path — the local player just lost.**

##### 5a. Delete beacons

```c
BeaconClass::DeleteAllBeaconsForPlayer(PlayerPtr->HouseIndex);
// FUN_00431410 — iterates 3 beacon slots for this player, deletes each
```

##### 5b. Close sidebar (if open)

```c
bVar8 = DAT_00884d2c != 0;  // sidebar-is-open flag
if (bVar8) {
    Sidebar::CollapseSidebar();  // FUN_006d1660 — closes the sidebar thumb
}
DAT_00824410 = bVar8;  // save "was sidebar open" for later restore
```

`FUN_006d1660` hides the main sidebar column, shows tooltip 0xF1, sets thumb state to
collapsed, calls full relayout, marks dirty, triggers redraw.

##### 5c. Disable input & UI

```c
FUN_00637a10();  // Disable all mouse input / selection
FUN_004ac960(0); // Set mouse cursor to disabled state (param=0 means "off")
```

`FUN_00637a10` iterates display class elements and disables input handling. The mouse
cursor is set to the "disabled" appearance.

##### 5d. Mark game as pending end

```c
DAT_00a8b538 = 1;  // "game ending" flag
```

##### 5e. Reveal entire map (clear shroud)

```c
FUN_00577f30(this);  // HouseClass::Reveal_Entire_Map
```

This is the "Map Is Clear" function referenced in the debug string
`"MPlayer_Defeated: frame %d, house id %d, MapIsClear set to true"`.
It reveals the entire map for the defeated player so they can spectate.

##### 5f. Disable radar tactical map

```c
FUN_00656df0(1);  // Radar::SetTacticalMapAvailable(true)
// Debug: "Radar: TacticalMap availability is %s"
```

##### 5g. Disable sidebar input

```c
(**(code **)(*DAT_0088730c + 0x18))(0);
// Calls HiddenSurface->vtable[6](0) — likely disables surface input
```

##### 5h. Play EVA speech: "You have lost"

```c
FUN_004f42f0(2);  // RequestRedraw with mode 2

// Set screen tint to dark/red:
_DAT_00a8d108 = 0x01010101;  // screen color filter RGBA
_DAT_00a8d10c = 0x01010101;

// Check if sound is 0x73:
iVar6 = FUN_00775b20();  // Get current sound theme ID
if (iVar6 == 0x73) {
    uVar11 = FUN_00775b10();  // Get sound handle
    FUN_00658640(uVar11);     // Stop current theme music
}
```

##### 5i. Display defeat message (if not observer)

```c
if (ObserverHouse != PlayerPtr) {  // DAT_00ac1198 != DAT_00a83d4c
    char msg[160];
    // Format: player name from this+0x1602A
    sprintf(msg, TXT_PLAYER_DEFEATED, this->PlayerName);

    // Get color from player's color scheme
    uint color = FUN_0069a310(DAT_00a8b394);  // Resolve message color

    // Display message via MessageListClass
    FUN_005d3ba0(0, 0, msg, color, 0x4046, 0/*sound*/, 0);

    // Play EVA speech
    FUN_00752700(-1);  // Play "EVA_YouHaveLost"
}
FUN_004f42f0(0);  // RequestRedraw normal

// Debug print:
printf("MPlayer_Defeated() - Player %s has been defeated (OBIWAN MODE)\n", this->PlayerName);
```

The `0x4046` flag to `FUN_005d3ba0` means "UI channel, non-positional, interruptible."

#### STEP 6: Opponent defeated path

```c
else {  // This is NOT the local player
```

##### 6a. Check if house type is NOT MultiplayPassive

```c
if (this->HouseType->MultiplayPassive == false) {  // *(type+0x1A6) == 0
```

`MultiplayPassive` at HouseTypeClass+0x1A6 marks observer/passive houses that should
not trigger defeat messages.

##### 6b. Set MapIsClear flag for this player slot

```c
DAT_00a8022c[this->HouseIndex] + 0x241 = 1;  // Set "MapIsClear" on their slot
```

##### 6c. Debug print + display message

```c
printf("MPlayer_Defeated: frame %d, house id %d, MapIsClear set to true\n",
       CurrentFrame, this->HouseIndex);

if (this != ObserverHouse) {  // param_1 != DAT_00ac1198
    char msg[160];
    sprintf(msg, TXT_PLAYER_DEFEATED, this->PlayerName);  // +0x1602A

    // Color from the defeated player's color scheme: this+0x16054
    FUN_005d3ba0(0, 0, msg, this->ColorScheme, 0x4046, 0, 0);

    FUN_00752700(-1);  // Play "EVA_PlayerDefeated"
}
FUN_004f42f0(0);  // RequestRedraw

// Debug print:
printf("MPlayer_Defeated() - Opponent %s has been defeated\n", this->PlayerName);
```

Key difference: the **opponent's message uses their own color scheme** (`this+0x16054`),
while the local player's message uses a global message color (`DAT_00a8b394`).

#### STEP 7: Count remaining alive houses (CRITICAL)

```c
int alive = 0;    // total alive players
int humans = 0;   // alive human players

for (int i = 0; i < HouseClass::Array.Count; i++) {  // DAT_00a80238
    HouseClass *h = HouseClass::Array[i];  // DAT_00a8022c

    if (h == NULL) continue;
    if (h->IsDefeated) continue;          // +0x1F5 != 0 -> skip
    if (h->HouseType->MultiplayPassive) continue;  // type+0x1A6 != 0 -> skip observers

    // Determine if this house is "human"
    bool isHuman;
    if (GameMode == 0) {  // campaign
        isHuman = (h->IsHuman || h->IsSpectator);  // +0x1EC || +0x1ED
    } else {  // multiplayer
        isHuman = h->IsHuman;  // +0x1EC
    }

    if (isHuman) {
        humans++;
    }
    alive++;
}

printf("MPlayer_Defeated() - Alive = %d, Humans = %d\n", alive, humans);
```

**A house is "alive" if:**
1. Not NULL
2. `IsDefeated` (+0x1F5) is false
3. `HouseType->MultiplayPassive` (+0x1A6) is false (not an observer house type)

#### STEP 8: Check if all remaining are allied (CRITICAL)

This is a **double-nested loop** — for every alive house, check if it is allied with
every other alive house.

```c
for (int i = 0; i < HouseClass::Array.Count; i++) {
    HouseClass *outer = HouseClass::Array[i];
    if (outer == NULL || outer->IsDefeated || outer->HouseType->MultiplayPassive)
        continue;

    for (int j = 0; j < HouseClass::Array.Count; j++) {
        HouseClass *inner = HouseClass::Array[j];
        if (inner == NULL || inner->IsDefeated || inner->HouseType->MultiplayPassive)
            continue;
        if (outer == inner) continue;  // skip self

        // Check: are outer and inner NOT allied? (either direction)
        int innerIdx = inner->HouseIndex;  // +0x30
        int outerIdx = outer->HouseIndex;

        bool not_allied = false;

        // Check outer->allies for inner
        if (innerIdx != outerIdx) {
            if (innerIdx == -1 ||
                (outer->AllianceBitfield & (1 << (innerIdx & 0x1F))) == 0) {
                not_allied = true;
            }
        }
        // Also check inner->allies for outer (reciprocal)
        if (!not_allied && outer != inner) {
            if (outerIdx != innerIdx) {
                if (outerIdx == -1 ||
                    (inner->AllianceBitfield & (1 << (outerIdx & 0x1F))) == 0) {
                    not_allied = true;
                }
            }
        }

        if (not_allied) {
            // Found two alive, non-allied houses
            if (alive != 1 && humans != 0) {
                // Multiple players alive and at least one human:
                // Game is NOT over. Return immediately.
                return;
            }
            goto game_completion_check;  // LAB_004fc591
        }
    }
}

// If we get here: ALL remaining alive houses are allied with each other
DAT_00a8b8c1 = 1;  // "game completion seen" flag
printf("Saw game completion due to player defeat\n");
allAllied = true;  // iStack_ac = 1
printf("MPlayer_Defeated() - All remaining players are allied\n");
```

**Key logic:** The alliance check is **bidirectional** — both houses must have each other
in their alliance bitfield (`+0x5788`). If house A allies house B but B doesn't ally A,
they are NOT considered allied.

**Early return condition:** If there are 2+ alive houses with at least one human, and any
two are not allied, the game is **not over** — return immediately without triggering
win/lose.

**Single player remaining:** If `alive == 1`, the game always reaches the completion
check (no opponents left).

**All AI remaining:** If `humans == 0` (all remaining are AI), the game also reaches
completion even if they have enemies (the game doesn't keep running for AI-only matches).

#### STEP 9: Game completion — Flag_To_Win or Flag_To_Lose (CRITICAL)

```c
game_completion_check:  // LAB_004fc591

// Re-expand sidebar if it was open before defeat
if (DAT_00824410 != 0) {  // saved "was sidebar open" from step 5b
    FUN_006d1610();  // Sidebar::ExpandSidebar — re-opens the sidebar
}
```

##### BRANCH A: Local player is NOT defeated

```c
if (PlayerPtr->IsDefeated == false) {  // +0x1F5 on DAT_00a83d4c
    printf("MPlayer_Defeated() - Flag_To_Win\n");
    HouseClass::Flag_To_Win(PlayerPtr, 0);  // FUN_004fc9e0
    return;
}
```

If the local player is still alive when game completion triggers, they WIN.

##### BRANCH B: Local player IS defeated — co-op check

```c
// Check if this is a co-op game (GameMode 3 or 4) with an active session
bool isCoop = false;
if ((GameMode == 3 || GameMode == 4) && DAT_00a8b23c != NULL) {
    bool sessionValid = DAT_00a8b23c->vtable[1]();  // IsValid
    bool sessionActive = DAT_00a8b23c->vtable[2]();  // IsActive
    if (sessionValid && sessionActive) {
        isCoop = true;
    }
}
```

**GameMode values:**
- 0 = Campaign
- 3 = LAN multiplayer
- 4 = WOL (online) multiplayer
- 5 = Skirmish

##### BRANCH B1: Co-op and all remaining are allied

```c
if (allAllied && isCoop) {
    // In co-op, check if any surviving ally is human
    bool allyHumanAlive = false;

    for (int i = 0; i < HouseClass::Array.Count; i++) {
        HouseClass *h = HouseClass::Array[i];
        if (h == NULL || h->IsDefeated) continue;

        bool isHuman;
        if (GameMode == 0) {
            isHuman = (h->IsHuman || h->IsSpectator);
        } else {
            isHuman = h->IsHuman;
        }
        if (isHuman) {
            allyHumanAlive = true;
        }
    }

    if (allyHumanAlive) {
        // A human ally survived — "Allies win"
        printf("MPlayer_Defeated() - Allies win, Flag_To_Win\n");
        HouseClass::Flag_To_Win(PlayerPtr, 0);
        return;
    } else {
        // No human allies survived — "Allies lost"
        printf("MPlayer_Defeated() - Allies lost, Flag_To_Lose\n");
        HouseClass::Flag_To_Lose(PlayerPtr, 0);
        return;
    }
}
```

**Co-op win condition:** If the local player is defeated but all remaining players are
allied (the enemy team was wiped out) AND at least one surviving ally is human, the
local player gets a **WIN** (co-op victory). If no human allies survived (only AI allies
remain), it is a **LOSS**.

##### BRANCH B2: Not co-op, or not all-allied

```c
else {
    printf("MPlayer_Defeated() - Flag_To_Lose\n");
    HouseClass::Flag_To_Lose(PlayerPtr, 0);
    return;
}
```

Default: the defeated local player loses.

---

## 2. Destroy_All_Owned (0x4FB920)

**Size:** 141 bytes
**Signature:** `void __fastcall HouseClass::Destroy_All_Owned(HouseClass *this)`
**Called by:** FUN_00686890 (likely the game cleanup/house destruction path)

```c
void Destroy_All_Owned(HouseClass *this) {
    // Phase 1: Destroy all teams owned by this house
    for (int i = 0; i < TeamClass::Array.Count; i++) {  // DAT_00a8ec88
        TeamClass *team = TeamClass::Array[i];  // DAT_00a8ec7c
        if (team->Owner == this) {  // team[0x87] == this
            if (team != NULL) {
                team->vtable->Destroy(1);  // vtable+0x20, arg=1 (delete)
                // After deletion, re-check same index (decrement i)
            }
            i--;
        }
    }

    // Phase 2: Destroy all triggers associated with this house
    for (int i = 0; i < TriggerClass::Array.Count; i++) {  // DAT_00a8eaf8
        TriggerClass *trig = TriggerClass::Array[i];  // DAT_00a8eaec
        // Check: trigger's associated object's owner matches this house's index
        if (trig->Object->OwnerHouse == this->HouseIndex) {
            // +0x24->+0xA4 == this->HouseIndex (+0x30 * 4 = +0xD at int* scale)
            if (trig != NULL) {
                trig->vtable->Destroy(1);
            }
            i--;
        }
    }

    // Phase 3: Destroy the house itself
    if (this != NULL) {
        this->vtable->Destroy(1);
    }
}
```

**Important:** This function destroys teams and triggers owned by the house, then the
house itself. It does NOT directly destroy units/buildings. Unit destruction is handled
separately by the game's update loop — once `IsDefeated` is set, the house's units are
typically handled through scatter/self-destruct logic triggered by the defeat event
processing in the main game loop (FUN_006e3180 calls ScatterAllUnits).

---

## 3. ScatterAllUnits (0x4FC6D0)

**Size:** 181 bytes
**Signature:** `void __fastcall HouseClass::ScatterAllUnits(HouseClass *this)`
**Called by:** FUN_006e3180 (game event processing, likely EventClass::Execute for scatter)

```c
void ScatterAllUnits(HouseClass *this) {
    TechnoClass *prev = NULL;

    for (int i = 0; i < TechnoClass::Array.Count; ) {  // DAT_00a8ec88
        TechnoClass *techno = TechnoClass::Array[i];    // DAT_00a8ec7c

        int owner = TechnoClass::GetOwnerHouseIndex(techno);  // FUN_0070f820

        if (owner == techno->Owner  // [0x87]
            || (owner == this
                && (techno->IsDeployed == 0  // [0xB0]
                    || !FUN_00472330(techno)))) // additional deploy check
            && owner == this
            && techno != prev) {

            int homeCell = techno->HomeCell;  // [0x1B] at int* scale = +0x6C
            if (techno->HasPassengers) {       // [0x9E] != 0
                FUN_0071ad40();  // Eject passengers
            }
            // Scatter: vtable+0x16C = TechnoClass::Scatter
            techno->vtable->Scatter(&homeCell, 0, Rules->ScatterDistance, 0, 1, 1, 0);
            prev = techno;  // track last scattered to avoid infinite loops
        } else {
            i++;
        }
    }
}
```

**Behavior:** Iterates all techno objects. For those owned by the defeated house:
1. Ejects passengers if any
2. Scatters the unit toward its home cell position
3. Uses a `prev` tracker to avoid re-processing the same unit in case the array shifts

**Related scatter variants:**
- `FUN_004fc790` (ScatterBuildingsOnly): Only scatters RTTI==6 (deployed buildings)
- `FUN_004fc820` (ScatterNonNaval): Scatters non-naval units (type+0xCCE==0)
- `FUN_004fc8d0` (ScatterNaval): Scatters naval units (type+0xCCE!=0)

---

## 4. Flag_To_Win_Check (0x4FC980)

**Size:** 87 bytes
**Signature:** `char __fastcall HouseClass::Flag_To_Win_Check(HouseClass *this)`
**Called by:** FUN_0064c380, FUN_005da750, FUN_004c6cb0

```c
char Flag_To_Win_Check(HouseClass *this) {
    char result = this->FlagToWinPending;  // +0x1F6

    if (this->HasWon == false) {           // +0x1F7
        if (result == false && this->HasLost == false) {  // +0x1F8
            // Not won, not lost, not pending: set pending
            this->FlagToWinPending = 1;    // +0x1F6
            this->WinLossStartFrame = CurrentFrame;  // +0x298 = DAT_00a8ed84
            this->BorrowedTime = 0;        // +0x2A0
            return this->FlagToWinPending;
        }
        return result;
    }
    return result;
}
```

This is a "soft" win check — sets the pending flag and records the frame, but does NOT
actually call Flag_To_Win. The caller is expected to follow up.

---

## 5. Flag_To_Win (0x4FC9E0)

**Size:** 487 bytes
**Signature:** `undefined4 __thiscall HouseClass::Flag_To_Win(HouseClass *this, char param_2)`
**Called by:** FUN_006dd8b0 (HouseClass::Update), FUN_004fcdc0 (Check_Win_Condition),
              FUN_006ede40 (event handler), FUN_004fc0b0 (MPlayer_Defeated)

```c
int Flag_To_Win(HouseClass *this, char skipBorrowedTime) {
    if (this->HasWon || this->FlagToWinPending || this->HasLost)
        return this->HasWon;

    this->HasWon = 1;  // +0x1F7

    int currentFrame = CurrentFrame;  // DAT_00a8ed84

    if (!skipBorrowedTime) {
        // Record win start frame and calculate borrowed time
        this->WinLossStartFrame = currentFrame;  // +0x298
        this->BorrowedTime = FUN_007c5f00();     // +0x2A0 — some calculated time

        // Multiplayer borrowed time calculation
        if (GameMode != 0 && GameMode != 5) {  // not campaign, not skirmish
            int remaining = this->BorrowedTime;
            if (this->WinLossStartFrame != -1) {
                int elapsed = CurrentFrame - this->WinLossStartFrame;
                remaining = max(0, remaining - elapsed);
            }

            // Compare against SessionClass::MaxAhead (DAT_00a8b550)
            if (remaining <= MaxAhead) {
                // Use MaxAhead as the base borrowed time
                remaining = MaxAhead;
                // Reset start frame to now
            }

            // Round UP to next 10-frame boundary
            remaining = ((CurrentFrame + 9 + remaining) / 10) * 10 - CurrentFrame;

            this->WinLossStartFrame = CurrentFrame;
            this->BorrowedTime = remaining;
        }

        printf("Frame %d, BorrowedTime == %d\n", CurrentFrame, remaining);
    }

    // Show victory message/EVA
    if (GameMode == 0) {  // Campaign
        if (!this->IsHuman && !this->IsSpectator)
            goto done;
        ScenarioClass::Force_Freeze();  // FUN_00684240
        // Display "TXT_SCENARIO_WON" / "EVA_MissionAccomplished"
        Tactical::SetLabel(TXT_SCENARIO_WON, 0);  // FUN_006d4db0
    } else {  // Multiplayer
        if (this != PlayerPtr)
            goto done;
        // Display "TXT_VICTORIOUS" / "EVA_YouAreVictorious"
        Tactical::SetLabel(TXT_VICTORIOUS, 2);
    }

    FUN_00752700(-1);     // Play EVA speech
    FUN_004f42f0(0);      // RequestRedraw

done:
    return this->HasWon;
}
```

### Borrowed Time Explained

**Borrowed time** is the grace period after a win/lose flag is set before the game
actually ends. In multiplayer:

1. An initial value is computed (likely `Rules->BorrowedTimeFrames` or similar)
2. It is clamped to at least `SessionClass::MaxAhead` (the network lookahead buffer)
3. It is rounded UP to the nearest multiple of 10 frames
4. The game continues ticking for this many frames, allowing network traffic to flush
   and final events to propagate

In campaign (`GameMode==0`) and skirmish (`GameMode==5`), borrowed time is **skipped** —
the game freezes immediately via `ScenarioClass::Force_Freeze`.

---

## 6. Flag_To_Lose (0x4FCBD0)

**Size:** 495 bytes
**Signature:** `char __thiscall HouseClass::Flag_To_Lose(HouseClass *this, char param_2)`
**Called by:** FUN_006dd8b0 (HouseClass::Update), FUN_006ede60 (event handler),
              FUN_004fc0b0 (MPlayer_Defeated)

```c
char Flag_To_Lose(HouseClass *this, char skipBorrowedTime) {
    this->HasWon = 0;  // +0x1F7 = 0 (explicitly clear win flag)

    if (this->FlagToWinPending)  // +0x1F6
        return this->HasLost;

    if (this->HasLost)           // +0x1F8
        goto done;

    this->HasLost = 1;           // +0x1F8

    if (!skipBorrowedTime) {
        // Same borrowed time calculation as Flag_To_Win
        this->WinLossStartFrame = CurrentFrame;
        this->BorrowedTime = FUN_007c5f00();

        if (GameMode != 0 && GameMode != 5) {
            // ... identical borrowed time rounding logic ...
            // Round UP to next 10-frame boundary
        }

        printf("Frame %d, BorrowedTime == %d\n", CurrentFrame, remaining);
    }

    // Show loss message/EVA
    if (GameMode == 0) {  // Campaign
        if (!this->IsHuman && !this->IsSpectator)
            goto done;
        ScenarioClass::Force_Freeze();
        // Display "TXT_SCENARIO_LOST" / "EVA_MissionFailed"
        Tactical::SetLabel(TXT_SCENARIO_LOST, 1);
    } else {  // Multiplayer
        if (this != PlayerPtr)
            goto done;
        // Display "TXT_LOST" / "EVA_YouHaveLost"
        Tactical::SetLabel(TXT_LOST, 3);

        if (PlayerPtr != ObserverHouse)
            goto skip_redraw;  // Don't call FUN_00752700 for observer
    }

    FUN_00752700(-1);     // Play EVA speech

skip_redraw:
    FUN_004f42f0(0);      // RequestRedraw

done:
    return this->HasLost;
}
```

**Key differences from Flag_To_Win:**
1. Explicitly clears `HasWon` flag first
2. Will NOT proceed if `FlagToWinPending` is set (win takes priority over loss)
3. For multiplayer loss, there is an extra check: if the local player IS the observer
   house, the EVA speech is skipped but redraw still happens

---

## 7. Check_Win_Condition (0x4FCDC0)

**Size:** 49 bytes
**Signature:** `void __fastcall HouseClass::Check_Win_Condition(HouseClass *this)`
**Called by:** FUN_006dd8b0 (HouseClass::Update)

```c
void Check_Win_Condition(HouseClass *this) {
    if (this->HasLost == false && this->HasWon == false) {
        // Neither won nor lost: call Flag_To_Win
        HouseClass::Flag_To_Win(this, 0);
        return;
    }

    // Already won or lost: just ensure start frame is set
    if (this->WinLossStartFrame == -1) {
        this->WinLossStartFrame = CurrentFrame;
    }
}
```

This is called from the main house update loop. If the house hasn't been flagged yet,
it calls Flag_To_Win. Otherwise it ensures the timing data is initialized.

---

## 8. Alliance AI Rearrangement (0x501640)

**Size:** 271 bytes
**Called from:** MPlayer_Defeated (and 5 other callers)

When triggered from MPlayer_Defeated, this function restructures AI alliances:

```c
void Rearrange_AI_Alliances() {
    bool hasSession = (DAT_00a8b23c != NULL) && DAT_00a8b23c->IsValid();
    bool sessionFlag = Rules->HasSidebarPanels;  // +0x14B5

    if (!(hasSession || sessionFlag)) return;
    if (Scen->SomeFlag) return;  // +0x11E0 on DAT_00a8b230

    for (int i = 0; i < HouseClass::Array.Count; i++) {
        HouseClass *h = HouseClass::Array[i];
        if (h == NULL || h->IsDefeated) continue;

        bool isHuman = h->IsHuman;
        if (GameMode == 0) {
            isHuman = (h->IsHuman || h->IsSpectator);
        }

        if (!isHuman) {
            // This is an AI house — set its alliance flag
            h->SomeFlag_24A = 1;  // +0x24A

            // Re-ally/break-ally with all other houses
            for (int j = 0; j < HouseClass::Array.Count; j++) {
                HouseClass *other = HouseClass::Array[j];
                if (other == NULL || other->IsDefeated) continue;

                bool otherIsHuman = other->IsHuman;
                if (GameMode == 0) {
                    otherIsHuman = (other->IsHuman || other->IsSpectator);
                }

                if (!otherIsHuman) {
                    // Both are AI: ally them
                    HouseClass::Make_Ally(h, other, 0);
                } else {
                    // AI vs Human: break alliance
                    HouseClass::Break_Alliance(h, other, 0);
                }
            }
        }
    }
}
```

---

## 9. Key Globals and Offsets

### Globals

| Global | Name | Description |
|--------|------|-------------|
| `DAT_00a83d4c` | `PlayerPtr` | Pointer to local player's HouseClass |
| `DAT_00ac1198` | `ObserverHouse` | Pointer to the neutral observer house |
| `DAT_00a8022c` | `HouseClass::Array` | Array of all HouseClass pointers |
| `DAT_00a80238` | `HouseClass::Array.Count` | Number of houses |
| `DAT_00a8b238` | `SessionClass::GameMode` | 0=campaign, 3=LAN, 4=WOL, 5=skirmish |
| `DAT_00a8b23c` | `SessionClass::Instance` | Session management object |
| `DAT_00a8b550` | `SessionClass::MaxAhead` | Multiplayer frame lookahead |
| `DAT_00a8ed84` | `Scen->Frame` | Current game frame number |
| `DAT_00a8ef98` | `InvalidCell` | Sentinel for null/invalid cell coordinates |
| `DAT_008871e0` | `RulesClass` | Game rules singleton |
| `DAT_00a8b230` | `SpecialFlags` | Pointer to special flags bitfield |
| `DAT_00a8b538` | `GameEnding` | Set to 1 when game is ending |
| `DAT_00a8b8c1` | `GameCompletionSeen` | Set to 1 when game completion detected |
| `DAT_00824410` | `SidebarWasOpen` | Saved sidebar state during defeat |
| `DAT_00884d2c` | `SidebarIsOpen` | Current sidebar open state |
| `DAT_00a8b394` | `DefaultMessageColor` | Default color for system messages |
| `DAT_008b410c` | `BuildingClass::Array` | Building object array |
| `DAT_008b4118` | `BuildingClass::Array.Count` | Building count |
| `DAT_00a8ec7c` | `TechnoClass::Array` | All techno objects |
| `DAT_00a8ec88` | `TechnoClass::Array.Count` | Techno count |

### HouseClass Offsets (defeat-relevant)

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| +0x30 | int | HouseIndex | 0-31 house ID |
| +0x34 | ptr | HouseType | Pointer to HouseTypeClass |
| +0x1EC | bool | IsHuman | True for human-controlled houses |
| +0x1ED | bool | IsSpectator | True for spectator houses |
| +0x1F5 | bool | IsDefeated | Set by MPlayer_Defeated |
| +0x1F6 | bool | FlagToWinPending | Soft win pending |
| +0x1F7 | bool | HasWon | Set by Flag_To_Win |
| +0x1F8 | bool | HasLost | Set by Flag_To_Lose |
| +0x241 | bool | MapIsClear | Set when shroud is revealed |
| +0x24C | int | IQLevel | AI intelligence level |
| +0x298 | int | WinLossStartFrame | Frame when win/loss was flagged |
| +0x29C | ??? | WinLossField2 | Unknown secondary field |
| +0x2A0 | int | BorrowedTime | Remaining frames before game ends |
| +0x5788 | uint32 | AllianceBitfield | One bit per house (1=allied) |
| +0x53DC | ptr | RallyPointObject | Current rally point target |
| +0x53E0 | cell | RallyPointCell | Rally point cell coordinates |
| +0x15FF4 | char[20] | PlayerName | Player/house name string |
| +0x1602A | wchar[] | PlayerNameWide | Wide-char player name |
| +0x16054 | int | ColorScheme | Color index for messages |

### HouseTypeClass Offsets

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| +0x1A6 | bool | MultiplayPassive | If true, house is passive/observer type |

### SpecialFlags Bits (at *DAT_00a8b230)

| Bit | Value | Name | Used in MPlayer_Defeated? |
|-----|-------|------|---------------------------|
| 4 | 0x10 | Unknown (rally clear?) | Yes - clears rally point |
| 11 | 0x800 | HarvesterImmune | Yes - ejects garrison occupants |
| 12 | 0x1000 | FogOfWar | No (used elsewhere) |

---

## 10. Implementation Notes

### Complete defeat flow sequence

1. **IsDefeated = 1** (immediate)
2. **AI alliance rearrangement** (if defeated was max-IQ AI in multiplayer)
3. **Rally point cleared** (if SpecialFlags bit 4)
4. **Garrison occupants ejected** (if multiplayer + SpecialFlags bit 11)
5. **Local player effects**: beacons deleted, sidebar collapsed, input disabled,
   map revealed, screen tinted, "EVA_YouHaveLost" played, defeat message shown
6. **Opponent effects**: MapIsClear set, defeat message in their color,
   "EVA_PlayerDefeated" played
7. **Count alive houses** (skip NULL, defeated, and MultiplayPassive)
8. **Alliance check** (bidirectional — BOTH houses must have each other as ally)
9. **Game completion decision:**
   - If alive>1 and humans>0 and any two alive houses are enemies: **RETURN** (game continues)
   - If alive==1: game over, remaining player wins
   - If all alive are allied: game over
   - If humans==0: game over (no point continuing AI-only)
10. **Win/Lose assignment:**
    - Local player alive -> **Flag_To_Win**
    - Local player defeated + co-op + all-allied + human ally alive -> **Flag_To_Win**
    - Local player defeated + co-op + all-allied + no human ally -> **Flag_To_Lose**
    - Local player defeated + anything else -> **Flag_To_Lose**

### Borrowed time mechanism

Both Flag_To_Win and Flag_To_Lose calculate borrowed time the same way:
1. Get initial borrowed time value
2. In multiplayer (mode 3 or 4, NOT skirmish 5), clamp to at least `MaxAhead`
3. Round up to next 10-frame boundary: `((frame + 9 + remaining) / 10) * 10 - frame`
4. Store as `BorrowedTime` at +0x2A0 with `WinLossStartFrame` at +0x298
5. The game continues ticking until borrowed time expires

In campaign mode, `Force_Freeze()` is called instead — no borrowed time, immediate freeze.

### Units are NOT immediately destroyed

MPlayer_Defeated does NOT call Destroy_All_Owned or ScatterAllUnits directly.
- `Destroy_All_Owned` (0x4FB920) is called from FUN_00686890, which is the house
  **destruction** path — called later during cleanup, not during defeat.
- `ScatterAllUnits` (0x4FC6D0) is called from FUN_006e3180, which is the event
  processing system. Units scatter when defeat events propagate.

The defeated player's units continue to exist after MPlayer_Defeated returns. They are
cleaned up through subsequent game ticks and events.

### Alliance check is O(n^2)

The "all remaining allied" check does a full N*N comparison of all alive houses.
For a typical 8-player game this is trivial, but the algorithm is worth noting for
correctness — it checks **both directions** of every pair.

### Observer handling

- `ObserverHouse` (`DAT_00ac1198`) is a special neutral house
- MultiplayPassive houses (type+0x1A6) are excluded from alive counts
- When local player == observer, some EVA speeches are skipped
- When defeated player == observer, defeat message is not shown

### Confidence level

**HIGH confidence** on the complete control flow — this is all from direct decompilation
with string references confirming every branch. The debug printf strings embedded in
the binary (`"MPlayer_Defeated() - Allies win, Flag_To_Win"` etc.) confirm the logic
at each decision point.

**MEDIUM confidence** on SpecialFlags bit 4 (0x10) — this bit is not in the documented
13-flag set (bits 5-18). It may be an undocumented internal flag.

**MEDIUM confidence** on the exact borrowed time initial value calculation
(`FUN_007c5f00`) — the function is called but its internals were not decompiled in this
session.
