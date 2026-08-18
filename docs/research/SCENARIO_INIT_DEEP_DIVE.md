# Scenario Initialization Deep Dive — gamemd.exe

Complete reverse-engineering of the four core scenario initialization functions.
All addresses verified from Ghidra decompilation of `gamemd.exe` (Yuri's Revenge).

Sources: Decompiled C files `109_006829a0_00689d30.c`, `110_00689e90_006916b0.c`,
Ghidra reports 056, 057, 109, 110, and prior doc `GAME_START_INITIALIZATION.md`.

Correction (2026-07-25): the value stored at `Scenario+0x34B8`
(`Scen[0xD2E]`) is the selected local side index, not map theater. The
multiplayer writer reads `HouseTypeClass+0xBC` at
`0x0068479D..0x006847C9`; `Full_Init` repeats the side selection at
`0x00687794..0x00687833`. The separate map theater is read from the scenario
INI into the field described in phase 7 below.

---

## Call Chain Overview

```
Main_Game() [0x0052D9A0]                          ← outer game loop, dispatch by game state
  └→ ScenarioClass::Start_Scenario() [0x00683AB0] ← entry point, movie+briefing, progress bar
       └→ ScenarioClass::Read_Scenario() [0x00684620] ← orchestrator: random map vs INI, networking
            ├→ ScenarioClass::Read_Scenario_INI() [0x00686730] ← opens .map file, parses INI
            │    └→ ScenarioClass::Full_Init() [0x00686B20]    ← THE MONSTER (4532 bytes)
            │         ├→ Clear_All() [0x006851F0]              ← destroy previous game state
            │         ├→ Create_Houses() [0x00687F10]          ← player+AI house creation
            │         ├→ ReadINI() [0x00689E90]                ← full scenario INI parsing
            │         ├→ Post_Map_Init() [0x00686890]          ← starting units+credits
            │         └→ ~30 subsystem init calls
            ├→ Post_Load_Init() [0x00684C30]                   ← terrain, vision, AI, particles
            └→ Wait_For_Players() [0x00684370]                 ← multiplayer sync barrier
```

---

## 1. Start_Scenario (0x00683AB0) — 1011 bytes

**Source file:** `D:\ra2mdpost\Scenario.CPP`
**Called by:** Main_Game (0x0052D9A0), End_Game variants (0x00685670, 0x00685DC0, 0x006863E0)
**Calling convention:** `__thiscall` — `this` = scenario filename string, `param_2` = campaign index

### Pseudocode (annotated)

```c
int Start_Scenario(char* filename, int campaign_index) {
    // --- Phase 1: Resolve scenario filename ---
    if (filename == NULL || strlen(filename) == 0) {
        if (campaign_index != -1) {
            if (DAT_00a8ed5d == 1)  // special case flag
                filename = &DAT_00a83e48;  // fixed filename buffer
            else
                filename = CampaignList[campaign_index] + 0x9C;  // campaign entry name
        }
    }

    // Store campaign index
    Scen->field_34CC = campaign_index;

    // --- Phase 2: Debug logging ---
    Debug_Log("\n----- Starting scnenario: %s -----\n", filename);  // NOTE: "scnenario" is a typo in the original!
    Debug_Log("Player Count: %d\n", PlayerList_Count);  // DAT_00a8da84

    // --- Phase 3: Copy filename to ScenarioClass ---
    strcpy(Scen->Filename, filename);  // Scen + 0x125C (260-byte buffer)
    strupr(Scen->Filename);            // FUN_007dcfc4 — uppercase it

    // --- Phase 4: Sound system init ---
    FUN_004790b0();  // Init/reset sound system

    // --- Phase 5: Network carrier detect check (multiplayer only) ---
    if (SessionClass::GameMode != 0) {  // DAT_00a8b238
        if (DAT_00a8b254 == -1) goto skip_network;
        FUN_004790a0();       // get network state
        if (!FUN_0069ac30())  // carrier detect check
            goto skip_network;
        FUN_0069acc0();       // reset network state
    } else {
        if (Scen->field_34CC == -1) goto skip_network;
    }
    FUN_004790b0();

skip_network:
    // --- Phase 6: Increment activity counter ---
    DAT_00a8dab4++;

    // --- Phase 7: Check if CCFile system can open the file ---
    if (!CCFile::Exists(filename)) {
        DAT_00a8dab4--;
        // vtable call on game mode object
        (*DAT_00887640->vfunc_0C)();

        // --- Phase 7a: Pre-read [Basic] for Intro/Brief movies ---
        CCINIClass ini;
        ini.Open(filename);
        ini.Parse(/*streaming=*/1, /*append=*/0);

        int intro = ini.ReadType("Basic", "Intro", -1);
        int brief = ini.ReadType("Basic", "Brief", -1);
        if (intro != -1 || brief != -1) {
            Play_Movie(0);       // FUN_00720ea0
            Show_Briefing(1,1,1); // FUN_005bf260
        }
        ini.~CCINIClass();

        // --- Phase 8: Main scenario loading ---
        Debug_Log("Reading scenario: %s\n", filename);
        Show_Loading_Text("LOADING");

        if (!Read_Scenario()) {  // FUN_00684620 — THE MAIN CALL
            // Error handling
            (*DAT_00887640->vfunc_10)();
            // ... show error dialog, cleanup ...
            return 0;
        }

        // --- Phase 9: Post-load: VQA movie queue ---
        if (Scen->MovieIndex != -1) {  // Scen + 0x1438
            wsprintf(buf, "%s.VQA", MovieNameTable[Scen->MovieIndex]);
        }

        // --- Phase 10: Display resolution toggle ---
        if (current_width != target_width || current_height != target_height) {
            Debug_Log("Toggle display mode to %d x %d\n", target_w, target_h);
            Toggle_Display();
        }

        // --- Phase 11: Final init ---
        FUN_0072def0();  // rendering subsystem init
        FUN_005fb160();  // display surface setup
        Lock_HiddenSurface(0);
        Present(0);

        // --- Phase 12: Start elapsed timer ---
        if (Scen->ElapsedTimer_Start == -1) {  // Scen + 0x614
            Scen->ElapsedTimer_Start = GetTickCount();
        }

        // --- Phase 13: Hand off scenario music ---
        if (Scen->ThemeIndex == -1) {  // Scen + 0x1C70
            ThemeClass::Stop(/* fade = */ true);
            // ThemeClass AI chooses a later track; Start_Scenario does not.
        } else {
            ThemeClass::Queue_Song(Scen->ThemeIndex);
            // Queue_Song fades the current stream before starting this index.
        }

        // --- Phase 14: Set game-active flags ---
        DAT_00a8ed5c = 1;  // game loaded flag
        DAT_00a8e378 = 1;  // game active flag

        // WoL cleanup
        if (SessionClass::GameMode == 4)
            FUN_0078abf0();

        return 1;
    }

    // ... error path, WoL cleanup ...
    return 0;
}
```

### Key observations

1. **Typo in original**: The log string is literally `"Starting scnenario"` — transposed letters.
2. **Campaign vs MP branching**: When `GameMode == 0` (campaign), filename comes from the
   campaign list at `DAT_00a83cfc[campaign_index] + 0x9C`. When multiplayer, filename is
   passed directly.
3. **Pre-read of [Basic]**: Before the full load, it opens the INI just to check for
   `Intro` and `Brief` movie keys. If either exists, it plays the intro movie and shows
   the briefing screen BEFORE loading the scenario. This explains why campaign missions
   show their briefing before the loading bar.
4. **GameMode 4 (WoL/Internet)**: Gets special cleanup via `FUN_0078abf0` on both success
   and failure paths.

---

## 2. Read_Scenario (0x00684620) — 1549 bytes

**Source file:** `D:\ra2mdpost\Scenario.CPP`
**Called by:** Start_Scenario (0x00683AB0) only
**Calling convention:** `__fastcall` — `param_1` = scenario filename

This is the **orchestrator** that decides between random map generation vs. INI-based
loading, sets up the loading progress bar, handles network synchronization, and calls
Post_Load_Init afterward.

### Pseudocode (annotated)

```c
int Read_Scenario(char* filename) {
    // --- Phase 1: Setup ---
    char local_filename[260];
    strcpy(local_filename, filename);
    DAT_00a8ed84 = 0;                      // reset global frame counter
    Scen->LoadingInProgress = 1;            // Scen + 0x3598
    DAT_00a8e7ac++;                         // increment activity/heap lock counter

    // --- Phase 2: Random map detection ---
    // Check if filename ends with random-map suffix (stricmp against suffix)
    if (stricmp(filename_suffix, random_map_marker) == 0) {
        Scen->IsRandom = 1;                // Scen + 0x34BD
        Debug_Log("Scen->IsRandom = true\n");
    } else {
        Scen->IsRandom = 0;
        Debug_Log("Scen->IsRandom = false\n");
    }

    // --- Phase 3: Progress bar setup (if not in editor mode) ---
    if (DAT_00a8ed6b == 0) {  // not editor mode
        int player_count = 1;
        if (GameMode != 0 && GameMode != 5)    // not campaign, not skirmish
            player_count = PlayerList_Count;     // multiplayer player count

        // Create progress bar
        Init_Progress(0, 100.0, player_count, 0);

        // Select local side identity for progress/side-specific UI assets
        int local_side;
        if (GameMode == 0) {                    // campaign
            if (Scen->CampaignIndex != -1)
                local_side = CampaignList[Scen->CampaignIndex] + 0x98;
        } else {                                 // multiplayer
            int country = PlayerList[0]->field_4B;
            if (country == -3) country = 0;      // observer -> default
            local_side = CountryTypeClass::Array[country] + 0xBC;
        }
        Scen->LocalSideID = local_side;          // Scen + 0x34B8

        // Progress bar SHP: campaign uses SPLDBR.SHP, multiplayer uses PROGBARM.SHP
        if (GameMode == 0)
            Set_Progress_Bar("SPLDBR.SHP", 0, FUN_0072b380());
        else
            Set_Progress_Bar("PROGBARM.SHP", 0, 0);

        // --- Phase 3a: UDP broadcast setup (WoL with multiple players) ---
        if (DAT_00887628 != NULL && GameMode == 4 && PlayerList_Count > 1) {
            Debug_Log("Setting addresses for UDP broadcast\n");
            (*SessionObject->vfunc_30)();  // clear broadcast list
            for (int i = 1; i < PlayerList_Count; i++) {
                Format_IP(&ip_struct, &ip_bytes);
                sprintf(ip_string, "%d.%d.%d.%d", ...);
                Debug_Log("Adding broadcast address %s\n", ip_string);
                (*SessionObject->vfunc_2C)(ip_string, port);
            }
        }
    }

    // --- Phase 4: CORE LOADING — random vs. INI ---
    if (Scen->IsRandom == 0) {
        // Normal map: read from INI file
        success = Read_Scenario_INI();              // FUN_00686730
    } else {
        // Random map generation
        success = Generate_Random_Map(local_filename); // FUN_00597a10
        if (success) {
            FUN_00598960(0, 0);     // random map post-processing
            Post_Map_Init();         // FUN_00686890 — create houses, starting units
        }
        // Copy filename to Scen even for random maps
        strcpy(Scen->Filename, local_filename);
    }

    // Phase 4a: Rules-based check
    if (Rules->field_14AE == 0)         // DAT_008871e0 + 0x14AE
        FUN_00577f30(0);                // some terrain init

    // --- Phase 5: Error handling ---
    if (!success) {
        Debug_Log("Error - Unable to read scenario: %s\n", local_filename);
        // Show error dialog: "TXT_UNABLE_READ_SCENARIO" / "TXT_OK"
        DAT_00a8e7ac--;
        Scen->LoadingInProgress = 0;
        return 0;
    }

    // --- Phase 6: Post_Load_Init ---
    // Set up progress bar range text
    FUN_005d3a60(...);  // progress bar positioning
    Post_Load_Init();   // FUN_00684c30 — terrain, objects, AI, particles

    // --- Phase 7: Network progress updates ---
    if (GameMode == 3 || GameMode == 4) {
        Send_Progress(99);   // FUN_0069ae90
        if (GameMode == 4 && DAT_00a8dba0 != 0)
            FUN_00664610(15000, 1);  // wait for sync
    }

    // Send progress 100 or 200 depending on random map
    Send_Progress(Scen->IsRandom ? 200 : 100);
    DAT_00a8b8c1 = 0;
    DAT_00a8b8c2 = 0;

    // --- Phase 8: Wait for all players ---
    success = Wait_For_Players();  // FUN_00684370

    if (GameMode == 4)
        FUN_0069b170();  // WoL specific sync

    DAT_00a8e7ac--;

    // --- Phase 9: Final player sync (multiplayer) ---
    if (DAT_00a8ed6b == 0 && success) {
        Process_Messages();
        while (GetLoadProgress() < 100.0) {
            for (int i = 0; i < PlayerList_Count; i++) {
                Set_Player_Progress(i, 0, 100.0, -1, -1);
            }
        }
    }

    // --- Phase 10: Cleanup ---
    Scen->LoadingInProgress = 0;
    // ... cleanup progress bar ...

    // Per-house post-init
    for (int i = HouseCount - 1; i >= 0; i--) {
        FUN_0050d610();  // per-house finalization
    }

    return 1;
}
```

### Key observations

1. **Random map path**: When `Scen->IsRandom` is true, it calls `FUN_00597a10`
   (random map generator) instead of `Read_Scenario_INI`. After generation, it calls
   `Post_Map_Init` directly (which handles house creation and starting units).
2. **Progress bar**: Campaign uses `SPLDBR.SHP` (single-player loading bar), multiplayer
   uses `PROGBARM.SHP`. Progress values go 3 -> 30 -> 31 -> 35 -> ... -> 96 -> 98 -> 100.
3. **Two-phase post-load**: After the core loading, `Post_Load_Init` handles terrain and
   objects, then `Wait_For_Players` blocks until all multiplayer clients are synced.
4. **Progress 200 signal**: Random maps send progress value 200 (vs 100 for normal maps)
   to signal to other players that random generation completed.

---

## 3. Full_Init (0x00686B20) — 4532 bytes (THE MONSTER FUNCTION)

**Source file:** `D:\ra2mdpost\Scenario.CPP`
**Called by:** Read_Scenario_INI (0x00686730), and saved-game reload (0x00599650)
**Calling convention:** `__fastcall` — `param_1` = official flag, `param_2` = reload flag

This is the single largest initialization function. It takes a parsed INI file (already
loaded into the global INI system) and transforms it into a playable game state. Contains
~70 sub-calls, multiple conditional branches for campaign vs. multiplayer, and two
iteration passes (the `do/while(true)` is for the reload path from saved games).

### Complete Call Sequence (in order)

```
Phase 1: Clear Previous State
  ├─ Debug_Log("Clearing old scenario")
  └─ if (!reloading): Clear_All() [0x006851F0]

Phase 2: Game Mode Setup
  ├─ if (GameMode == 0):  // CAMPAIGN
  │    Scen[0x183] = DAT_00a8eb64          // human difficulty (0=easy,1=med,2=hard)
  │    Scen[0x184] = 2 - DAT_00a8eb64      // AI difficulty (inverse)
  │    Scen[0] &= ~0x1000                   // clear crates flag
  │    DAT_00a8e960 &= ~0x1000              // clear crates in special flags too
  └─ else:  // MULTIPLAYER/SKIRMISH
       Scen[0x183] = DAT_00a8b278           // MP difficulty setting
       Scen[0x184] = 2 - Scen[0x183]
       Scen[0] = (DAT_00a8b31f & 1) << 12 | Scen[0] & ~0x1000  // separate scenario cell-pass flag
       DAT_00a8e960 = (DAT_00a8b31f & 1) << 12 | DAT_00a8e960 & ~0x1000

Phase 3: [Basic] Section
  ├─ InitTime = ReadInt("Basic", "InitTime", 10000)  → Scen[0xD67]
  └─ Official = ReadBool("Basic", "Official", false)

Phase 4: Campaign Metadata (GameMode == 0 only)
  ├─ If NOT reloading:
  │    Open scenario .INI extension file
  │    Load trigger extensions via FUN_00668bf0
  │    Close extension file
  ├─ Open "MISSIONMD.INI"
  │    Parse (streaming, no append)
  │    Read from [ScenarioName] section:
  │      Briefing → Scen+0x4D8 (wide string, 45 chars)
  │        → if non-empty: localize via CSF, store at Scen+0x514
  │      UIName → Scen+0x13BA
  │        → if non-empty: localize, format "%sSav" for save name
  │        → store localized save name at Scen+0x13DA
  │      LSLoadMessage → Scen+0xD8B (31 chars)
  │      LSLoadBriefing → Scen+0x364B (31 chars)
  │      LS640BriefLocX → Scen[0xD9B]
  │      LS640BriefLocY → Scen[0xD9C]
  │      LS800BriefLocX → Scen[0xD9D]
  │      LS800BriefLocY → Scen[0xD9E]
  │      LS640BkgdName → Scen+0xD9F (64 chars)
  │      LS800BkgdName → Scen+0xDAF (64 chars)
  │      LS800BkgdPal → Scen+0xDBF (64 chars)

Phase 5: Initialize Map Slot Arrays
  ├─ Fill Scen+0x1180..0x11C0 with 0xFFFFFFFF (16 dwords = spawn slot table)
  └─ if (GameMode != 0):  // MULTIPLAYER
       FUN_0068bdc0()       // MP house init from lobby data
       if (Scen->IsRandom):
         Copy waypoints from Scen+0x11C0 to Scen+0x632 (8 waypoints = 32 bytes)
       FUN_006722f0()       // rules re-read (overlay/general)
       FUN_0066d530()       // RulesClass::ReadGeneral
       For each CountryTypeClass:
         Call vtable+100 (ReadINI from current rules)
       Create_Houses() [0x00687F10]

       // Set up map LocalSize from INI
       Read "LocalSize" coordinate → FUN_00653F50 / FUN_00654490
       Call map object vtable+0x80 (Set_Map_Dimensions)
       if (DAT_00a8b244 == 2):
         AssignStartingPoints() [0x005EE9D0]
       else:
         Call map object vtable+0x84 (alternate assignment)

Phase 6: Progress Updates + INI System Flush
  ├─ FUN_00552a40 / FUN_00552d60
  └─ Send_Progress(3)

Phase 6.5: Reload Loop Exit
  └─ if (reloading): break out of do/while, re-enter at top with reload=false
     (This allows saved-game loading to re-run the full init with clean state)

Phase 7: Map Object Creation
  ├─ Clear trigger hash table (FUN_006cf230)
  ├─ Destroy existing TacticalMap, allocate new (0xE18 bytes) → DAT_00887324
  ├─ Init display dimensions from DAT_00886fa0
  ├─ Read "Theater" from INI → Scen[0x496]
  ├─ Init_Theater() [0x005349C0]           // load MIX, PAL, unit palettes
  └─ Send_Progress(30)

Phase 8: Terrain + Overlay Loading
  ├─ FUN_00674650 — Load terrain objects (campaign/skirmish flag)     Progress: 31
  ├─ FUN_006686C0 — Load overlays from map INI                        Progress: 35
  └─ Process_Messages()

Phase 9: Variable Names
  ├─ Count = GetSectionEntryCount("VariableNames")
  ├─ Clamp to 50 entries
  └─ For each: atoi(key_name) → index, read value → Scen + index*0x29 + 0x1C88

Phase 10: Trigger Extensions
  └─ FUN_00668bf0 — Load trigger/event extensions        Progress: 45

Phase 11: Determine Local Side for Side-Specific MIX
  ├─ if (GameMode == 0):
  │    local_side = CampaignList[Scen->CampaignIndex] + 0x98
  ├─ else:
  │    country = PlayerList[0]->field_4B (if -3, use 0)
  │    local_side = CountryTypeClass[country] + 0xBC
  ├─ if (GameMode == 0):
  │    FUN_005009b0()              // campaign-specific init
  │    Read "Player" from [Basic] (default "Americans")
  │    house_index = Find_House_By_Name()
  │    local_side = HouseArray[house_index]->HouseType + 0xBC
  └─ Scen[0xD2E] = local_side
     Load_Side_MIX(local_side) [0x00534FA0]   // SIDEC01MD.MIX etc + UIMD.INI

Phase 12: Full Scenario INI Parsing (THE BIG ONE)
  ├─ Send_Progress(50)
  ├─ ReadINI() [0x00689E90]         // buildings, units, infantry, aircraft, houses, lighting
  │    (3800 bytes — reads [Header], [Basic], [Ranking], [Lighting], player assignment)
  └─ Send_Progress(58)

Phase 13: Campaign Player Assignment (GameMode == 0)
  ├─ Re-read "Player" from [Basic] (default "Americans")
  ├─ house_index = Find_House_By_Name()
  ├─ local_side = resolved from house's HouseTypeClass + 0xBC
  ├─ Load_Side_MIX(local_side) — reload if side changed
  └─ Scen[0xD2E] = local_side

Phase 13a: Multiplayer Special Flags
  └─ if (GameMode != 0 && DAT_00a8b260 == 0):
       DAT_00a8e960 &= ~0x8000    // clear superweapons flag if disabled

Phase 14: AI + Scripting Pipeline (sequential, ~20 calls)
  ├─ FUN_006b8b10 — AI system init
  ├─ Process_Messages()
  ├─ FUN_006f19b0 — Waypoint/script loading (called TWICE)
  ├─ FUN_00691970 — Team/taskforce loading (called TWICE)
  ├─ FUN_006e8220 — Tag loading (called TWICE)
  ├─ FUN_007275d0 — Global variables loading
  ├─ FUN_006e5ed0 — Cell trigger loading
  ├─ FUN_0041f2e0 — Action loading (called TWICE)
  ├─ FUN_006f2040 — Script action loading
  ├─ Send_Progress(60)
  ├─ FUN_004ace70 — Smudge loading
  ├─ Process_Messages()
  ├─ FUN_007283c0 — Special flags parsing
  ├─ FUN_00465cc0 — Unknown init
  ├─ FUN_004f42f0(2) — Object pool init (param=2)
  └─ Send_Progress(70)

Phase 15: Fog, Shroud, Terrain Passability
  ├─ Process_Messages()
  ├─ FUN_005fd2e0 — Fog of war init
  ├─ Process_Messages()
  ├─ FUN_00578350 — Start terrain iteration
  ├─ while (FUN_00578290() != 0):
  │    FUN_0047d2b0(-1) — Per-cell terrain passability update
  ├─ FUN_005fddf0 — Shroud edge calculation
  ├─ FUN_0071ca70 — Lighting system init
  ├─ Process_Messages()
  └─ FUN_0074de90 — Ambient sound system init

Phase 16: Tech Tree + Superweapons
  ├─ FUN_00722d00 — Tech tree init
  ├─ FUN_00722240 — Tech tree prerequisites
  ├─ Send_Progress(72)
  ├─ FUN_00654650 — Cell passability recalc
  ├─ FUN_00743270 — Superweapon init
  ├─ Process_Messages()
  └─ Send_Progress(74)

Phase 17: Additional Subsystems
  ├─ FUN_0041b110 — Unknown init
  ├─ Process_Messages()
  ├─ FUN_0051fb00 — Unknown init
  ├─ Process_Messages()
  ├─ Send_Progress(76)
  ├─ DAT_00829ae4 = 0    // disable certain logic
  ├─ FUN_0044f820 — Unknown init
  ├─ Process_Messages()
  ├─ Send_Progress(78)
  ├─ DAT_00829ae4 = 1    // re-enable
  ├─ Process_Messages()
  └─ FUN_006b4c80 — AI trigger init
     Process_Messages()

Phase 18: Skirmish Cheat File
  └─ if (GameMode == 5 && DAT_00a8ed91 != 0):
       Open "TMCJ4F.INI"
       if exists: Parse and load trigger extensions

Phase 19: Final Steps
  ├─ Send_Progress(82)
  ├─ DAT_0087f91c = FUN_00568bb0(0)  // terrain cache value
  ├─ Send_Progress(86)
  ├─ FUN_004309d0 — Unknown final init
  ├─ Send_Progress(90)
  ├─ Process_Messages()
  │
  ├─ if (GameMode != 0 && !reloading):
  │    ReadBool("Basic", "Official", false)   // re-check official flag
  │    Post_Map_Init() [0x00686890]           // starting units/credits + initial crates
  │      if (DAT_00A8B261):
  │        count = min(max(CrateMinimum, human session count), CrateMaximum)
  │        call MapClass__PlaceCrateAtRandomCell once per requested crate
  │
  ├─ Process_Messages()
  ├─ Clear trigger hash table (FUN_006cf230)
  ├─ Send_Progress(96)
  ├─ FUN_00725c70 — Heap garbage collection
  │
  ├─ if (GameMode != 0):
  │    Scen[0] = DAT_00a8e960   // copy special flags to scenario
  │
  ├─ // Temporarily zero heap lock for building state update
  ├─ saved = DAT_00a8e7ac
  ├─ DAT_00a8e7ac = 0
  ├─ FUN_00452d40 — Building state update (idle anims, etc.)
  ├─ DAT_00a8e7ac = saved - 1
  ├─ Send_Progress(98)
  │
  ├─ // vtable call on DAT_00880a0c (map logic object)
  ├─ (*DAT_00880a0c->vfunc_0C)()
  │
  ├─ if (Scen[0] & 0x1000):
  │    FUN_005866c0()            // separate gated MapClass cell pass; not crate placement
  │
  ├─ FUN_00660b00 — Final overlay cleanup
  ├─ FUN_00657ce0 — Final cell cleanup
  └─ return 1  (success)
```

### Conditional Branches Summary

| Condition | When True | When False |
|-----------|-----------|------------|
| `GameMode == 0` | Campaign: read MISSIONMD.INI metadata, use CampaignList for local-side setup | MP/Skirmish: use lobby data, Create_Houses, AssignStartingPoints |
| `Scen->IsRandom` | Copy random map waypoints from +0x11C0 to +0x632 | Read [Header] section for waypoints |
| `reloading` (param_2) | Skip Clear_All on first pass, then re-run with reloading=false | Normal single-pass init |
| `GameMode == 5 && cheat_flag` | Load TMCJ4F.INI cheat triggers | Skip |
| `Scen[0] & 0x1000` (crates) | Place crates on map via FUN_005866c0 | Skip crate placement |
| `DAT_00a8b244 == 2` | Use AssignStartingPoints (0x005EE9D0) | Use alternate map vtable assignment |
| `GameMode != 0 && !reloading` | Call Post_Map_Init for MP starting units | Skip (campaign units come from map INI) |

### Global Variables Read

| Address | Name | Usage |
|---------|------|-------|
| `DAT_00a8b230` | ScenarioClass* (Scen) | Target of all field writes |
| `DAT_00a8b238` | SessionClass::GameMode | 0=campaign, 1-4=MP, 5=skirmish |
| `DAT_00a8eb64` | Campaign difficulty | 0=easy, 1=medium, 2=hard |
| `DAT_00a8b278` | MP difficulty | Same encoding |
| `DAT_00a8b31f` | MP crates flag | Bit 0 = crates enabled |
| `DAT_00a8e960` | Special flags (global copy) | Bitfield, bit 12 = crates, bit 15 = superweapons |
| `DAT_00a8da78` | PlayerList[0] | First human player connection object |
| `DAT_00a8da84` | PlayerList.Count | Number of human players |
| `DAT_00a83cfc` | CampaignList | Array of campaign entry pointers |
| `DAT_00a83c9c` | CountryTypeClass::Array | Array of country type pointers |
| `DAT_00a83ca8` | CountryTypeClass::Count | Number of country types |
| `DAT_00a8022c` | HouseClass::Array | Array of house pointers |
| `DAT_00a80238` | HouseClass::Count | Number of active houses |
| `DAT_008871e0` | RulesClass* | Rules singleton |
| `DAT_00887048` | CCINIClass* (current INI) | The parsed map INI data |
| `DAT_00887324` | TacticalMap* | Main tactical display object (0xE18 bytes) |
| `DAT_00a8ed6b` | Editor mode flag | Nonzero when in map editor |
| `DAT_00a8ed91` | Cheat/debug flag | Enables TMCJ4F.INI loading |
| `DAT_00a8e7ac` | Activity/heap lock counter | Prevents GC during loading |
| `DAT_00880a0c` | Map logic vtable ptr | Used for final init vtable call |
| `DAT_00829ae4` | Logic enable flag | 0=disabled, 1=enabled |

### Global Variables Written

| Address | Name | Value Set |
|---------|------|-----------|
| `DAT_00a8e7ac` | Heap lock | Incremented at start, decremented at end |
| `DAT_00a8e7a8` | Unknown | Set to 0 |
| `DAT_00829ae4` | Logic enable | Toggled 0 then 1 around FUN_0044f820 |
| `DAT_0087f91c` | Terrain cache | Set from FUN_00568bb0(0) return |
| `DAT_00b0c110` | Trigger hash table | Cleared twice via FUN_006cf230 |

### Progress Value Timeline

| Value | What just completed |
|-------|-------------------|
| 3 | Phase 6 — INI system flush |
| 30 | Theater loaded |
| 31 | Terrain objects loaded |
| 35 | Overlays loaded |
| 45 | Trigger extensions loaded |
| 50 | Side MIX loaded |
| 58 | Full ReadINI completed |
| 60 | Script actions loaded |
| 70 | Special flags + object pool |
| 72 | Tech tree initialized |
| 74 | Superweapons initialized |
| 76 | Additional subsystems |
| 78 | Unknown init |
| 82 | AI triggers initialized |
| 86 | Terrain cache computed |
| 90 | Final init |
| 96 | Hash table + GC |
| 98 | Building state update |

---

## 4. Clear_All (0x006851F0) — 1140 bytes

**Called by:** Full_Init (0x00686B20), saved-game load (0x00599650), game-load from save (0x0067E730)

This function performs a **complete teardown** of the previous game state. It destroys
every game object, clears all arrays, resets all timers, and prepares for a fresh
scenario load.

### Pseudocode (annotated)

```c
void Clear_All() {
    // --- Phase 1: Reset core state ---
    Scen->ParTime = 1000000;                 // Scen + 0x214
    Scen->GameActive = 1;                    // Scen + 0x218
    DAT_0083d834 = 0x4000;                   // frame timing default
    FUN_00407150();                           // stop frame processing
    FUN_00750fa0();                           // stop music
    DAT_00a83d4c = 0;                        // clear PlayerPtr (local player)

    // --- Phase 2: Reset scenario defaults ---
    Set_Defaults();                           // FUN_00683610 — reset ALL scenario parameters

    // --- Phase 3: Clear selection ---
    FUN_0053a090();                           // clear current object selection list

    // --- Phase 4: Remove type-0x12 objects from active list ---
    // Iterate backwards through active object list
    for (int i = DAT_00a8e978 - 1; i >= 0; i--) {
        AbstractClass* obj = ActiveList[i] + 4;
        int type = obj->vfunc_0C();          // GetAbstractType
        if (type == 0x12) {                  // type 0x12 = some specific object type
            // Remove from active list (shift elements left)
            DAT_00a8e978--;
            for (int j = i; j < DAT_00a8e978; j++)
                ActiveList[j] = ActiveList[j+1];
        }
    }

    // --- Phase 5: Free ALL game object heaps ---
    Free_Heaps();                             // FUN_00534450 — destroys 30+ object type arrays

    // --- Phase 6: Rebuild active object list ---
    // Re-add objects from a secondary list (DAT_00a8ed2c) to the active list
    for (int i = 0; i < DAT_00a8ed38; i++) {
        // ... complex validation and re-insertion ...
    }

    // --- Phase 7: Clear managed buffers ---
    FUN_00565b00();                           // unknown subsystem cleanup

    // Clear overlay buffer
    DAT_0087f848 = 0;
    if (DAT_0087f83c != 0 && DAT_0087f845 != 0)
        free(DAT_0087f83c);
    DAT_0087f845 = 0;
    DAT_0087f840 = 0;

    // --- Phase 8: Clear trigger system ---
    FUN_006e5570();                           // clear cell triggers

    // --- Phase 9: Clear local variables (fire trigger notifications) ---
    for (int i = 0; i < 50; i++) {           // 50 local variables, stride 0x29
        if (Scen[0x1CB0 + i * 0x29] != 0) {
            Scen[0x1CB0 + i * 0x29] = 0;
            Scen->DirtyFlag = 1;             // Scen + 0x34AA
            Fire_Trigger_Notification();      // FUN_006e57f0
        }
    }

    // --- Phase 10: Recreate TacticalMap ---
    if (DAT_00887324 != NULL)
        DAT_00887324->vfunc_20(1);           // destroy old tactical map
    DAT_00887324 = new TacticalClass();       // allocate 0xE18 bytes
    FUN_006da980();                           // display init

    // --- Phase 11: Destroy all remaining game objects ---
    DAT_00829ae4 = 0;                        // disable logic during cleanup

    // Destroy all AnimClass objects
    while (DAT_00a8e370 != 0) {
        int type = (*ActiveAnims->first->vfunc_2C)();
        if (type == 8)
            (*ActiveAnims->first->vfunc_08)(self); // Limbo
        else if (ActiveAnims->first != NULL)
            (*ActiveAnims->first->vfunc_20)(1);    // Delete
    }

    DAT_00829ae4 = 1;
    DAT_00a8e370 = 0;

    // Destroy all TechnoClass objects
    while (DAT_00b0e790 != 0) {
        if (*DAT_00b0e784 != NULL)
            (*DAT_00b0e784->first->vfunc_20)(1);  // Delete
    }

    // --- Phase 12: Clear dynamic vectors (smudges, overlays) ---
    // Clear smudge array
    DAT_008b41b8 = 0;
    if (DAT_008b41ac != 0 && DAT_008b41b5 != 0)
        free(DAT_008b41ac);
    // ... similar for overlay array at DAT_008b40cc ...

    // --- Phase 13: Reset timers ---
    Scen->TimerA = 0xE10;                    // 3600 ticks = 60 seconds
    Scen->TimerB = 0xE10;                    // Scen + 0x35A8
    Scen->TimerC = 0xE10;                    // Scen + 0x35AC

    // --- Phase 14: Clear string buffers ---
    Scen[0x35B0] = 0;                        // scenario text buffers
    Scen[0x35CF] = 0;
    Scen[0x35EE] = 0;
    Scen[0x360D] = 0;

    // --- Phase 15: Reset ~12 subsystems ---
    FUN_005549a0();                           // subsystem 1
    FUN_00539760();                           // subsystem 2
    FUN_004c5470();                           // subsystem 3
    FUN_0074d760();                           // subsystem 4 (ambient sounds?)
    FUN_00722390();                           // tech tree clear
    FUN_00722e50();                           // tech tree prerequisites clear
    FUN_00439110();                           // subsystem 7
    FUN_0054e6f0();                           // subsystem 8
    FUN_00413800();                           // subsystem 9
    FUN_006370b0();                           // subsystem 10

    // --- Phase 16: Clear map geometry ---
    DAT_0087f8d4 = 0;
    DAT_0087f8d8 = 0;
    DAT_0087f8dc = 0;
    DAT_0087f8e0 = 0;

    // --- Phase 17: Map + Logic init ---
    Debug_Log("Map.Init_Clear()\n");
    FUN_005bdf50();                           // MapClass::Init_Clear
    Debug_Log("Logic.Init()\n");
    (*DAT_0087f778->vfunc_0C)();             // LogicClass::Init

    // --- Phase 18: Clear selection list ---
    Debug_Log("CurrentObjects.Clear()\n");
    DAT_00a8ecc8 = 0;                        // count = 0
    if (DAT_00a8ecbc != 0 && DAT_00a8ecc5 != 0)
        free(DAT_00a8ecbc);
    DAT_00a8ecc5 = 0;
    DAT_00a8ecc0 = 0;

    // --- Phase 19: Clear waypoints ---
    Debug_Log("Scen->Clear_All_Waypoints()\n");
    // Fill all 702 waypoint slots with sentinel value
    for (int i = 0; i < 702; i++)
        Scen->Waypoints[i] = WAYPOINT_SENTINEL;  // DAT_00b05458

    // --- Phase 20: Reload battle config ---
    Debug_Log("Init_Campaigns()\n");
    Load_Battle_INI();                        // FUN_0052cb90 — reload BATTLEMD*.INI

    // --- Phase 21: Cleanup special objects ---
    if (DAT_00a8ed78 != NULL) {
        FUN_0062e650();                       // release
        if (DAT_00a8ed78 != NULL)
            DAT_00a8ed78->vfunc_20(1);        // delete
        DAT_00a8ed78 = NULL;
    }

    // --- Phase 22: Final resets ---
    Scen->GameActive = 0;                    // Scen + 0x218
    FUN_00734210();                           // string table reset
    FUN_00734270();                           // string table reset
    FUN_00660c50();                           // overlay system reset
    Scen->ParTime = 1000000;                 // Scen + 0x214

    return;
}
```

### What Gets Destroyed (Complete List)

| System | How It's Cleared |
|--------|-----------------|
| PlayerPtr (local player) | Set to NULL |
| All scenario parameters | `Set_Defaults()` (1179 bytes of resets) |
| Current object selection | `FUN_0053a090()` + dynamic vector clear |
| Type-0x12 active objects | Removed from active list |
| 30+ game object heaps | `Free_Heaps()` — buildings, units, infantry, etc. |
| Overlay buffer | Freed + zeroed |
| Cell triggers | `FUN_006e5570()` |
| 50 local variables | Zeroed with trigger notifications |
| TacticalMap object | Destroyed + reallocated (0xE18 bytes) |
| All AnimClass instances | Destroyed via vtable |
| All TechnoClass instances | Destroyed via vtable |
| Smudge array | Freed + zeroed |
| Overlay dynamic array | Freed + zeroed |
| 3 countdown timers | Reset to 3600 (60 sec) |
| 4 scenario string buffers | Zeroed |
| 12+ subsystems | Individual clear/init calls |
| Map geometry (4 dwords) | Zeroed |
| MapClass state | `Init_Clear()` |
| LogicClass state | `Init()` |
| CurrentObjects selection | Count zeroed, buffer freed |
| 702 waypoints | Filled with sentinel value |
| Battle config | Reloaded from BATTLEMD*.INI |
| Special objects | Released + deleted |
| String tables | Reset |
| Overlay system | Reset |

---

## 5. SessionClass::GameMode Values

| Value | Mode | Behavior in Init |
|-------|------|-----------------|
| 0 | Campaign/Single-player | Uses CampaignList, reads MISSIONMD.INI, player from [Basic] |
| 1 | LAN (Direct) | Multiplayer path, Create_Houses from lobby data |
| 2 | LAN (?) | Multiplayer path |
| 3 | Direct Connect | Multiplayer path, network progress at 99 |
| 4 | Internet/WoL | Multiplayer path, UDP broadcast setup, WoL cleanup |
| 5 | Skirmish | Multiplayer path but some campaign-like behavior |

---

## 6. Interactions Between Functions

```
Start_Scenario
  │
  ├─ Pre-reads [Basic] for Intro/Brief movies (before loading)
  ├─ Calls Read_Scenario
  │     │
  │     ├─ Sets up progress bar (SPLDBR.SHP or PROGBARM.SHP)
  │     ├─ Random path: Generate_Random_Map → Post_Map_Init
  │     │   OR
  │     ├─ Normal path: Read_Scenario_INI
  │     │     │
  │     │     ├─ Opens file, parses INI
  │     │     └─ Calls Full_Init
  │     │           │
  │     │           ├─ Clear_All (destroy old state)
  │     │           ├─ Read [Basic] (InitTime, Official)
  │     │           ├─ Campaign: Read MISSIONMD.INI metadata
  │     │           ├─ MP: Init houses from lobby + Create_Houses
  │     │           ├─ Create TacticalMap
  │     │           ├─ Load Theater
  │     │           ├─ Load Terrain + Overlays
  │     │           ├─ Parse Variables
  │     │           ├─ Load Side MIX + UIMD.INI
  │     │           ├─ ReadINI (full object+house parsing)
  │     │           ├─ AI + Scripting pipeline (~20 calls)
  │     │           ├─ Fog + Shroud + Terrain passability
  │     │           ├─ Tech Tree + Superweapons
  │     │           ├─ Post_Map_Init (MP only: starting units + lobby-gated initial crates)
  │     │           ├─ Building state update
  │     │           ├─ Scenario-bit-gated MapClass cell pass (not crate placement)
  │     │           └─ return 1
  │     │
  │     ├─ Post_Load_Init (terrain recalc, AI, particles)
  │     ├─ Wait_For_Players (multiplayer sync)
  │     └─ Per-house finalization loop
  │
  ├─ Queue VQA movie (if specified)
  ├─ Toggle display resolution
  ├─ Start elapsed timer
  ├─ Queue/fade scenario music (automatic or specified theme; later Theme AI starts it)
  └─ Set game-active flags
```

---

## Confidence Summary

| Item | Confidence | Basis |
|------|-----------|-------|
| Call chain order (Start→Read→ReadINI→Full_Init) | **HIGH** | Direct decompilation with call graph |
| Full_Init sub-call sequence (70+ calls) | **HIGH** | Complete decompiled function with string refs |
| Clear_All destruction sequence | **HIGH** | Debug strings confirm each phase |
| Campaign vs MP branching on GameMode | **HIGH** | Multiple if/else on DAT_00a8b238, confirmed values |
| Progress bar values (3→30→...→98) | **HIGH** | Literal constants in FUN_0069ae90 calls |
| ScenarioClass field offsets | **HIGH** | Cross-referenced with constructor, ReadINI, Save/Load |
| SessionClass::GameMode values (0-5) | **HIGH** | Multiple switch/if chains across 4+ functions |
| Reload loop in Full_Init | **MEDIUM** | do/while structure visible but param semantics inferred |
| TMCJ4F.INI cheat file purpose | **MEDIUM** | Only loaded in skirmish+debug, triggers loaded |
| Some FUN_* identity (Phases 15-17) | **MEDIUM** | Called but not named; inferred from context |
