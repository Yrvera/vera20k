# Credits Counter System — Ghidra Analysis

Reverse-engineered from `gamemd.exe` via Ghidra MCP decompilation.
Covers the complete credits display: counting animation, rendering,
background art, sound effects, and observer mode.

Source file: `D:\ra2mdpost\Credits.CPP`

## Key Functions

| Address      | Name                        | Role                                         |
|--------------|-----------------------------|----------------------------------------------|
| `0x004a2350` | `CreditsClass__Init`        | Constructor — zeroes all fields               |
| `0x004a2600` | `CreditsClass__AI`          | Per-frame tick — animates displayed → target  |
| `0x004a2370` | `CreditsClass__Draw`        | Renders background SHP + text number          |
| `0x006d0e60` | `DrawCreditsSHPBackground`  | Draws CREDITS.SHP frame 0                     |
| `0x006d0a30` | (caller)                    | Sidebar draw — calls background + Draw + AI   |
| `0x00750920` | (sound)                     | Plays CreditUp/CreditDown tick sound          |

## CreditsClass Memory Layout

Embedded in the sidebar object at offset `+0x551C`.
Static instance at `0x00A83E18`.

| Offset | Type  | Field          | Purpose                                      |
|--------|-------|----------------|----------------------------------------------|
| +0x00  | `i32` | `target`       | Actual player credits (from economy system)  |
| +0x04  | `i32` | `displayed`    | Currently shown value (animates toward target)|
| +0x08  | `u8`  | `dirty`        | Needs-redraw flag                            |
| +0x09  | `u8`  | `counting_up`  | True if counting up (selects CreditUp sound) |
| +0x0A  | `u8`  | `animating`    | True while value is changing (triggers sound)|
| +0x0C  | `i32` | `direction`    | 1=up, 3=down (vestigial — always overwritten)|

## Counting Animation (`CreditsClass__AI` at `0x004a2600`)

Called every game frame from the sidebar message handler at `0x006d0680`.
Always called with `param_2 = 0` (normal animated mode).

### Algorithm

```c
// 1. Read actual player credits via vtable call
target = house->GetCredits();   // [player + 0x24] vtable offset 0x18
if (target < 0) target = 0;     // clamp negative

// 2. Skip if no change
if (target == displayed && !force) return;

// 3. Force mode (param_2 != 0): instant update, no animation
if (force) {
    displayed = target;
    animating = false;
    goto mark_dirty;
}

// 4. Normal mode: geometric decay toward target
diff = target - displayed;

// Step = |diff| / 8, clamped to [1, 143]
step = abs(diff) >> 3;          // SAR (signed arithmetic right shift)
if (step < 1)   step = 1;      // minimum: 1 credit per frame
if (step > 143) step = 143;    // maximum: 143 credits per frame

// Apply direction
if (target < displayed) step = -step;

// Update
displayed += step;

// Set animation flags
if (displayed actually changed) {
    animating = true;
    counting_up = (step > 0);
}

mark_dirty:
    dirty = true;
    sidebar_needs_redraw = true;    // DAT_00884b90 = 1
```

### Convergence Behavior

This is **geometric decay** — each frame reduces the remaining gap by 7/8.
The `[1, 143]` clamp ensures it always progresses and never overshoots wildly.

At 15 fps (default game speed):

| Credit change | First step | Frames to converge | Time      |
|---------------|------------|---------------------|-----------|
| 8             | 1          | 8                   | ~0.5s     |
| 100           | 12         | ~18                 | ~1.2s     |
| 1,000         | 125        | ~28                 | ~1.9s     |
| 10,000        | 143 (max)  | ~85                 | ~5.7s     |

The effect is the distinctive C&C "fast start, slow finish" counter feel.

### Timing

- Called every game frame — no delay before counting starts.
- At 15 fps, counting ticks 15 times per second.
- No frame-skip or delta-time compensation — tied to game frame rate.

### Force Mode

When `param_2 != 0`, `displayed` is set to `target` instantly:
- Used at game start / map load
- `animating` is cleared (no sound plays)
- Still marks dirty for redraw

## Rendering (`CreditsClass__Draw` at `0x004a2370`)

Called every frame from the sidebar draw path. Only draws when
`param_2 != 0` (force redraw) OR `dirty != 0`.

### Background: CREDITS.SHP

`DrawCreditsSHPBackground` at `0x006d0e60`:
- Draws `DAT_00b0fb08` (CREDITS.SHP) frame 0 via `DrawSHP`
- Position: `(0, 0)` within the credits area surface
- The credits area rect (`DAT_00b0fc58`) is positioned by the sidebar
  layout system — maps to the top of the sidebar chrome
- Drawn every frame before the text

CREDITS.SHP is loaded during sidebar initialization by `FUN_0072fa10`
via `FUN_004a38d0` (SHP load). It provides the dark background strip
behind the credits number, themed per side (Allied/Soviet/Yuri chrome).

### Text Rendering

```c
// Position
x = screen_width / 2;        // horizontally centered on full screen
y = 2;                        // 2 pixels from the very top of screen

// Format
sprintf(buf, "%ld", displayed);  // plain integer, no separators, no "$"

// Draw
DrawText(buf, surface, x, y, color, flags=0x4108);
```

### Text Alignment Flags: `0x4108`

| Bit      | Value  | Meaning                    |
|----------|--------|----------------------------|
| `0x0008` | set    | Top-aligned text           |
| `0x0100` | set    | Horizontally centered on X |
| `0x4000` | set    | Draw drop shadow           |

The drop shadow renders the text twice — once offset in black, then
the colored text on top — providing readability against any background.

### Font

