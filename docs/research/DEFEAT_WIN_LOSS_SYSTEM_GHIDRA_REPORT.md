# Multiplayer Defeat Detection & Win/Loss Borrowed Time System

> **2026-08-29 active-binary correction.** Any `ScatterAllUnits` wording in
> this report for `0x004FC6D0` is superseded. The function is the shared House
> destruction sweep: it walks the live Techno registry, clears incoming
> Temporal links, and calls concrete `ReceiveDamage` with current health and
> Rules C4. See `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`.

Source: Ghidra decompilation of `gamemd.exe` (Yuri's Revenge), `D:\ra2mdpost\House.CPP`

Confidence: HIGH for MPlayer_Defeated, Flag_To_Win, Flag_To_Lose, and the active
win/loss timer/Vox-expiry blocks in HouseClass::Update. The latter were rechecked
against the live `0x004F8440` body and active mode-5 callsites on 2026-08-13.

> **2026-08-13 active-YR correction.** Live bodies and active offline callsites
> disprove this report's old "random ~7 frame" initialization. Both accepted
> result functions load `RulesClass+0x14C8` (`[AudioVisual] SavourDelay`),
> multiply by `900.0`, and call `Math__ftol`; mode 5 skips only the subsequent
> network MaxAhead/10-frame realignment. The constructor default is exact f64
> `0.03` (27 frames), while an explicit INI `.03` passes through f32-narrow
> `ReadDouble` and truncates to 26; retail `.1` yields 90. At expiry,
> `HouseClass__Update @ 0x004F8440` also waits for the whole Vox system to go
> idle or for 120 fresh 16-ms buckets before raising the exit request. Sections
> 4, 5, 7, 8, and the lifecycle below have been corrected for this active
> offline contract; their network-only MaxAhead discussion remains scoped to
> modes 3/4.

---

## Table of Contents

1. [HouseClass Field Map (Win/Loss/Defeat)](#1-housclass-field-map)
2. [HouseClass::Update Defeat Detection (step 13)](#2-defeat-detection-in-update)
3. [MPlayer_Defeated (0x4fc0b0)](#3-mplayer_defeated)
4. [Flag_To_Win (0x4fc9e0)](#4-flag_to_win)
5. [Flag_To_Lose (0x4fcbd0)](#5-flag_to_lose)
6. [Flag_To_Win_Check (0x4fc980)](#6-flag_to_win_check)
7. [Borrowed Time Calculation — Detailed Walkthrough](#7-borrowed-time)
8. [Win/Loss Processing in Update (steps 6-8)](#8-win-loss-processing-in-update)
9. [Complete State Machine Diagram](#9-state-machine)

---

## 1. HouseClass Field Map

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x30  | 4    | HouseIndex | Index into global house array |
| +0x34  | 4    | HouseTypeClass* | Pointer to country type data |
| +0x1ec | 1    | IsHuman | Human-controlled flag |
| +0x1ed | 1    | IsPlayerControl | Co-op player control flag |
| +0x1f5 | 1    | IsDefeated | Set by MPlayer_Defeated(); permanent |
| +0x1f6 | 1    | FlagToWinPending | Intermediate "about to scatter units" state |
| +0x1f7 | 1    | HasWon | Victory flag; set by Flag_To_Win |
| +0x1f8 | 1    | HasLost | Loss flag; set by Flag_To_Lose |
| +0x298 | 4    | WinLossStartFrame | Frame counter when win/loss was triggered (-1 = inactive) |
| +0x29c | 4    | (unused/padding) | Part of timer triple; not used meaningfully |
| +0x2a0 | 4    | BorrowedTimeFrames | Remaining borrowed time duration in frames |
| +0x5788 | 4   | AllianceBitfield | Bit N set = allied with house N |

**Timer pattern**: The engine uses a triple `[start_frame, unused, duration]` (12 bytes).
A timer is expired when `current_frame - start_frame >= duration`. A start_frame of -1
means the timer is inactive/not running. The win/loss timer at +0x298 follows this pattern.

**Key globals**:

| Address | Name | Purpose |
|---------|------|---------|
| DAT_00a8022c | HouseArray | Array of HouseClass* pointers |
| DAT_00a80238 | HouseCount | Number of active houses |
| DAT_00a83d4c | PlayerPtr | Local human player's HouseClass* |
| DAT_00ac1198 | NeutralHouse | Neutral/civilian HouseClass* |
| DAT_00a8b238 | SessionType | 0=campaign, 1-4=multiplayer modes, 5=observer |
| DAT_00a8ed84 | CurrentFrame | Global frame counter |
| DAT_00a8b550 | MaxAhead | SessionClass::MaxAhead — network lookahead frames |
| DAT_008871e0 | RulesClass | Singleton game rules |

**HouseTypeClass flags**:

| Offset | Field | Notes |
|--------|-------|-------|
| +0x1a6 | MultiplayPassive | If true, house is passive in MP (not counted for defeat) |

---

## 2. Defeat Detection in HouseClass::Update (Step 13)

Address: within 0x4f8440 (HouseClass::Update), around 0x4f8d00-0x4f9000.

**NOTE**: The function at 0x4f8440 (3879 bytes) is not in the static decompiled files.
The following is reconstructed from the existing verified report (produced from a prior
live Ghidra session) plus cross-referencing with the OwnedOf counting system.

### What triggers defeat

The defeat detection runs **every frame** as step 13 of HouseClass::Update, but only
for **multiplayer** games (SessionType != 0). It is gated by:

1. `this->IsDefeated == false` (already defeated houses are skipped)
2. `this->HouseTypeClass->MultiplayPassive == false` (passive houses exempt)

### What is counted

The engine counts **total owned objects** using the IndexClass tracking arrays. The
HouseClass contains 12 IndexClass arrays (constructed in the HouseClass constructor at
0x4f54a0) that track per-type counts. The key function is:

**OwnedOf (0x49fae0)** — Returns the count of owned units of a specific type index:
```c
int OwnedOf(IndexClass* tracking_array, int type_index) {
    if (type_index >= tracking_array->capacity)
        if (!Grow(type_index + 10)) return 0;
    return tracking_array->data[type_index];
}
```

**GetTotal (0x49fb60)** — Returns the grand total across all types in a tracking array:
```c
int GetTotal(IndexClass* tracking_array) {
    return tracking_array->total;  // field at +0x10
}
```

### The defeat condition (pseudocode)

```c
// In HouseClass::Update, step 13 (around 0x4f8d00):
// Only in multiplayer AND only if not already defeated
if (SessionType != 0 && !this->IsDefeated && !this->HouseTypeClass->MultiplayPassive) {

    // Count total owned buildings
    int total_buildings = GetTotal(&this->OwnedBuildingTracker);  // IndexClass for buildings

    // Count total owned units (vehicles)
    int total_units = GetTotal(&this->OwnedUnitTracker);         // IndexClass for vehicles

    // Count total owned infantry
    int total_infantry = GetTotal(&this->OwnedInfantryTracker);  // IndexClass for infantry

    // Count total owned aircraft
    int total_aircraft = GetTotal(&this->OwnedAircraftTracker);  // IndexClass for aircraft

    // Defeat occurs when the house owns NOTHING
    if (total_buildings + total_units + total_infantry + total_aircraft == 0) {
        MPlayer_Defeated(this);  // 0x4fc0b0
    }
}
```

**Key finding**: Defeat is triggered when the house has **zero total owned objects** across
ALL categories (buildings, units, infantry, aircraft). It does NOT only check ConYards.
The "Count ConYards + units" description in the earlier report summary is misleading —
it counts everything.

The tracking arrays are maintained by:
- **Added_To_Game (0x502a80)**: Called when any unit is created/captured, increments the
  appropriate IndexClass tracker
- **Removed_From_Game (0x5025f0)**: Called when any unit is destroyed/lost, decrements
  the appropriate IndexClass tracker

There is **no grace period** before defeat. The moment the total drops to zero, defeat
triggers immediately on that frame.

---

## 3. MPlayer_Defeated (0x4fc0b0) — Full Annotated Decompilation

Address: 0x4fc0b0, Size: 1559 bytes. Source: `D:\ra2mdpost\House.CPP`

```c
// this = HouseClass* (param_1 in decompilation)
void HouseClass::MPlayer_Defeated(void) {
    // ---- STEP 1: Mark as defeated ----
    this->IsDefeated = true;  // +0x1f5 = 1

    // ---- STEP 2: Check IQ for alliance recalc ----
    // If this house's IQ (+0x24c) equals the game's max IQ level (RulesClass+0x1434):
    if (this->CurrentIQ == RulesClass->MaxIQLevels) {
        // Check if this is a human or player-controlled house
        bool is_human_or_pc;
        if (SessionType == 0) {  // campaign
            is_human_or_pc = (this->IsHuman || this->IsPlayerControl);
        } else {
            is_human_or_pc = this->IsHuman;  // multiplayer
        }

        // If human AND game has "fog of war" type enabled AND is multiplayer:
        if (!is_human_or_pc && RulesClass->field_0x17e0 && SessionType != 0) {
            FUN_00501640();  // AI alliance recalculation
        }
    }

    // ---- STEP 3: Clear rally point ----
    // If OBIWAN mode (special debug flag at DAT_00a8b230 & 0x10):
    if (*DAT_00a8b230 & 0x10) {
        // Clear the rally point object/cell
        int rally_obj = this->RallyPointObject;  // +0x53dc
        if (rally_obj == 0) {
            if (this->RallyCell != InvalidCell)  // +0x53e0
                rally_obj = CellClass::GetFromCell(this->RallyCell);
        }
        ClearRallyPoint(rally_obj, true);  // FUN_004fbe40
    }

    // ---- STEP 4: Destroy owned buildings (if flag 0x800 set) ----
    // SessionFlags & 0x800 AND multiplayer: destroy all owned buildings
    if (SessionType != 0 && (*DAT_00a8b230 & 0x800)) {
        for (int i = 0; i < BuildingClass::Count; i++) {
            BuildingClass* bld = BuildingClass::Array[i];
            if (bld->Owner == this && (char)bld->field_0x24 != '\0') {
                bld->vtable->TakeDamage();  // vtable+0xf8
            }
        }
    }

    // ---- STEP 5: Handle local player defeat ----
    if (this == PlayerPtr) {  // DAT_00a83d4c
        FUN_00431410(this->HouseIndex);  // Disable keyboard input
        if (DAT_00884d2c) {  // Screen recording active?
            FUN_006d1660();  // Stop recording
        }
        DAT_00824410 = (DAT_00884d2c != '\0');

        FUN_00637a10();             // Clear selection
        FUN_004ac960(0);            // Clear placement state
        DAT_00a8b538 = 1;           // Disable production
        FUN_00577f30(this);         // Disable shroud updates
        FUN_00656df0(1);            // Disable sidebar
        (*DAT_0088730c->vtable+0x18)(0);  // Disable radar

        FUN_004f42f0(2);            // Request full redraw (type 2)

        // Set screen to darkened state
        _DAT_00a8d108 = 0x01010101;
        _DAT_00a8d10c = 0x01010101;

        // Play EVA sound
        int sound_id = FUN_00775b20();
        if (sound_id == 0x73) {     // "EVA_YouHaveLost"
            int eva = FUN_00775b10();
            FUN_00658640(eva);      // Play speech
        }

        // Show defeat message (unless this IS the neutral house)
        if (NeutralHouse != PlayerPtr) {
            char* uiname = &this->UIName;  // +0x1602a
            int assert_id = FUN_00734e60("D:\\ra2mdpost\\House.CPP", 0x1383);
            FUN_007ca564(msg_buf, assert_id, uiname);
            // Show "TXT_PLAYER_DEFEATED" message with player name and color
            FUN_005d3ba0(0, 0, msg_buf, this->Color, 0x4046, FUN_007c5f00(0), 0);
            FUN_00752700(-1);  // Play EVA
        }
        FUN_004f42f0(0);  // Request redraw

        Debug("MPlayer_Defeated() - Player %s has been defeated (OBIWAN MODE)", this->PlayerName);

    // ---- STEP 6: Handle opponent defeat ----
    } else {
        // Only for non-MultiplayPassive opponents
        if (this->HouseTypeClass->MultiplayPassive == '\0') {
            // Mark in score tracking
            DAT_00a8022c[this->HouseIndex]->field_0x241 = 1;

            Debug("MPlayer_Defeated: frame %d, house id %d, MapIsClear set to true",
                  CurrentFrame, this->HouseIndex);

            // Show message for non-neutral houses
            if (this != NeutralHouse) {
                char* uiname = &this->UIName;
                int assert_id = FUN_00734e60("D:\\ra2mdpost\\House.CPP", 0x1399);
                FUN_007ca564(msg_buf, assert_id, uiname);
                FUN_005d3ba0(0, 0, msg_buf, this->ColorSchemeIndex, 0x4046, FUN_007c5f00(0), 0);
                FUN_00752700(-1);  // Play "EVA_PlayerDefeated"
            }
            FUN_004f42f0(0);
        }
        Debug("MPlayer_Defeated() - Opponent %s has been defeated", this->PlayerName);
    }

    // ---- STEP 7: Count remaining alive players ----
    int alive_count = 0;
    int human_alive_count = 0;
    for (int i = 0; i < HouseCount; i++) {
        HouseClass* h = HouseArray[i];
        if (h != NULL
            && h->IsDefeated == false            // +0x1f5
            && h->HouseTypeClass->MultiplayPassive == false)  // +0x1a6
        {
            // Check if this house is human
            bool is_human;
            if (SessionType == 0) {
                is_human = (h->IsHuman || h->IsPlayerControl);
            } else {
                is_human = h->IsHuman;
            }
            if (is_human) {
                human_alive_count++;
            }
            alive_count++;
        }
    }
    Debug("MPlayer_Defeated() - Alive = %d, Humans = %d", alive_count, human_alive_count);

    // ---- STEP 8: Check if all remaining players are allied ----
    bool all_allied = true;
    for (int i = 0; i < HouseCount; i++) {
        HouseClass* h1 = HouseArray[i];
        if (h1 == NULL || h1->IsDefeated || h1->HouseTypeClass->MultiplayPassive)
            continue;

        for (int j = 0; j < HouseCount; j++) {
            HouseClass* h2 = HouseArray[j];
            if (h2 == NULL || h2->IsDefeated || h2->HouseTypeClass->MultiplayPassive)
                continue;
            if (h1 == h2) continue;

            // Check if h1 and h2 are mutual allies
            int h2_idx = h2->HouseIndex;
            int h1_idx = h1->HouseIndex;

            bool h1_allies_h2 = (h1_idx == h2_idx)
                || (h2_idx != -1 && (h1->AllianceBitfield & (1 << (h2_idx & 0x1f))) != 0);
            bool h2_allies_h1 = (h2_idx == h1_idx)
                || (h1_idx != -1 && (h2->AllianceBitfield & (1 << (h1_idx & 0x1f))) != 0);

            if (!h1_allies_h2 || !h2_allies_h1) {
                // Found two non-allied alive players — game continues
                // EARLY EXIT condition:
                if (alive_count != 1 && human_alive_count != 0) {
                    return;  // Game not over yet
                }
                goto decide_win_loss;
            }
        }
    }

    // If we reach here: ALL remaining alive players are allied with each other
    DAT_00a8b8c1 = 1;  // Global "game completion" flag
    Debug("Saw game completion due to player defeat");
    all_allied = true;
    Debug("MPlayer_Defeated() - All remaining players are allied");

decide_win_loss:
    // Stop recording if needed
    if (DAT_00824410) {
        FUN_006d1610();  // Stop screen recording
    }

    // ---- STEP 9: Decide win or loss for local player ----
    if (PlayerPtr->IsDefeated == false) {
        // Local player is alive — they WIN
        Debug("MPlayer_Defeated() - Flag_To_Win");
        PlayerPtr->Flag_To_Win(false);   // 0x4fc9e0, param=0 (calculate borrowed time)
        return;
    }

    // Local player is defeated. Check co-op/team win conditions:
    // SessionType 3 or 4 (co-op modes) AND internet game exists AND connected
    bool is_coop = (SessionType == 3 || SessionType == 4)
                   && DAT_00a8b23c != NULL
                   && (*DAT_00a8b23c->vtable+4)()   // IsConnected
                   && (*DAT_00a8b23c->vtable+8)();   // IsHost

    if (all_allied && is_coop) {
        // Co-op mode: check if any alive human ally exists
        bool any_alive_human_ally = false;
        for (int i = 0; i < HouseCount; i++) {
            HouseClass* h = HouseArray[i];
            if (h != NULL && h->IsDefeated == false) {
                bool is_human;
                if (SessionType == 0) {
                    is_human = (h->IsHuman || h->IsPlayerControl);
                } else {
                    is_human = h->IsHuman;
                }
                if (is_human) {
                    any_alive_human_ally = true;
                    break;
                }
            }
        }
        if (any_alive_human_ally) {
            // Allied team still has human players alive — count as win!
            Debug("MPlayer_Defeated() - Allies win, Flag_To_Win");
            PlayerPtr->Flag_To_Win(false);
            return;
        }
        // No human allies left
        Debug("MPlayer_Defeated() - Allies lost, Flag_To_Lose");
    } else {
        // Not co-op or not all-allied: straight loss
        Debug("MPlayer_Defeated() - Flag_To_Lose");
    }

    PlayerPtr->Flag_To_Lose(false);  // 0x4fcbd0, param=0 (calculate borrowed time)
}
```

### Summary of MPlayer_Defeated decision tree

```
MPlayer_Defeated called on house H:
  1. H.IsDefeated = true
  2. If local player (H == PlayerPtr):
       - Disable all UI, show "You have lost" EVA
  3. Else (opponent):
       - Show "Player X has been defeated" message
  4. Count alive, non-defeated, non-MultiplayPassive houses
  5. Check if ALL remaining alive are mutually allied:
     - If two non-allied alive houses exist AND alive>1 AND humans>0:
         return (game continues, no win/loss yet)
     - If only 1 alive OR 0 humans remain:
         fall through to win/loss decision
     - If ALL allied: set game-completion flag
  6. Win/Loss for local player:
     - If PlayerPtr is NOT defeated: Flag_To_Win(0)
     - Else if co-op AND ally-humans alive: Flag_To_Win(0)
     - Else: Flag_To_Lose(0)
```

---

## 4. Flag_To_Win (0x4fc9e0) — Full Annotated Decompilation

Address: 0x4fc9e0, Size: 487 bytes. Source: `D:\ra2mdpost\House.CPP`

String references: "Frame %d, BorrowedTime == %d", "TXT_SCENARIO_WON",
"EVA_MissionAccomplished", "TXT_VICTORIOUS", "EVA_YouAreVictorious"

```c
// param_2: char bypass_borrowed_time (0 = calculate normally, 1 = skip)
// Returns: HasWon flag value
char HouseClass::Flag_To_Win(char bypass_borrowed_time) {

    // Only trigger if NONE of HasWon, FlagToWinPending, HasLost are set
    if (this->HasWon == 0       // +0x1f7
        && this->FlagToWinPending == 0   // +0x1f6
        && this->HasLost == 0)  // +0x1f8
    {
        // ---- Set the win flag ----
        this->HasWon = 1;  // +0x1f7

        // ---- Calculate borrowed time ----
        if (bypass_borrowed_time == 0) {
            // Initialize timer from [AudioVisual] SavourDelay.
            this->WinLossStartFrame = CurrentFrame;  // +0x298
            this->BorrowedTimeFrames = ftol(Rules->SavourDelay * 900.0);  // +0x2a0

            // In multiplayer (not campaign, not observer mode):
            if (SessionType != 0 && SessionType != 5) {
                // Get current remaining borrowed time
                int remaining = this->BorrowedTimeFrames;
                if (this->WinLossStartFrame != -1) {
                    int elapsed = CurrentFrame - this->WinLossStartFrame;
                    remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
                }

                // Clamp minimum to SessionClass::MaxAhead
                int* timer_source = &this->WinLossStartFrame;
                if (remaining <= MaxAhead) {
                    // Use MaxAhead as minimum: create a temp timer
                    // temp_timer = { CurrentFrame, unused, MaxAhead }
                    timer_source = &temp_timer;  // local variable with MaxAhead
                }

                // Recalculate remaining from the chosen timer source
                remaining = timer_source[2];  // duration field
                if (timer_source[0] != -1) {  // if timer is active
                    int elapsed = CurrentFrame - timer_source[0];
                    remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
                }

                // Round up to next 10-frame boundary
                remaining = ((CurrentFrame + 9 + remaining) / 10) * 10 - CurrentFrame;

                // Store the final computed borrowed time
                this->WinLossStartFrame = CurrentFrame;  // reset start to NOW
                this->BorrowedTimeFrames = remaining;    // store computed time
            }

            // Log the borrowed time
            int bt_remaining = this->BorrowedTimeFrames;
            if (this->WinLossStartFrame != -1) {
                int elapsed = CurrentFrame - this->WinLossStartFrame;
                bt_remaining = (elapsed < bt_remaining) ? (bt_remaining - elapsed) : 0;
            }
            Debug("Frame %d, BorrowedTime == %d", CurrentFrame, bt_remaining);
        }

        // ---- Show victory message ----
        if (SessionType == 0) {
            // CAMPAIGN mode:
            if (this->IsHuman || this->IsPlayerControl) {
                FUN_00684240();  // Campaign completion handler
                // StringTable entry 0x1607 = "TXT_SCENARIO_WON"
                // EVA type 0 = "EVA_MissionAccomplished"
                ShowMessageAndEVA(0x1607, 0);
                PlayEVA(-1);
                RequestRedraw(0);
            }
            // Non-human houses in campaign: skip message
        } else {
            // MULTIPLAYER mode:
            if (this == PlayerPtr) {
                // StringTable entry 0x160a = "TXT_VICTORIOUS"
                // EVA type 2 = "EVA_YouAreVictorious"
                ShowMessageAndEVA(0x160a, 2);
                PlayEVA(-1);
                RequestRedraw(0);
            }
            // Non-local players in MP: skip message
        }
    }

    return this->HasWon;  // +0x1f7
}
```

---

## 5. Flag_To_Lose (0x4fcbd0) — Full Annotated Decompilation

Address: 0x4fcbd0, Size: 495 bytes. Source: `D:\ra2mdpost\House.CPP`

String references: "Frame %d, BorrowedTime == %d", "TXT_SCENARIO_LOST",
"EVA_MissionFailed", "TXT_LOST", "EVA_YouHaveLost"

```c
// param_2: char bypass_borrowed_time (0 = calculate normally, 1 = skip)
// Returns: HasLost flag value
char HouseClass::Flag_To_Lose(char bypass_borrowed_time) {

    // ---- CRITICAL: Clear HasWon first ----
    this->HasWon = 0;  // +0x1f7 = 0 (losing overrides winning!)

    // If FlagToWinPending is set, bail out (return HasLost state)
    if (this->FlagToWinPending != 0) {  // +0x1f6
        return this->HasLost;
    }

    // If already lost, skip to end
    if (this->HasLost != 0) goto return_lost;

    // ---- Set the loss flag ----
    this->HasLost = 1;  // +0x1f8

    // ---- Calculate borrowed time (identical logic to Flag_To_Win) ----
    if (bypass_borrowed_time == 0) {
        this->WinLossStartFrame = CurrentFrame;
        this->BorrowedTimeFrames = ftol(Rules->SavourDelay * 900.0);

        if (SessionType != 0 && SessionType != 5) {
            int remaining = this->BorrowedTimeFrames;
            if (this->WinLossStartFrame != -1) {
                int elapsed = CurrentFrame - this->WinLossStartFrame;
                remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
            }

            int* timer_source = &this->WinLossStartFrame;
            if (remaining <= MaxAhead) {
                timer_source = &temp_timer;  // {CurrentFrame, unused, MaxAhead}
            }

            remaining = timer_source[2];
            if (timer_source[0] != -1) {
                int elapsed = CurrentFrame - timer_source[0];
                remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
            }

            remaining = ((CurrentFrame + 9 + remaining) / 10) * 10 - CurrentFrame;
            this->WinLossStartFrame = CurrentFrame;
            this->BorrowedTimeFrames = remaining;
        }

        int bt_remaining = this->BorrowedTimeFrames;
        if (this->WinLossStartFrame != -1) {
            int elapsed = CurrentFrame - this->WinLossStartFrame;
            bt_remaining = (elapsed < bt_remaining) ? (bt_remaining - elapsed) : 0;
        }
        Debug("Frame %d, BorrowedTime == %d", CurrentFrame, bt_remaining);
    }

    // ---- Show loss message ----
    if (SessionType == 0) {
        // CAMPAIGN mode:
        if (this->IsHuman || this->IsPlayerControl) {
            FUN_00684240();  // Campaign failure handler
            // StringTable entry 0x163e = "TXT_SCENARIO_LOST"
            // EVA type 1 = "EVA_MissionFailed"
            ShowMessageAndEVA(0x163e, 1);
            PlayEVA(-1);
        }
    } else {
        // MULTIPLAYER mode:
        if (this == PlayerPtr) {
            // StringTable entry 0x1641 = "TXT_LOST"
            // EVA type 3 = "EVA_YouHaveLost"
            ShowMessageAndEVA(0x1641, 3);

            // Special: if local player IS the observer, skip the final PlayEVA
            if (PlayerPtr != NeutralHouse) {
                PlayEVA(-1);
            }
        }
    }
    RequestRedraw(0);

return_lost:
    return this->HasLost;  // +0x1f8
}
```

### Key difference from Flag_To_Win

Flag_To_Lose **clears HasWon** (+0x1f7 = 0) at the very top, BEFORE any other checks.
This means: if a race condition causes both win and lose to be triggered, **lose wins**.
It also checks `FlagToWinPending` — if the house was in the "about to scatter units"
state, the loss is suppressed (returns current HasLost which is 0).

The guard conditions differ:
- Flag_To_Win: `HasWon==0 && FlagToWinPending==0 && HasLost==0` (all three must be clear)
- Flag_To_Lose: `FlagToWinPending==0 && HasLost==0` (HasWon is forcibly cleared first)

---

## 6. Flag_To_Win_Check (0x4fc980) — Annotated Decompilation

Address: 0x4fc980, Size: 87 bytes.

This is a simpler entry point used by capture/crate/trigger events (called from 3 places:
0x64c380, 0x5da750, 0x4c6cb0).

```c
// Returns: FlagToWinPending value
char HouseClass::Flag_To_Win_Check(void) {
    char pending = this->FlagToWinPending;  // +0x1f6

    if (this->HasWon == 0) {   // +0x1f7
        if (pending == 0 && this->HasLost == 0) {  // +0x1f8
            // Set the "pending win" flag
            this->FlagToWinPending = 1;

            // Record current frame as start, with ZERO borrowed time
            this->WinLossStartFrame = CurrentFrame;   // +0x298 = g_CurrentFrame
            // +0x29c = local_8 (uninitialized/garbage — not used)
            this->BorrowedTimeFrames = 0;             // +0x2a0 = 0

            return this->FlagToWinPending;
        }
        return pending;  // already set or already lost
    }
    return pending;  // already won
}
```

**Purpose**: This sets up the "FlagToWinPending" state with zero borrowed time. The
actual scatter-and-win happens in HouseClass::Update step 8 (see below).

---

## 7. Borrowed Time Calculation — Detailed Walkthrough

The result timer keeps the scenario running long enough to savour the result EVA. Its base
duration is rules-driven in every mode; network modes additionally align/clamp it for lockstep.

### The algorithm (shared between Flag_To_Win and Flag_To_Lose)

```c
void calculate_borrowed_time(HouseClass* house) {
    // Step 1: Convert [AudioVisual] SavourDelay minutes to 15-Hz frames.
    int base_time = ftol(Rules->SavourDelay * 900.0);

    // Step 2: Store timer triple
    house->WinLossStartFrame = CurrentFrame;  // +0x298
    house->BorrowedTimeFrames = base_time;    // +0x2a0

    // Step 3: Only adjust in multiplayer (not campaign, not observer)
    if (SessionType == 0 || SessionType == 5)
        return;  // campaign/offline skirmish: keep the SavourDelay conversion

    // Step 4: Calculate remaining time from timer
    int remaining = house->BorrowedTimeFrames;
    if (house->WinLossStartFrame != -1) {
        int elapsed = CurrentFrame - house->WinLossStartFrame;
        // Since we JUST set start = CurrentFrame, elapsed = 0, so remaining unchanged
        remaining = max(0, remaining - elapsed);
    }

    // Step 5: Clamp minimum to MaxAhead
    // MaxAhead (DAT_00a8b550) = SessionClass::MaxAhead = network lookahead frames
    // This ensures the borrowed time is at LEAST MaxAhead frames
    int effective_remaining;
    int* timer_ptr;
    if (remaining <= MaxAhead) {
        // Override: use MaxAhead as the duration
        // Create a virtual timer: {CurrentFrame, unused, MaxAhead}
        timer_ptr = &virtual_timer;  // points to local stack variable
        effective_remaining = MaxAhead;
    } else {
        timer_ptr = &house->WinLossStartFrame;
        effective_remaining = remaining;
    }

    // Step 6: Re-read duration from chosen timer (handles the clamp)
    int duration = timer_ptr[2];  // offset +8 from timer start = duration field
    if (timer_ptr[0] != -1) {
        int elapsed = CurrentFrame - timer_ptr[0];
        duration = max(0, duration - elapsed);
    }

    // Step 7: Round up to next 10-frame boundary
    // This aligns the game end to a clean frame number
    int final_time = ((CurrentFrame + 9 + duration) / 10) * 10 - CurrentFrame;

    // Step 8: Store final values
    house->WinLossStartFrame = CurrentFrame;  // reset start to now
    house->BorrowedTimeFrames = final_time;   // store aligned borrowed time
}
```

### Concrete example

Network-only realignment example (the active offline skirmish path does not enter this block):

Suppose:
- `CurrentFrame = 1543`
- `MaxAhead = 5` (typical LAN game)
- base `SavourDelay` has already been converted to a duration no greater than MaxAhead

1. `remaining = 0`
2. `0 <= 5` (MaxAhead), so use MaxAhead path: `duration = 5`
3. Round up: `(1543 + 9 + 5) / 10 * 10 - 1543 = 1557 / 10 * 10 - 1543 = 1550 - 1543 = 7`
4. Final: `BorrowedTimeFrames = 7`

The network-aligned timer will expire 7 frames later, at frame 1550. Stock offline skirmish
instead keeps the unaligned 90-frame duration produced by `SavourDelay=.1`.

### Why 10-frame alignment?

The `((frame + 9 + remaining) / 10) * 10` formula rounds up to the next multiple of 10.
This is for **lockstep network synchronization** — the game processes commands in batches,
and ending on a clean boundary ensures all clients agree on the exact end frame.

---

## 8. Win/Loss Processing in HouseClass::Update (Steps 6-8)

These are steps 6, 7, and 8 of HouseClass::Update at 0x4f8440, running every frame.

### Step 6: HasWon processing

```c
// Update step 6: Win timer countdown
if (this->HasWon) {  // +0x1f7
    int remaining = this->BorrowedTimeFrames;  // +0x2a0
    if (this->WinLossStartFrame != -1) {       // +0x298
        int elapsed = CurrentFrame - this->WinLossStartFrame;
        remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
    }

    if (remaining <= 0) {
        // Savour timer expired. Anchor a fresh timeGetTime()>>4 bucket,
        // pump the whole Vox system until inactive or delta >= 0x78,
        // then raise the victory transition request.
        WaitForVoxOr120Buckets();
        RequestVictoryExit(this);
    }
    // else: still waiting, do nothing (borrowed time ticking down)
}
```

### Step 7: HasLost processing

```c
// Update step 7: Loss timer countdown
if (this->HasLost) {  // +0x1f8
    int remaining = this->BorrowedTimeFrames;
    if (this->WinLossStartFrame != -1) {
        int elapsed = CurrentFrame - this->WinLossStartFrame;
        remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
    }

    if (remaining <= 0) {
        // Same fresh-bucket Vox gate, then raise the defeat transition request.
        WaitForVoxOr120Buckets();
        RequestDefeatExit(this);
    }
}
```

### Step 8: FlagToWinPending processing

> **SUPERSEDED:** the pseudo-code and interpretation below are retained only as
> historical context. Active `0x004FC6D0` destroys admitted Technos; it does
> not scatter them or flag a win. The pending byte is cleared and the shared
> destruction sweep runs when the exact signed timer reaches zero.

```c
// Update step 8: Pending win → scatter units → actual win
if (this->FlagToWinPending) {  // +0x1f6
    int remaining = this->BorrowedTimeFrames;  // +0x2a0
    if (this->WinLossStartFrame != -1) {
        int elapsed = CurrentFrame - this->WinLossStartFrame;
        remaining = (elapsed < remaining) ? (remaining - elapsed) : 0;
    }

    if (remaining <= 0) {
        // Pending period expired — scatter all units, then flag actual win
        ScatterAllUnits(this);  // FUN_004fc6d0 — scatter to home positions
        this->FlagToWinPending = 0;
        Flag_To_Win(false);  // Now trigger the actual win with borrowed time
    }
}
```

**FlagToWinPending** is a two-phase win: first scatter units (cosmetic — they celebrate),
then after that timer expires, trigger the actual Flag_To_Win which starts its own
borrowed time before the game truly ends.

---

## 9. Complete State Machine Diagram

```
                          ┌──────────────────────┐
                          │     PLAYING           │
                          │  IsDefeated=0         │
                          │  HasWon=0             │
                          │  HasLost=0            │
                          │  FlagToWinPending=0   │
                          └──────────┬────────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                 │
            Total owned=0    Crate/Trigger     All enemies defeated
                    │           win event        (via MPlayer_Defeated
                    ▼                │            on another house)
           ┌────────────────┐       │                 │
           │ MPlayer_Defeated│       │                 │
           │ IsDefeated=1   │       ▼                 ▼
           └───────┬────────┘  ┌──────────┐    ┌───────────────┐
                   │           │FlagToWin  │    │MPlayer_Defeated│
                   │           │Pending    │    │on opponent     │
                   │           │+0x1f6=1   │    └───────┬───────┘
                   │           │BT=0       │            │
                   │           └─────┬─────┘            │
                   │                 │              Check: all
                   │           (wait BT=0)         remaining
                   │                 │              allied?
                   │                 ▼                  │
                   │           ScatterUnits        ┌────┴────┐
                   │           FlagToWin→0         │         │
                   │                 │            YES        NO
                   │                 ▼             │         │
                   │           Flag_To_Win    (game        (return,
                   │           HasWon=1       continues)   game goes on)
                   │           BT=calculated       │
                   │                 │             ▼
                   │           (wait BT→0)   Flag_To_Win(0)
                   │                 │        for local if alive
                   │                 ▼        Flag_To_Lose(0)
                   │           GAME OVER      for local if dead
                   │           (VICTORY)           │
                   │                          ┌────┴────┐
                   │                          │         │
                   │                     HasWon=1  HasLost=1
                   │                     BT=calc   BT=calc
                   │                          │         │
                   │                    (wait BT)  (wait BT)
                   │                          │         │
                   │                          ▼         ▼
                   │                      GAME OVER  GAME OVER
                   │                      (VICTORY)  (DEFEAT)
                   │
              ┌────┴───────────┐
              │                │
         PlayerPtr==this   PlayerPtr!=this
         (local defeat)    (opponent defeat)
              │                │
              ▼                ▼
         Flag_To_Lose    Check all-allied
         for self        → may trigger
                         Flag_To_Win for
                         local player
```

### The frame-by-frame lifecycle of a multiplayer defeat

1. **Frame N**: Last unit of House X is destroyed
   - `Removed_From_Game()` decrements tracking counter
   - Total owned objects now = 0

2. **Frame N+1** (or same frame): `HouseClass::Update()` runs for House X
   - Step 13 detects total owned = 0
   - Calls `MPlayer_Defeated(X)`

3. **Inside MPlayer_Defeated**:
   - Sets `X.IsDefeated = 1`
   - If X is local player: disables UI, shows EVA
   - Counts remaining alive players
   - If all remaining are allied OR only 1 remains:
     - Local player alive? `Flag_To_Win(0)` → `HasWon=1, BorrowedTime=90 frames` with stock `.1`
     - Local player dead? `Flag_To_Lose(0)` → `HasLost=1, BorrowedTime=90 frames` with stock `.1`

4. **The next 90 frames with stock rules**: Update continues running
   - Step 6 or 7: checks borrowed time each frame
   - `remaining = BorrowedTime - (CurrentFrame - StartFrame)`
   - Each frame, remaining decreases by 1

5. **At timer expiry**: `remaining <= 0`; a fresh at-most-120-bucket whole-Vox wait completes before the transition request is raised
   - Game-over screen triggers
   - Score is displayed
   - Session ends

### EVA Speech Summary

| Context | EVA String | StringTable | Notes |
|---------|-----------|-------------|-------|
| Local player defeated (in MPlayer_Defeated) | EVA_YouHaveLost | — | Immediate on defeat |
| Opponent defeated | EVA_PlayerDefeated | TXT_PLAYER_DEFEATED | With opponent name |
| Flag_To_Win (campaign) | EVA_MissionAccomplished | TXT_SCENARIO_WON (0x1607) | |
| Flag_To_Win (multiplayer) | EVA_YouAreVictorious | TXT_VICTORIOUS (0x160a) | Only for PlayerPtr |
| Flag_To_Lose (campaign) | EVA_MissionFailed | TXT_SCENARIO_LOST (0x163e) | |
| Flag_To_Lose (multiplayer) | EVA_YouHaveLost | TXT_LOST (0x1641) | Skipped if observer |

---

## Appendix: Raw Decompiled Code Reference

The raw Ghidra decompilation for the three fully-available functions (MPlayer_Defeated,
Flag_To_Win, Flag_To_Lose) can be found in:
`<local>/Documents/gidra/gidra c files/048_004f9700_004ff233.c`
- MPlayer_Defeated (FUN_004fc0b0): line 2055
- Flag_To_Win (FUN_004fc9e0): line 2570
- Flag_To_Lose (FUN_004fcbd0): line 2701
- Flag_To_Win_Check (FUN_004fc980): line 2514
- Check_Win_Condition (FUN_004fcdc0): line 2810

HouseClass::Update (0x4f8440) is NOT in the static decompiled files — it falls in a gap
between file 047 (ending at FUN_004f7870+0xAF4=0x4f8364) and file 047's next function
(FUN_004f93e0). A live Ghidra MCP session would be needed for direct verification of the
defeat detection pseudocode in section 2.
