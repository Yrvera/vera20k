# Hotkey System — Ghidra Reverse Engineering Report

**Binary:** `gamemd.exe` (Yuri's Revenge 1.001)
**Source file (confirmed):** `D:\ra2mdpost\MainLoop.CPP` (input dispatch), `D:\ra2mdpost\UICmnds.cpp` (command handlers)
**Confidence:** HIGH — all findings verified via live Ghidra decompilation of multiple functions

---

## 1. Architecture Overview

The hotkey system has three layers:

```
Windows Messages (WM_KEYDOWN/UP)
       ↓
WWKeyboardClass — circular buffer (256 entries, 16-bit event codes)
       ↓
ProcessKeyboardInput (FUN_0055dee0) — dispatch to commands
       ↓
CommandClass hierarchy — ~89 registered command objects
```

### Key Encoding (16-bit event word)

| Bits   | Mask   | Meaning                          |
|--------|--------|----------------------------------|
| 0–7    | 0x00FF | Virtual key code (Windows VK_*)  |
| 8      | 0x0100 | Shift modifier held              |
| 9      | 0x0200 | Ctrl modifier held               |
| 10     | 0x0400 | Alt modifier held                |
| 11     | 0x0800 | Key-up event (release)           |

This encoding is used both in the event buffer and in the KEYBOARDMD.INI binding values.
For example: `TeamCreate_1=561` → 561 = 512 + 49 = 0x0231 = Ctrl(0x200) + '1'(0x31).

---

## 2. Input Capture Pipeline

### WWKeyboardClass::WindowProc (FUN_0054f790, 953 bytes)

Dispatches Windows messages into the circular key buffer:

| Message              | Code   | Action                                      |
|----------------------|--------|---------------------------------------------|
| WM_KEYDOWN           | 0x100  | Enqueue VK code + modifiers (if not repeat) |
| WM_KEYUP             | 0x101  | Enqueue VK code + 0x800 (key-up flag)       |
| WM_SYSKEYDOWN        | 0x104  | Same as KEYDOWN (captures Alt combos)       |
| WM_SYSKEYUP          | 0x105  | Same as KEYUP                               |
| WM_CHAR              | 0x102  | Enqueue as event type 10 + character code   |
| WM_LBUTTONDOWN/UP    | 0x201+ | Mouse events (type 1/2/4 + X,Y coords)     |

**Special cases in EnqueueKeyEvent (FUN_0054f200):**
- **Alt+Tab (VK 0x09 with Alt):** Silently dropped — prevents accidental alt-tab
- **Pause mode:** When game is paused (`DAT_00a8ed9c`), ONLY Escape (0x1B) is enqueued; all other keys are dropped
- **Scroll Lock repeat:** Ignored

### Circular Buffer

- 256 entries of `ushort` at object offset +0x114
- Read pointer at +0x314, write pointer at +0x318
- Wraps with `(ptr + 1) & 0xFF`
- `PeekEvent` (FUN_0054f000): non-blocking peek at next event
- `GetNextEvent` (FUN_0054f050): blocking dequeue
- `Flush` (FUN_0054f720): drains buffer (`read = write`), called before cinematics/menus

---

## 3. Keyboard Input Dispatch (FUN_0055dee0, 636 bytes)

Called every frame from the main game loop (FUN_0055d360).

### Dispatch Flow

```
1. Call ProcessChatInput (FUN_0055e420) — handles chat key interception
2. Read raw key event from buffer
3. Strip modifier bits:
   - uVar2 = raw & 0xFFFFF7FF   (remove key-up bit only)
   - uVar6 = raw & 0xFFFFE0FF   (remove ALL modifier+keyup bits = base VK code)
4. FIRST LOOKUP: search hotkey table for base VK code (uVar6)
   - If found: call command->AcceptsModifiers(uVar2)
   - If AcceptsModifiers returns true → dispatch this command
   - If AcceptsModifiers returns false → fall through
5. SECOND LOOKUP: search hotkey table for key+modifiers (uVar2)
   - If found → dispatch this command
6. If no command found → handle built-in hardcoded keys
7. Dispatch: check debug category, call CanExecute, Execute, IsRepeatable
```

This two-pass lookup is how the same physical key can have different behaviors with and without modifiers. For example, pressing '1' finds `TeamSelect_1` in the first pass, but Ctrl+'1' fails the first pass's `AcceptsModifiers` check, then the second pass finds `TeamCreate_1` (key value 561 = Ctrl+'1').

### Hotkey Table (DAT_0087f680)

- Sorted array of 8-byte entries: `{ uint32 keycode, CommandClass* command }`
- Binary search via FUN_0055f6e0 with qsort on first access
- MRU cache at DAT_0087f690 for O(1) repeat lookups
- Populated from KEYBOARDMD.INI at startup

### Debug Command Filtering

Before executing a command, the dispatch checks `command->GetCategory()` (vtable+0xC). If it returns "Debug" (compared via FUN_007dd0f8), the command is skipped unless debug mode is active (`DAT_00a8ed9c != 0`).

### Repeatable Key Handling

After executing a command, `IsRepeatable()` (vtable+0x1C) is checked. If true, up to 10 duplicate key events are dequeued from the buffer — this prevents key repeat from flooding the command system.

---

## 4. Hardcoded Keys (NOT from KEYBOARDMD.INI)

These keys are handled directly in code and cannot be rebound by the player.

### In ProcessKeyboardInput (FUN_0055dee0)

Reached only if no command was found in the hotkey table:

| VK Code | Key        | Action                                                  |
|---------|------------|---------------------------------------------------------|
| 0x1B    | Escape     | Opens game menu (FUN_00647040)                          |
| 0x09    | Tab        | Toggle sidebar visibility (FUN_006abc40)                |
| 0x25    | Left Arrow | Set scroll-left flag (DAT_00abce14 \|= 0x100)          |
| 0x26    | Up Arrow   | Set scroll-up flag (DAT_00abce14 \|= 0x001)            |
| 0x27    | Right Arrow| Set scroll-right flag (DAT_00abce14 \|= 0x1000)        |
| 0x28    | Down Arrow | Set scroll-down flag (DAT_00abce14 \|= 0x010)          |

Arrow key-up events (bit 0x800 set) clear the corresponding flag.

**Scroll direction bitmask (DAT_00abce14):**

| Bit    | Direction |
|--------|-----------|
| 0x0001 | Up        |
| 0x0010 | Down      |
| 0x0100 | Left      |
| 0x1000 | Right     |

### In ProcessChatInput (FUN_0055e420)

Checked BEFORE the hotkey table lookup, intercepted when in-game (not in main menu):

| VK Code | Key       | Action                                        |
|---------|-----------|-----------------------------------------------|
| 0x0D    | Enter     | Open team chat (DAT_00abce18 = 1)             |
| 0x5C    | Backslash | Open all-chat (DAT_00abce18 = 3)              |
| 0x08    | Backspace | Open ally chat (DAT_00abce18 = 2)             |

**Note:** The character translation function (FUN_0054f450) also maps:
- **Home key (VK 0x24)** → character 0x08 (Backspace) → opens ally chat
- **End key (VK 0x23)** → character 0x5C (Backslash) → opens all chat

### Special Forced Rebindings

At the end of Register_Game_Commands (FUN_00532150), three keys are forcibly bound regardless of KEYBOARDMD.INI:

| Key Code | Key    | Forced Command           |
|----------|--------|--------------------------|
| 0x2E     | Delete | DeleteCommandClass       |
| 0x1B     | Escape | OptionsCommandClass      |
| 0x20     | Space  | CenterREventCommandClass |

These override any prior binding from KEYBOARDMD.INI for these key codes.

### Win32 RegisterHotkey

From the game window initialization (report 151):
- **Ctrl+Alt+Shift+M** (VK 0x4D): Registered via Win32 `RegisterHotKey()` — purpose: likely developer/debug shortcut

---

## 5. CommandClass Hierarchy

### Vtable Layout (verified from decompilation)

| Offset | Method              | Returns/Does                                      |
|--------|---------------------|---------------------------------------------------|
| +0x00  | Destructor          | Cleanup                                           |
| +0x04  | GetName()           | `const char*` — INI key name (e.g., "GuardObject")|
| +0x08  | GetDescription()    | `const char*` — human-readable description        |
| +0x0C  | GetCategory()       | `const char*` — category (e.g., "Interface", "Debug", "Team", "Taunt", "Selection") |
| +0x10  | (unknown)           | Possibly GetUIName or GetTooltip                  |
| +0x14  | AcceptsModifiers(key)| `bool` — whether this command accepts the given modifier state |
| +0x18  | CanExecute(key)     | `bool` — whether the command can execute right now|
| +0x1C  | IsRepeatable(key)   | `bool` — whether key-repeat should be consumed    |
| +0x20  | Execute(key)        | `void` — perform the action                      |

### Object Layout

- **Simple commands:** 4 bytes (just a vtable pointer)
- **Parameterized commands:** 8 bytes (vtable pointer + `uint32` parameter, e.g., team number 1–10 or taunt number 1–8)

---

## 6. Complete Registered Command List

All commands are registered in `Register_Game_Commands` (FUN_00532150, 7118 bytes). Registration order below matches the binary.

### Debug-Only Commands (gated by DAT_00a8b8b4)

| # | Class Name                    | INI Name | Description |
|---|-------------------------------|----------|-------------|
| 1 | MultiplayerDebugCommandClass  | (debug)  | Multiplayer debug mode toggle |
| 2 | MultiplayerSyncCommandClass   | (debug)  | Force sync check |

### Simple Commands (no parameter)

| # | Class Name (from RTTI/vtable)     | INI Name           | Default Key | Description |
|---|-----------------------------------|--------------------|-------------|-------------|
| 3 | FollowCommandClass                | Follow             | F           | Camera follows selected unit |
| 4 | View1CommandClass                 | View1              | F1          | Jump to bookmark 1 |
| 5 | View2CommandClass                 | View2              | F2          | Jump to bookmark 2 |
| 6 | View3CommandClass                 | View3              | F3          | Jump to bookmark 3 |
| 7 | View4CommandClass                 | View4              | F4          | Jump to bookmark 4 |
| 8 | SetView1CommandClass              | SetView1           | Ctrl+F1     | Set bookmark 1 |
| 9 | SetView2CommandClass              | SetView2           | Ctrl+F2     | Set bookmark 2 |
| 10| SetView3CommandClass              | SetView3           | Ctrl+F3     | Set bookmark 3 |
| 11| SetView4CommandClass              | SetView4           | Ctrl+F4     | Set bookmark 4 |
| 12| OptionsCommandClass               | Options            | Escape      | Open options/game menu |
| 13| SidebarUpCommandClass             | SidebarUp          | Numpad 8    | Scroll sidebar up |
| 14| SidebarDownCommandClass           | SidebarDown        | Numpad 2    | Scroll sidebar down |
| 15| CenterREventCommandClass          | CenterOnRadarEvent | Space       | Jump to last radar event |
| 16| BeaconPlacementCommandClass       | PlaceBeacon        | B           | Place map beacon (MP) |
| 17| ToggleSellCommandClass            | ToggleSell         | L           | Toggle sell cursor mode |
| 18| ToggleRepairCommandClass          | ToggleRepair       | K           | Toggle repair cursor mode |
| 19| AllianceCommandClass              | ToggleAlliance     | A           | Toggle alliance with target (MP) |
| 20| CenterBaseCommandClass            | CenterBase         | H           | Center camera on Construction Yard |
| 21| CenterViewCommandClass            | CenterView         | Numpad 5    | Center camera on selection |
| 22| ScatterCommandClass               | ScatterObject      | X           | Scatter selected units |
| 23| GuardCommandClass                 | GuardObject        | G           | Guard area command |
| 24| StopCommandClass                  | StopObject         | S           | Stop all selected units |
| 25| AllToCheerCommandClass            | AllToCheer         | C           | All selected units play cheer anim |
| 26| DeployCommandClass                | DeployObject       | D           | Deploy/unload selected unit |
| 27| PrevObjectCommandClass            | PreviousObject     | M           | Cycle to previous unit |
| 28| NextObjectCommandClass            | NextObject         | N           | Cycle to next unit |
| 29| PlanningModeCommandClass          | PlanningMode       | Z           | Toggle waypoint planning mode |
| 30| CombatantSelectCommandClass       | CombatantSelect    | P           | Select all combat units on screen |
| 31| TypeSelectCommandClass            | TypeSelect         | T           | Select all units of same type |
| 32| HealthNavCommandClass             | (unbound)          | —           | Navigate units by health (no default key) |
| 33| VeterancyNavCommandClass          | VeterancyNav       | Y           | Navigate units by veterancy |
| 34| SetStructureTabCommandClass       | StructureTab       | Q           | Switch sidebar to Structures tab |
| 35| SetDefenseTabCommandClass         | DefenseTab         | W           | Switch sidebar to Defense tab |
| 36| SetUnitTabCommandClass            | UnitTab            | R           | Switch sidebar to Units tab |
| 37| SetInfantryTabCommandClass        | InfantryTab        | E           | Switch sidebar to Infantry tab |

### Parameterized Commands: Team Control (8-byte objects)

| # Range | Class Name              | INI Name Pattern     | Default Keys    | Description |
|---------|-------------------------|----------------------|-----------------|-------------|
| 38–47   | CreateTeamCommandClass  | TeamCreate_1–10      | Ctrl+1 – Ctrl+0 | Assign selected units to team N |
| 48–57   | SelectTeamCommandClass  | TeamSelect_1–10      | 1 – 0           | Select team N (double-tap = center camera) |
| 58–67   | AddTeamCommandClass     | TeamAddSelect_1–10   | Shift+1 – Shift+0 | Add selected units to team N |
| 68–77   | CenterTeamCommandClass  | TeamCenter_1–10      | Alt+1 – Alt+0   | Center camera on team N |

### Parameterized Commands: Taunts (8-byte objects)

| # Range | Class Name          | INI Name Pattern | Default Keys | Description |
|---------|---------------------|------------------|--------------|-------------|
| 78–85   | TauntCommandClass   | Taunt_1–8        | F5 – F12     | Send taunt message 1–8 (MP) |

### Additional Standalone Commands

| #  | Class Name                   | INI Name        | Default Key  | Description |
|----|------------------------------|-----------------|--------------|-------------|
| 86 | ScreenCaptureCommandClass    | ScreenCapture   | Shift+S      | Take screenshot |
| 87 | PageUserCommandClass         | PageUser        | U            | Page/whisper a player (MP) |
| 88 | CursorPositionCommandClass   | (unbound)       | —            | Display cursor position (no default key) |
| 89 | DeleteCommandClass           | Delete          | Numpad Del   | Delete selected unit (force-kill own unit) |

**Total: 89 registered commands** (2 debug + 37 simple + 40 team + 8 taunt + 2 unbound)

---

## 7. KEYBOARDMD.INI Format

```ini
[Hotkey]
CommandName=KeyValue
```

Key values are encoded as: `modifier_bits + VK_code`

| Modifier | Bit Value | Decimal Offset |
|----------|-----------|----------------|
| None     | 0x000     | +0             |
| Shift    | 0x100     | +256           |
| Ctrl     | 0x200     | +512           |
| Alt      | 0x400     | +1024          |

### Loading Process (FUN_00533d20, 549 bytes)

1. Construct CCINIClass, open `KEYBOARDMD.INI`
2. If file not found: print "Unable to load KEYBOARDMD.INI", return
3. Clear hotkey binding array (DAT_0087f680–0087f690)
4. For each entry in `[Hotkey]` section:
   a. Read command name string (the INI key)
   b. Read key value integer (the INI value)
   c. Iterate all registered commands, compare names via `command->GetName()`
   d. On match: store `(keycode, command_ptr)` in sorted binding array
5. Mark array as unsorted (sorted on first lookup via qsort)

---

## 8. Hotkey Display Name Formatting (FUN_0061ef70, 535 bytes)

Converts a key code with modifier bits into a human-readable string like "Ctrl+Shift+F5".

### Process:
1. If key-up bit (0x800) is set → return empty string
2. If Alt (0x400): `MapVirtualKeyA(VK_MENU)` → `GetKeyNameTextA` → append "Alt+"
3. If Ctrl (0x200): `MapVirtualKeyA(VK_CONTROL)` → `GetKeyNameTextA` → append "Ctrl+"
4. If Shift (0x100): `MapVirtualKeyA(VK_SHIFT)` → `GetKeyNameTextA` → append "Shift+"
5. Append base key name: `MapVirtualKeyA(VK & 0xFF)` → `GetKeyNameTextA`
6. Result written via sprintf to output buffer

The separator string between modifiers and key is at DAT_00835a34 ("+").

**Note:** The order is always Alt → Ctrl → Shift → Key (if multiple modifiers present).

---

## 9. Team Hotkey Double-Tap Detection (FUN_007311c0)

When a team number key is pressed (TeamSelect_N):

1. Read `timeGetTime()` and compare to last team-assign timestamp (DAT_00845550)
2. If elapsed < **800ms** AND same team number as last press (DAT_00845554):
   - **Double-tap detected** → search backwards through unit roster for first alive unit in this team → center camera on it
3. If NOT double-tap (or first press):
   - Record timestamp and team number
   - Clear current selection display
   - Select all units assigned to this team number
   - Suppress selection sound during bulk select, re-enable after

---

## 10. Chat System Keys (FUN_0055e420, 3507 bytes)

Chat input is processed BEFORE the hotkey table lookup.

| Character | Source Key(s)       | Chat Mode | Description |
|-----------|---------------------|-----------|-------------|
| 0x0D (CR) | Enter               | 1 (Team)  | Open chat to teammates only |
| 0x5C (\\) | Backslash, End key  | 3 (All)   | Open chat to all players |
| 0x08 (BS) | Backspace, Home key | 2 (Ally)  | Open chat to allies only |

Chat modes (DAT_00abce18): 0=closed, 1=team, 2=ally, 3=all

**Additional chat features:**
- Page command: send private message to specific player
- Reply command: reply to last received private message
- Beacon messages: special map ping messages
- Observer restriction: observers cannot send chat in standard mode

---

## 11. Attack-Move Hotkey (FUN_00731af0 / FUN_00731bf0)

Attack-move is triggered by a key combination check, NOT from the standard hotkey table:

1. `FUN_00731bf0` checks four key codes stored at `DAT_00a8ec00..0c` (likely Ctrl+Shift or a configurable pair)
2. If BOTH modifier pairs have at least one key pressed AND all selected units support attack-move (`vtable+0x4C0`):
   - Sets `DAT_00b0fe58 = 1` (attack-move cursor active)
3. Cancel: right-click or Escape clears the flag

If no units are selected, displays "MSG:NothingSelected".
If any selected unit cannot attack-move, displays "MSG:AttackMoveUnsupported".

---

## 12. Key Address Reference

| Address      | Name / Global                          | Purpose |
|--------------|----------------------------------------|---------|
| 0x00532150   | Register_Game_Commands                 | Allocates and registers all 89 commands |
| 0x00533d20   | Load_Hotkeys                           | Loads KEYBOARDMD.INI into binding table |
| 0x00533f50   | Execute_Command_By_Name                | Lookup command by name string and execute |
| 0x0054f000   | PeekEvent                              | Non-blocking peek at key buffer |
| 0x0054f050   | GetNextEvent (blocking)                | Dequeue key event from buffer |
| 0x0054f200   | EnqueueKeyEvent                        | Add key event to buffer with modifiers |
| 0x0054f450   | TranslateVKey                          | Convert VK + modifiers → ASCII char |
| 0x0054f790   | WWKeyboardClass::WindowProc            | Windows message handler for input |
| 0x0055d360   | MainGameLoop                           | Calls ReadKeyboard + ProcessInput each frame |
| 0x0055dee0   | ProcessKeyboardInput                   | Main hotkey dispatch function |
| 0x0055e420   | ProcessChatInput                       | Chat key interception (Enter/\/BS) |
| 0x0055f6c0   | CheckMRUCache                          | O(1) hotkey cache check |
| 0x0055f6e0   | BinarySearchHotkeyTable                | Binary search hotkey lookup |
| 0x00565090   | KeyboardClass::Constructor             | Initialize keyboard handler |
| 0x0061ef70   | FormatHotkeyName                       | Build "Ctrl+Shift+F5" display string |
| 0x007311c0   | TeamHotkeyHandler                      | Team select with double-tap center |
| 0x00731af0   | EnterAttackMoveMode                    | Validate and activate attack-move cursor |
| DAT_0087f65c | Command vector (data pointer)          | Array of CommandClass* pointers |
| DAT_0087f668 | Command vector (count)                 | Number of registered commands |
| DAT_0087f680 | Hotkey binding table (data pointer)    | Sorted array of (keycode, command*) pairs |
| DAT_0087f684 | Hotkey binding table (count)           | Number of active bindings |
| DAT_0087f690 | MRU cache pointer                      | Last-looked-up binding entry |
| DAT_00abce14 | Scroll direction bitmask               | Arrow key scroll state |
| DAT_00abce18 | Chat mode                              | 0=off, 1=team, 2=ally, 3=all |
| DAT_00845550 | Last team-assign timestamp             | For double-tap detection (800ms window) |
| DAT_00845554 | Last team-assign number                | Which team was last selected |

---

## 13. Default Key Binding Quick Reference

### Unit Commands
| Key | Command | Description |
|-----|---------|-------------|
| G   | GuardObject | Guard area |
| S   | StopObject | Stop all movement/actions |
| D   | DeployObject | Deploy/unload unit |
| X   | ScatterObject | Scatter units |
| C   | AllToCheer | Units play cheer animation |
| F   | Follow | Camera follows unit |

### Selection & Navigation
| Key | Command | Description |
|-----|---------|-------------|
| T   | TypeSelect | Select all same type (screen/map toggle) |
| P   | CombatantSelect | Select all combat units |
| Y   | VeterancyNav | Navigate by veterancy rank |
| M   | PreviousObject | Cycle to previous unit |
| N   | NextObject | Cycle to next unit |
| Tab | (hardcoded) | Toggle sidebar |
| Numpad Del | Delete | Force-kill own unit |

### Camera & Bookmarks
| Key | Command | Description |
|-----|---------|-------------|
| H   | CenterBase | Center on Construction Yard |
| Space | CenterOnRadarEvent | Jump to last radar event |
| Numpad 5 | CenterView | Center on selection |
| F1–F4 | View1–4 | Jump to bookmark 1–4 |
| Ctrl+F1–F4 | SetView1–4 | Set bookmark 1–4 |
| Arrows | (hardcoded) | Scroll map |

### Sidebar Tabs
| Key | Command | Description |
|-----|---------|-------------|
| Q   | StructureTab | Buildings tab |
| W   | DefenseTab | Defense tab |
| E   | InfantryTab | Infantry tab |
| R   | UnitTab | Units/vehicles tab |
| Numpad 8 | SidebarUp | Scroll sidebar up |
| Numpad 2 | SidebarDown | Scroll sidebar down |

### Building Commands
| Key | Command | Description |
|-----|---------|-------------|
| K   | ToggleRepair | Toggle repair cursor |
| L   | ToggleSell | Toggle sell cursor |

### Team Control
| Key | Command | Description |
|-----|---------|-------------|
| 0–9 | TeamSelect_N | Select team (double-tap = center) |
| Ctrl+0–9 | TeamCreate_N | Assign selection to team |
| Shift+0–9 | TeamAddSelect_N | Add selection to team |
| Alt+0–9 | TeamCenter_N | Center camera on team |

### Multiplayer
| Key | Command | Description |
|-----|---------|-------------|
| A   | ToggleAlliance | Toggle alliance with target |
| B   | PlaceBeacon | Place map beacon |
| U   | PageUser | Page/whisper player |
| F5–F12 | Taunt_1–8 | Send taunt message |
| Enter | (hardcoded) | Open team chat |
| Backslash | (hardcoded) | Open all chat |
| Backspace | (hardcoded) | Open ally chat |

### Other
| Key | Command | Description |
|-----|---------|-------------|
| Escape | Options | Open game menu |
| Z   | PlanningMode | Toggle waypoint planning |
| Shift+S | ScreenCapture | Take screenshot |

---

## 14. Unbound Commands (registered but no default key)

| Class Name                 | INI Name | Notes |
|----------------------------|----------|-------|
| HealthNavCommandClass      | —        | Navigate by health; no default binding in KEYBOARDMD.INI |
| CursorPositionCommandClass | —        | Display cursor coordinates; no default binding |
| MultiplayerDebugCommandClass | — | Debug only, gated by DAT_00a8b8b4 |
| MultiplayerSyncCommandClass  | — | Debug only, gated by DAT_00a8b8b4 |

---

## 15. Real-Time Modifier Keys (Right-Click Command Modifiers)

These are NOT part of the hotkey table system. They are checked via `GetAsyncKeyState` (FUN_0054f5c0) every frame to modify how right-click commands behave. Each modifier uses a pair of VK codes (left/right variants) stored in globals.

### Modifier Key Globals

| Global Pair | Modifier | Effect on Right-Click | Effect on Scroll |
|-------------|----------|----------------------|------------------|
| DAT_00a8ebf8 / DAT_00a8ebfc | **Shift** | Queue waypoint (add to path) | — |
| DAT_00a8ec00 / DAT_00a8ec04 | **Ctrl** | Force-fire (attack ground/allies) | Fast scroll speed |
| DAT_00a8ec08 / DAT_00a8ec0c | **Alt** | Force-move (ignore enemies) | Maximum scroll speed |

### Right-Click Command Priority (FUN_006ffec0, 700 bytes)

When the player right-clicks, the modifier state determines the action:

```
1. Check Shift (DAT_00a8ebf8/fc) → bVar3 = "queue" flag
2. Check Ctrl (DAT_00a8ec00/04) → bVar1 = "force-fire" flag
3. Check Alt (DAT_00a8ec08/0c):
   - If Alt AND Ctrl held → clears Ctrl flag (Alt overrides Ctrl)
   - If Alt only → returns action 8 (FORCE_MOVE) if target is friendly
4. If Shift + own unit target → returns action 1 (QUEUE_WAYPOINT)
5. If Ctrl → force-attack action
6. Otherwise → default move/attack based on target type
```

### Scroll Speed Modification (in MainGameLoop FUN_0055d360)

Arrow-key scroll speed is dynamically modified by the same modifier keys:

```c
scroll_speed = base_scroll_speed;  // DAT_0082a030 (from options slider)

if (Alt_held) {
    scroll_speed = FUN_007c5f00();  // maximum speed
} else if (Ctrl_held) {
    scroll_speed = max(DAT_0087f8dc, DAT_0087f8e0) << 8;  // fast speed
}

// Apply scroll in each held direction
if (scroll_left)  ScrollMap(LEFT,  scroll_speed);
if (scroll_right) ScrollMap(RIGHT, scroll_speed);
if (scroll_up)    ScrollMap(UP,    scroll_speed);
if (scroll_down)  ScrollMap(DOWN,  scroll_speed);
```

**Alt takes priority over Ctrl** for scroll speed.

### Attack-Move Activation (FUN_00731bf0)

Attack-move requires BOTH Ctrl AND Alt held simultaneously:

```c
bool ctrl_held = IsKeyDown(DAT_00a8ec00) || IsKeyDown(DAT_00a8ec04);
bool alt_held  = IsKeyDown(DAT_00a8ec08) || IsKeyDown(DAT_00a8ec0c);

if (ctrl_held && alt_held) {
    // Check all selected units support attack-move (vtable+0x4C0)
    if (all_can_attack_move) return ATTACK_MOVE_ACTIVE;
}
```

---

## 16. Sidebar Command Bar (Click-Based Commands)

The sidebar/command bar provides mouse-clickable buttons that invoke the same commands as keyboard hotkeys. These are defined in the `TabClass` constructor (FUN_006cfe20).

### Sidebar Command Name Table (PTR_DAT_008427d0)

An 11-entry table of command name strings used for sidebar button dispatch:

| Index | Command Name | Global Storing Index | Click Handler |
|-------|-------------|---------------------|---------------|
| — | "AttackMove" | DAT_00b0cd24 | FUN_00731af0 (enter attack-move mode) |
| — | "Beacon" | DAT_00b0cb3c | FUN_00731a30 (toggle beacon display) |
| — | "Cheer" | DAT_00b0c1b8 | FUN_00730f30 (record cheer command) |
| — | "Deploy" | DAT_00b0cb20 | FUN_00730af0 (guard/deploy validation) |
| — | "Guard" | DAT_00b0cb68 | FUN_00730d60 (deploy command) |
| — | "PlanningMode" | DAT_00b0cc1c | FUN_00731a70 (enter) / FUN_00731a50 (exit) |
| — | "Stop" | DAT_00b0cb6c | FUN_00730ea0 (scatter command) |
| — | Team slot 1 | DAT_00b0cc20 | Team create/add/select logic |
| — | Team slot 2 | DAT_00b0cc28 | Team create/add/select logic |
| — | Team slot 3 | DAT_00b0cd28 | Team create/add/select logic |
| — | "TypeSelect" | DAT_00b0cb38 | FUN_00732950 (select across map) |

### Sidebar Button Layout

- 25 button strips loaded from `Button00.SHP` through `ButtonNN.SHP`
- Expandable sidebar with "thumb" button (expand/collapse toggle)
- Planning mode button has a special "toggled" visual state
- Team buttons use display type 0x55 (team button visual style)
- Tooltip strings: `"Tip:AttackMove"`, `"Tip:Beacon"`, `"Tip:Deploy"`, `"Tip:Guard"`, `"Tip:PlanningMode"`, `"Tip:Stop"`, `"Tip:TypeSelect"`, `"Tip:Team01"`–`"Tip:Team03"`, `"Tip:ThumbOpen"`, `"Tip:ThumbClosed"`

### Sidebar Command Dispatch (FUN_006d0680)

Button clicks generate command codes in the range 0x80D6–0x80EE. The dispatch:
- Subtracts 0xD6 from the code to get the command index
- Compares against each stored command global (DAT_00b0cd24, etc.)
- Calls the corresponding handler function
- Team buttons have special 3-state logic: `FUN_00730a10` (count) → `FUN_00731060` (create) / `FUN_007311c0` (add) / `FUN_007313a0` (select)
- Ctrl+click on team buttons (code range 0xC0D6–0xC0EE) calls `FUN_007310d0` (deassign team)

---

## 17. HealthNav and VeterancyNav Command Details

### HealthNav (FUN_00733380)

Cycles through three health categories and selects matching units:

| Cycle State | Filter | EVA String ID | Message |
|-------------|--------|---------------|---------|
| 0 | Healthy | 0x644 | "MSG:Healthy" |
| 1 | Damaged | 0x646 | "MSG:HeavilyDamaged" |
| 2 | Critical | 0x648 | "MSG:Critical" |

- Uses selection mode `DAT_00b0fe54 = 3`
- State tracked in `DAT_00845560` (cycles -1 → 0 → 1 → 2 → 0...)
- Calls `FUN_005f5dd0` (GetHealthStatus) on each unit
- First press: filters from current selection; subsequent presses: cycles category
- Shows "MSG:Mixed" if selection contains mixed health

### VeterancyNav (FUN_007336c0)

Structurally identical to HealthNav but for experience levels:

| Cycle State | Filter | EVA String ID | Message |
|-------------|--------|---------------|---------|
| 0 | Rookie | 0x68E | "MSG:LittleExperience" |
| 1 | Veteran | 0x690 | "MSG:Veteran" |
| 2 | Elite | 0x692 | "MSG:Elite" |

- Uses selection mode `DAT_00b0fe54 = 4`
- State tracked in `DAT_00845564`
- Calls `FUN_00750030` (GetVeterancyLevel) on each unit

---

## 18. Edge Scrolling (Mouse-Based)

### FUN_00734120 — Process edge-scroll based on mouse position

Separate from arrow-key scrolling. Triggered when the mouse cursor reaches the screen edge.

- Uses `timeGetTime()` for speed control
- If < 1600ms since last scroll: fast scroll mode (`FUN_00660be0`)
- Otherwise: slow scroll mode (`FUN_00660bb0`)
- Converts cell coordinates to pixel position, calls `FUN_006d6070` to scroll viewport

### FUN_00734210 — Reset scroll timers

Resets timing state so next scroll triggers slow-scroll mode. Called on camera jumps (bookmarks, team center, etc.).

---

## 19. Band-Select (Drag Selection)

### Timing Thresholds

| System | Threshold | Global | Purpose |
|--------|-----------|--------|---------|
| Band-select | **501ms** | FUN_00732cc0 | Drag < 501ms = valid selection box; longer = camera scroll |
| Double-tap team | **800ms** | DAT_00845550 | Second press within 800ms = center camera |

### Band-Select Flow

1. Mouse-down: `FUN_00732ca0` sets `DAT_00b0fe65 = 1`, records `timeGetTime()`
2. Mouse-up: `FUN_00732cc0` checks elapsed time
   - If < 501ms: calls `FUN_00732950` (select across map) + redraw
   - If >= 501ms: treated as camera scroll, no selection
3. During drag: rendering code checks `DAT_00b0fe65` to draw selection rectangle
4. Distance threshold: band-select only activates if mouse moved > 4 pixels from start

---

## 20. Planning Mode Mouse Interaction

When planning mode is active (`DAT_00ac4cf4 = 1`), mouse events are intercepted:

| Action | Handler | Behavior |
|--------|---------|----------|
| Right-click on node | FUN_0063a8e0 | Delete/toggle loop on selected node |
| Right-click elsewhere | FUN_0063a8e0 | Check insertion, clear selection |
| Mouse-down | FUN_0063aac0 | Record position, set drag flag |
| Mouse-up | FUN_0063ab00 | If moved < 4px: click-to-deselect loop target |
| Mouse-move | FUN_0063ab60 | Hit-test nodes (inflated by 4px), play highlight sound |

### Planning Mode Limits

- Maximum 128 nodes per player (`"MSG:PlannerMaximum"`)
- Network command type 0x2B sent on exit
- Tutorial messages shown on first use: `"MSG:PlanningModeIntro1Button"` / `"MSG:PlanningModeIntro1Key"`

### Planning Mode String Messages

- `MSG:PlanningModeIntro1Button` / `MSG:PlanningModeIntro1Key` — intro (button vs key entry)
- `MSG:PlanningModeIntro3` — additional intro
- `MSG:PlanningModeLoopHelp` — loop creation help
- `MSG:PlanningModeDeleteHelp` — delete help
- `MSG:PlanningModeHeteroSel` — heterogeneous selection warning
- `MSG:PlanningModeNoDeploy` / `MSG:NoStop` / `MSG:NoScatter` — invalid commands
- `MSG:PlanningModeNoGuardArea` — guard area unavailable
- `MSG:PostTerminatingCommand` / `MSG:PostContinualCommand` — command queueing

---

## 21. Network Replay Input Virtualization

During networked play or replay, the input system is virtualized to ensure deterministic behavior.

### Virtual Keyboard State Table (DAT_00aa0168)

- 256-entry array of `ushort` (one per VK code)
- Value: `0x8000` when pressed, `0x0000` when released
- Controlled by `DAT_00aa0444` (network mode):
  - **0** = inactive (use real `GetKeyState`/`GetAsyncKeyState`)
  - **1** = recording/live (capture real input, relay as network events)
  - **2** = playback/replay (feed from network event stream)

### Key Functions

| Address | Function | Purpose |
|---------|----------|---------|
| 0x0053ec40 | SetVirtualKeyState | Write to virtual state table |
| 0x0053ec70 | VirtualGetKeyState | Read from virtual table (or real GetKeyState) |
| 0x0053ec90 | VirtualGetAsyncKeyState | Read from virtual table (or real GetAsyncKeyState) |
| 0x0053e770 | NetworkPeekMessage | Network message pump (1156 bytes) |

### Replay Mode Behavior

During replay (mode 2):
- **Only Escape (0x1B) is accepted** from real keyboard to abort replay
- All other real keyboard input is discarded
- Game input comes from the recorded network event stream
- Mouse position replayed via event type 0x14
- Keyboard/window messages replayed via event type 0x28

---

## 22. Additional Hardcoded Keys and Edge Cases

### Scroll Lock Filter (VK 0x91)

At the Windows message handler level (FUN_0054f790), Scroll Lock key events are **explicitly dropped** and never enter the key buffer. This is the only VK code filtered at the message level.

### Alt+Tab Suppression

In EnqueueKeyEvent (FUN_0054f200), if the key is Tab (VK 0x09) and Alt is held, the function returns immediately without enqueuing. This allows Windows Alt+Tab to work without the game intercepting it.

### Escape Key in Multiple Contexts

Escape is handled in at least 4 separate places:

| Context | Address | Behavior |
|---------|---------|----------|
| Hotkey table fallback | 0x0055dee0 | Opens game menu (FUN_00647040) |
| Sim tick | 0x006474b0 | Exits game session (`DAT_00a8e9a0 = 0`) |
| Movie playback | 0x00759510 | Skip VQA movie (checks key-up: 0x81B) |
| DirectDraw pump | 0x0053e7e0 | Terminate loading wait loop |
| Chat input | 0x0055e420 | Cancel chat (`DAT_00abce18 = 0`) |
| Pause override | 0x0054f200 | Only key that bypasses pause block |

### Pause Mode Input Blocking

When game is paused (`DAT_00a8ed9c != 0`):
- **Keyboard:** Only Escape (0x1B) is enqueued; all other keys silently dropped
- **Mouse:** All mouse events blocked (EnqueueMouseEvent checks pause flag)

### Selection Mode State Machine (DAT_00b0fe54)

| Value | Mode | Triggered By |
|-------|------|-------------|
| 0 | Normal | Default / after any nav command completes |
| 1 | Select Across Screen | First press of TypeSelect (T) |
| 2 | Select Across Map | Second press of TypeSelect |
| 3 | Health Navigation | HealthNav command |
| 4 | Veterancy Navigation | VeterancyNav command |

---

## 23. WWKeyboardClass Object Layout

| Offset | Size | Type | Purpose |
|--------|------|------|---------|
| +0x00 | 4 | int | Current mouse X / last key code |
| +0x04 | 4 | int | Current mouse Y |
| +0x08 | 2 | short | Pending WM_CHAR character |
| +0x0C | 8 | int[2] | Stored mouse X/Y (copy) |
| +0x14 | 16 | byte[16] | Keyboard state array (256 bits, for ToAscii) |
| +0x24 | 1 | byte | Shift key state (0x80 = pressed) |
| +0x25 | 1 | byte | Ctrl key state |
| +0x26 | 1 | byte | Alt key state |
| +0x52 | 2 | short | Default key (0x0D = Enter) |
| +0x114 | 512 | ushort[256] | Circular event buffer |
| +0x314 | 4 | int | Read pointer (0–255) |
| +0x318 | 4 | int | Write pointer (0–255) |

### Event Buffer Entry Types

| Low Byte | Type | Extra Data |
|----------|------|-----------|
| 1 | Left mouse button | +2 words: X, Y coordinates |
| 2 | Right mouse button | +2 words: X, Y coordinates |
| 4 | Middle mouse button | +2 words: X, Y coordinates |
| 10 | WM_CHAR character | +1 word: character code |
| Other | Raw VK key code | No extra data |

---

## 24. Complete Address Reference (Extended)

### Input Pipeline
| Address | Name | Purpose |
|---------|------|---------|
| 0x0054ee60 | WWKeyboardClass::Constructor | Initialize keyboard buffer |
| 0x0054ee90 | GetNextEvent | Blocking read from buffer |
| 0x0054f000 | PeekEvent | Non-blocking peek |
| 0x0054f050 | GetNextEvent_Blocking | Blocking dequeue (variant) |
| 0x0054f1c0 | EnqueueRaw | Write 16-bit value to buffer |
| 0x0054f200 | EnqueueKeyEvent | Add key with modifiers to buffer |
| 0x0054f2f0 | EnqueueMouseEvent | Add mouse event to buffer |
| 0x0054f450 | TranslateVKey | Full VK → ASCII translation |
| 0x0054f530 | TranslateVKey_Simple | Simplified VK → ASCII |
| 0x0054f5c0 | IsKeyDown | Real-time key state query |
| 0x0054f650 | PeekRaw | Peek without consuming |
| 0x0054f6b0 | IsFull | Check buffer full |
| 0x0054f6d0 | IsEmpty | Check buffer empty |
| 0x0054f720 | Flush | Drain all buffered input |
| 0x0054f790 | WWKeyboardClass::WindowProc | Windows message handler |
| 0x00565090 | KeyboardClass::Constructor | Initialize keyboard handler |
| 0x00565190 | KeyboardClass::Reset | Reset keyboard state |

### Hotkey System
| Address | Name | Purpose |
|---------|------|---------|
| 0x00532150 | Register_Game_Commands | Register all 89 commands |
| 0x00533d20 | Load_Hotkeys | Load KEYBOARDMD.INI |
| 0x00533f50 | Execute_Command_By_Name | Lookup + execute by name string |
| 0x0055dee0 | ProcessKeyboardInput | Main hotkey dispatch |
| 0x0055e420 | ProcessChatInput | Chat key interception |
| 0x0055f6c0 | CheckMRUCache | O(1) hotkey cache |
| 0x0055f6e0 | BinarySearchHotkeyTable | Binary search lookup |
| 0x0061ef70 | FormatHotkeyName | Build display string |

### Command Handlers (UICmnds.cpp)
| Address | Name | Purpose |
|---------|------|---------|
| 0x00730af0 | GuardCommand_Execute | Guard area validation + execute |
| 0x00730d60 | DeployCommand_Execute | Deploy/unload validation |
| 0x00730ea0 | ScatterCommand_Execute | Scatter validation (ACTION_SCATTER=6) |
| 0x00730fe0 | StopCommand_Execute | Stop validation (ACTION_STOP=7) |
| 0x00731060 | TeamAssign | Toggle unit assignment to team slot |
| 0x007310d0 | TeamDeassign | Clear all units from team |
| 0x007311c0 | TeamHotkeyHandler | Team select + double-tap center |
| 0x007313a0 | TeamRecall | Select all team members + center |
| 0x007314c0 | TeamAddSelect | Additive team select (no center) |
| 0x007315a0 | ScrollToNextUnit | Tab-key unit cycling |
| 0x00731a10 | SelectAllToggle | Select-all / deselect-all |
| 0x00731a30 | ToggleBeaconDisplay | Toggle beacon overlay |
| 0x00731a50 | PlanningMode_Exit | Exit planning mode |
| 0x00731a70 | PlanningMode_Enter | Enter planning mode |
| 0x00731af0 | EnterAttackMoveMode | Validate + activate attack-move |
| 0x00731bf0 | IsAttackMoveActive | Query attack-move state |
| 0x00732280 | SelectAcrossScreen | Select same type on screen |
| 0x00732950 | SelectAcrossMap | Select same type on map |
| 0x00733380 | HealthNav_Execute | Cycle health filter selection |
| 0x007336c0 | VeterancyNav_Execute | Cycle veterancy filter selection |

### Sidebar Command Bar
| Address | Name | Purpose |
|---------|------|---------|
| 0x006cfcc0 | CommandFromName | Lookup command by name in table |
| 0x006cfe20 | TabClass_Constructor | Init sidebar + resolve 11 commands |
| 0x006d0680 | CommandBar_Dispatch | Main sidebar click handler |
| 0x006d0fd0 | StripLayout | Sidebar button layout |
| 0x006d1200 | FullStripRelayout | Expanded sidebar layout |
| 0x006d04f0 | ThumbToggle | Sidebar expand/collapse |

### Planning Mode
| Address | Name | Purpose |
|---------|------|---------|
| 0x006379c0 | EnterPlanningMode | Set flag + play EVA |
| 0x00637a10 | ExitPlanningMode | Send network packet + reset |
| 0x00637aa0 | IsPlanningModeActive | Getter for DAT_00ac4cf4 |
| 0x0063a8e0 | PlanMode_RightClick | Right-click handler in plan mode |
| 0x0063aac0 | PlanMode_MouseDown | Mouse-down handler |
| 0x0063ab00 | PlanMode_MouseUp | Mouse-up handler |
| 0x0063ab60 | PlanMode_MouseMove | Mouse-move / hover handler |

### Network Replay
| Address | Name | Purpose |
|---------|------|---------|
| 0x0053e770 | NetworkPeekMessage | Network message pump |
| 0x0053ec40 | SetVirtualKeyState | Write virtual key state |
| 0x0053ec70 | VirtualGetKeyState | Read virtual key state |
| 0x0053ec90 | VirtualGetAsyncKeyState | Read virtual async key state |

### Global Data
| Address | Name | Purpose |
|---------|------|---------|
| DAT_0087f65c | Command vector data | Array of CommandClass* |
| DAT_0087f668 | Command vector count | Number of registered commands |
| DAT_0087f680 | Hotkey table data | Sorted (keycode, cmd*) pairs |
| DAT_0087f684 | Hotkey table count | Number of bindings |
| DAT_0087f690 | MRU cache | Last-looked-up binding |
| DAT_00a8ebf8/fc | Shift VK codes | Shift modifier pair (queue waypoint) |
| DAT_00a8ec00/04 | Ctrl VK codes | Ctrl modifier pair (force-fire) |
| DAT_00a8ec08/0c | Alt VK codes | Alt modifier pair (force-move) |
| DAT_00a8ed9c | Pause flag | Blocks all input except Escape |
| DAT_00aa0168 | Virtual key state table | 256-entry replay state |
| DAT_00aa0444 | Network mode | 0=off, 1=live, 2=replay |
| DAT_00abce14 | Scroll direction bitmask | Arrow key scroll flags |
| DAT_00abce18 | Chat mode | 0=off, 1=team, 2=ally, 3=all |
| DAT_00ac4cf4 | Planning mode active | 1=active, 0=inactive |
| DAT_00b0fe54 | Selection mode | 0-4 state machine |
| DAT_00b0fe58 | Attack-move flag | 1=cursor active |
| DAT_00845550 | Team-assign timestamp | Double-tap detection (800ms) |
| DAT_00845554 | Team-assign number | Last team selected |
| DAT_00845560 | HealthNav cycle state | -1/0/1/2 health filter |
| DAT_00845564 | VeterancyNav cycle state | -1/0/1/2 rank filter |

---

## 25. Left-Click System (Complete Dispatch Chain)

Left-click is the primary command-issuing input. The dispatch chain is 9 layers deep.

### Overview Flow

```
WM_LBUTTONDOWN (0x201)
  → WindowProc enqueues event to buffer
  → FUN_006930a0 (Tactical::MouseButtonHandler)
    → FUN_00692300 (Screen-to-Cell: what's under cursor?)
    → FUN_0063a5a0 (sidebar intercept?)
    → DisplayClass__DetermineAction (compute action code)
    → DisplayClass__SetCursorFromAction (update cursor graphic)
    → FUN_004ac310 (save drag-start for band-box)
    → SetCapture(hWnd)

Mouse held (every tick in FUN_00692f30):
  → FUN_004ac380 (Update band-box if dragging > 4px)

WM_LBUTTONUP (0x202)
  → FUN_006930a0 case 0x202
    → FUN_0063a8e0 (sidebar up-click intercept?)
    → FUN_00692300 (what's under cursor at release?)
    → DisplayClass__DetermineAction (action at release)
    → DisplayClass__BandBox_LeftUp (EXECUTE COMMAND)
    → ReleaseCapture()
```

### Layer 1: Tactical Mouse Button Handler (FUN_006930a0, verified)

**Guard checks before processing:**
- `DAT_00a8e378 != 0` (game in playable state)
- `DAT_00a8ed5c != 0` or `DAT_00a8ed6b != 0` (SP or MP active)
- `g_GameActive != 0` (game not frozen)
- `g_Tactical != 0` and `g_DisplayChain != NULL` (map loaded)
- `DAT_00a8ed9c == 0` (not in modal dialog) — exception: WM_CAPTURECHANGED (0x215) bypasses this

**Dispatch by message type:**

| Message | Code | Actions |
|---------|------|---------|
| WM_LBUTTONDOWN | 0x201 | Screen-to-cell → sidebar check → DetermineAction → SetCursor → StartBandBox → SetCapture |
| WM_LBUTTONUP | 0x202 | Sidebar check → Screen-to-cell → DetermineAction → **ExecuteCommand** → ReleaseCapture |
| WM_RBUTTONDOWN | 0x204 | Sidebar handler → Screen-to-cell → save coords → SetCapture |
| WM_RBUTTONUP | 0x205 | Sidebar handler → execute right-click → ReleaseCapture |
| WM_CAPTURECHANGED | 0x215 | Abort action if window lost capture |

### Layer 2: Screen-to-Cell Conversion (FUN_00692300)

Converts screen pixel coordinates to map data:
- **Output:** cell coordinates, 3D position, object under cursor, visibility flags
- Calls `FUN_006d6590` (pixel-to-cell), `FUN_006d2280` (sub-cell precision)
- Checks shroud/fog via `FUN_00586360`/`FUN_005865e0`
- Gets object at position via `FUN_006da380`
- Filters dead objects, spy planes, objects player can't see

### Layer 3: Action Determination (DisplayClass__DetermineAction)

The central decision-maker. Returns an **action code** based on:
1. Current cursor mode (sell/repair/power/guard/waypoint)
2. What's under the cursor (unit, building, terrain, bridge)
3. Selected units' capabilities (vtable methods)

**Modal mode priority (checked first):**

| Global | Mode | Action Code |
|--------|------|-------------|
| DAT_00880998 | Sell mode | 0x0F (sell) or 0x0A |
| DAT_0088099a | Power mode | 0x22/0x21 (power toggle) |
| DAT_0088099b | Waypoint mode | 0x2B–0x30 (waypoint/targeting) |
| DAT_0088099c | Repair mode | 0x3C (repair) |
| DAT_00880999 | Guard mode | 0x0C–0x0E (guard area) |
| DAT_008809a0 | Special weapon targeting | Override from vtable+0x6c |

**Key action codes:**

| Code | Action | Left-Click Behavior |
|------|--------|---------------------|
| 0 | No action | Select target if valid, else clear selection |
| 1 | Force move | Issue move command to cell |
| 2 | Attack | Issue attack command on target |
| 7 | Select | Select unit under cursor |
| 8 | Select/Enter | Select or enter transport/garrison |
| 0x0A | Spy enter | Spy infiltrates building |
| 0x0C | Engineer capture | Engineer enters building |
| 0x0D | Engineer repair | Engineer repairs bridge |
| 0x0E | Guard area | Set guard area |
| 0x0F | Sell | Sell building/unit |
| 0x14 | Garrison | Enter civilian building |
| 0x21 | Toggle power on | Restore power to building |
| 0x22 | Toggle power off | Cut power to building |
| 0x2B | Superweapon target | Target cell for superweapon |
| 0x33 | Force move variant | Same as force move |
| 0x3C | Beacon placement | Place map beacon |
| 0x3D | Enter bridge | Unit crosses bridge |
| 0x45, 0x46 | No action (disabled) | Click does nothing |

### Layer 4: Command Execution (DisplayClass__BandBox_LeftUp, FUN_004ab9b0, 2298 bytes)

This is the core command executor, called on mouse-up.

**Building placement** (`param_1[0x469] != 0`):
- If a building is being placed (pending from sidebar), validates placement via `FUN_004a8eb0`
- Creates network command packet (type 0x0B) via `FUN_004c6ae0`
- Queues in command buffer (128 entries × 0x6F bytes, circular, timestamped)
- Clears placement state

**Band-box select** (`offset 0x11CF != 0`):
- Checks **Shift key** (VK 0x10) via `FUN_0054f5c0(0x10)`:
  - Shift NOT held: clear existing selection, then select units in box
  - Shift held: additive select (keep existing selection, add new units)
- Calls `Tactical__ProcessBandBoxSelection` to select all units within the drag rectangle
- Calls `FUN_0070d150` (refresh sidebar/UI)
- Clears band-box state, returns early

**Action 8 (select/enter):**
- If target is player-controlled and NOT already selected:
  - Check `FUN_00732d00()` (double-click timing flag):
    - **Single click:** `vtable+0x14C` (Select single unit)
    - **Double click:** `vtable+0x88` (group select) + `FUN_007327d0` (select all same type on screen)
- If target IS already selected (offset +0x83 set):
  - **Single click:** `vtable+0x150` (Unselect / deselect)
  - **Double click:** `vtable+0x88` + `FUN_00732600` (deselect all same type)
- Falls through to action 7 if target is not player-controlled

**Action 7 (select) and action 0 (empty ground):**
- If target exists, is selectable (`vtable+0x138`), and not already selected:
  - Deselect current selection, then select the new target
  - Same single-click/double-click logic as action 8
- If no valid target: clear selection

**Action 0x3D (enter bridge):**
- Calls `FUN_00430f70` with cell coordinates converted to leptons

**All other combat/command actions** (not 0, 7, 8, or the filtered set):
- Checks `FUN_00639040()` and `FUN_00639130()` (force-fire and force-move state)
- Calls `FUN_004ae750(target, cellCoords, action)` to issue the command to ALL selected units
- Post-dispatch special cases:
  - Action 0x21 (power toggle): sends network packet type 1 or 2
  - Action 0x3C (beacon): calls `RadarClass__PlaceBeacon`
  - Action 0x0A (spy enter): sends network packet type 0x15
  - Action 0x0D/0x0C (engineer): sends network packet type 0x16 or 0x17

**Network command packet queue:**
- Buffer: `g_CommandBuffer` (128 entries × 0x6F bytes each)
- Write index: `g_CommandQueue_WriteIndex` (circular, masked with 0x7F)
- Count: `g_CommandQueue_Count` (max 128)
- Each entry timestamped via `timeGetTime()`
- Packet types: 0x0B (building place), 0x12 (sidebar), 0x15 (spy), 0x16 (engineer), 0x17 (engineer repair)

### Layer 5: Issue Command to Selected Units (FUN_004ae750)

For each unit in the selection array (`DAT_00a8ecbc`, count `DAT_00a8ecc8`):
- If clicking on terrain (no target): calls `vtable+0x70` (compute mission for cell) then `vtable+0x140` (execute move)
- If clicking on a target: calls `vtable+0x74` (compute mission for target) then `vtable+0x144` (execute order)
- Special handling for force-move (action 1/0x33) and attack-ground (action 2)

### Band-Box Drag Select Details

**Start (FUN_004ac310):**
- Only starts if NO special modes active: no force-fire (0x11B0), no force-move (0x11B2), no planning mode (0x11B3), no superweapon (0x11B8), no special action (0x11A8)
- Sets drag-potential flag at offset +0x11D0
- Saves click position at +0x11D4/+0x11D8

**Update every tick (FUN_004ac380):**
- If drag potential set: checks if mouse moved > **4 pixels** (Euclidean distance)
- If threshold exceeded: sets band-box active (0x11CF = 1), starts rubber-band rendering
- While active: updates corner position, redraws selection rectangle

**Complete on mouse-up (in FUN_004ab9b0):**
- `FUN_0054f5c0(0x10)` checks Shift for additive selection
- `Tactical__ProcessBandBoxSelection` selects all units in rectangle
- Band-box state cleared

### Double-Click Select-All-Same-Type

The engine does NOT use WM_LBUTTONDBLCLK directly (it's expanded to down+up pair). Instead, double-click detection uses a timing flag:

1. On first click: `FUN_00732ca0` sets `DAT_00b0fe65 = 1` and records `timeGetTime()`
2. On second click: `FUN_00732cc0` checks if elapsed < **501ms** — if so, sets the timing flag
3. In `FUN_004ab9b0`: `FUN_00732d00()` returns the flag
   - If true: `vtable+0x88` (group select) + `FUN_007327d0` (select all same type on screen)
   - If false: `vtable+0x14C` (single select)

### Cursor Mode Flags

These globals control what left-click does (set by keyboard hotkeys like K, L, or sidebar buttons):

| Global | Mode | Set By | Left-Click Behavior |
|--------|------|--------|---------------------|
| DAT_00880998 | Sell | ToggleSell (L key) | Click building to sell it |
| DAT_00880999 | Guard | GuardObject (G key) | Click ground to set guard area |
| DAT_0088099a | Power | (sidebar button) | Click building to toggle power |
| DAT_0088099b | Waypoint/Beacon | PlaceBeacon (B key) | Click ground to place beacon |
| DAT_0088099c | Repair | ToggleRepair (K key) | Click building to repair it |
| DAT_008809a0 | Special weapon | (superweapon ready) | Click ground to target superweapon |

### DisplayClass Offsets (Mouse State)

| Offset | Purpose |
|--------|---------|
| +0x555A | Mouse button held flag (1 = left or right held) |
| +0x11A8 | Special action mode |
| +0x11B0 | Force-fire mode |
| +0x11B2 | Force-move mode |
| +0x11B3 | Planning mode / Ctrl state |
| +0x11CF | Band-box select active |
| +0x11D0 | Band-box drag potential |
| +0x11D4/D8 | Band-box start coords |
| +0x11DC/E0 | Band-box current corner |
| +0x0469 | Building placement pending (non-zero = placing) |
| +0x046A | Building type being placed |
| +0x046B | Building facing/variant |