GAME.FNT bitmap font (global at `DAT_0089c4d0`):
- 17px cell height, 16px bitmap rows
- Variable-width proportional glyphs
- Same font used for Ready text, tooltips, and all sidebar text

### Text Color

Side-dependent, set by `SetSidebarTextColor` at `0x0072f440`:

| Side    | RGB              | Appearance        |
|---------|------------------|--------------------|
| Allied  | (164, 210, 255)  | Light sky blue     |
| Soviet  | (255, 255, 0)    | Yellow             |
| Yuri    | (255, 255, 0)    | Yellow             |

Same color used for all sidebar text (Ready labels, queue counts, etc.).
Color bytes are at `DAT_00b0fa1c` / `DAT_00b0fa1d`, converted to the
display surface pixel format via bit-shift descriptors at `DAT_008a0dd0`.

## Sound Effects

### CreditTicks from rules.ini

```ini
[AudioVisual]
CreditTicks=CreditUp,CreditDown
```

Parsed in `FUN_006691e0` (rules loader) at `~0x0066a9B0`:
- Sound list stored at `g_RulesClass + 0x6CC` (TypeList vtable)
- Sound index array at `g_RulesClass + 0x6D0`
  - Element [0] = CreditUp sound index
  - Element [1] = CreditDown sound index
- Entry count at `g_RulesClass + 0x6DC`

### Sound Playback

In `CreditsClass__Draw`, before rendering text:

```c
if (animating && credit_ticks_count >= 2) {
    int sound_index;
    if (counting_up)
        sound_index = credit_ticks[0];   // CreditUp
    else
        sound_index = credit_ticks[1];   // CreditDown

    PlaySound(sound_index, volume=0.5, pan=center);
    // 0x3F000000 = 0.5f IEEE 754
    // 0x2000 = 8192 = center pan
}
```

### Sound Timing

- Sound plays **every frame** that `animating` is set — no throttle
- At 15 fps: up to 15 sound triggers per second during counting
- Volume: 50% (`0x3F000000` = 0.5f)
- Pan: center (`0x2000` = 8192)
- The rapid-fire playback creates the characteristic C&C credit tick sound

## Observer Mode

When the local player IS the observer (`DAT_00a83d4c == DAT_00ac1198`):

### Time Calculation

```c
// Game elapsed time from scenario timer
start_frame = g_Scenario[0x614];    // start timestamp
accumulated = g_Scenario[0x61C];    // accumulated ticks

if (start_frame != -1) {
    current = timeGetTime() >> 4;   // ~16ms units
    elapsed = accumulated + (current - start_frame);
} else {
    elapsed = accumulated;
}

seconds = elapsed / 60;            // convert to seconds
```

### Time Display

```c
target = seconds;
displayed = seconds;    // no counting animation in observer mode

if (seconds / 3600 > 99)
    seconds = 359999;   // cap at 99:59:59

hours   = seconds / 3600;
minutes = (seconds % 3600) / 60;
secs    = (seconds % 3600) % 60;

// CSF key: TXT_TIME_FORMAT_HOURS
// Typical format: "%01d:%02d:%02d"
format = StringTable__LoadString("TXT_TIME_FORMAT_HOURS");
swprintf(buf, format, hours, minutes, secs);
```

### Observer Differences

- `displayed` = `target` always (no counting animation)
- No CreditTick sounds play
- Same position, color, font, and flags as normal credits
- Time capped at `99:59:59`

## Sidebar Integration

### Call Chain

```
SidebarClass__Draw (0x006a6c30)
  └─ FUN_006d0a30 (sidebar main draw)
       ├─ DrawCreditsSHPBackground (0x006d0e60)  — CREDITS.SHP
       ├─ CreditsClass__Draw (0x004a2370)        — text number
       └─ StripClass__Draw (0x006a9540)          — cameos
```

### Dirty Flag Propagation

When credits change, `CreditsClass__AI` sets:
1. `dirty = 1` on the CreditsClass instance
2. `DAT_00884b90 = 1` (global sidebar redraw flag)
3. Calls `FUN_004f42f0` to mark the sidebar surface dirty

This ensures the sidebar redraws the credits area on the next frame.

### Scenario Timer Integration

Fields at `g_Scenario + 0x11E8` (timer start) and `+ 0x11F0` (duration)
force credits dirty when an active timer is running. This ensures the
credits display keeps updating during timed game events even if credits
haven't changed.

## Key Constants Summary

| Constant                  | Value  | Notes                                  |
|---------------------------|--------|----------------------------------------|
| Credits struct offset     | +0x551C| Within sidebar object                  |
| Text position X           | sw/2   | Centered on full screen width          |
| Text position Y           | 2      | 2px from screen top                    |
| Text flags                | 0x4108 | Center + top-align + drop shadow       |
| Format string             | `%ld`  | Plain integer, no separators           |
| Min counting step         | 1      | Credits per frame                      |
| Max counting step         | 143    | Credits per frame (0x8F)               |
| Step divisor              | 8      | step = \|diff\| >> 3                   |
| Sound volume              | 0.5    | 50% (0x3F000000 IEEE 754)              |
| Sound pan                 | 0x2000 | Center (8192)                          |
| Observer time cap         | 99:59:59 | 359999 seconds                       |
| Observer CSF key          | TXT_TIME_FORMAT_HOURS | Format: %01d:%02d:%02d |
| Font                      | GAME.FNT | 17px cell height bitmap font        |
| CreditTicks rules key     | CreditTicks | =CreditUp,CreditDown            |
| CreditTicks list offset   | +0x6CC | In g_RulesClass (TypeList)             |
| CreditTicks array offset  | +0x6D0 | Sound index array                      |
| CreditTicks count offset  | +0x6DC | Must be >= 2 for sound to play         |
