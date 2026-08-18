# Process-startup GLS splash — Ghidra research report

**Date:** 2026-07-28  
**Binary:** active `gamemd.exe` in Ghidra project `testProsjekt`  
**Primary owner:** `0x005312A0` (currently mislabeled `Init_DropPodAssets`)  
**Confidence:** HIGH for the active 800x600 path, composition, timing, and handoff. MEDIUM for the unavailable 640-specific retail asset's file metadata.  
**Active in Yuri's Revenge:** Yes. This is the process-start splash shown before the first main-menu owner runs.

## Verdict

The observed Yuri's Revenge artwork with `Loading...` in the upper-left is drawn by
`gamemd.exe`, not by the EA launcher and not by the later scenario-loading renderer.

`Init_Game @ 0x0052BA60` calls the startup compositor at `0x005312A0` immediately after
the MIX files become available. The compositor:

1. loads `GLSMD.PAL`;
2. selects `GLSSMD.SHP` only when the configured width is exactly 640, otherwise
   selects `GLSLMD.SHP`;
3. centers frame 0 without scaling;
4. draws five localized `GAME.FNT` strings, including `GUI:LoadingEx` at `(10,10)`;
5. synchronously blits the completed draw surface to the display; and
6. establishes a five-second minimum deadline which `Init_Game` honors after the
   rules/type initialization work.

The active retail `GLSLMD.SHP` was independently extracted from `langmd.mix` and
decoded. It is one opaque 800x600 frame and visually matches the user's reference
artwork exactly. The image does not contain the `Loading...` text; that text is a
separate binary-driven overlay.

Current Rust has no live equivalent. `startup_splash_until` is initialized to `None`
and is never armed; its dormant branch would draw the generic
`main_menu::draw_loading_screen(..., "Initializing client")`, whose image provider
currently returns `None`.

## Scope

In scope:

- normal Yuri's Revenge process startup;
- the first retail splash frame;
- asset and palette selection;
- all text overlays;
- display presentation;
- minimum dwell and main-menu handoff;
- current Rust divergence and an implementation handoff.

Out of scope:

- selected-map and random-map scenario loading;
- launcher/store UI before `gamemd.exe`;
- main-menu animations after the first menu owner takes over;
- implementation or Rust edits.

## 1. Ownership and call order

The verified call chain is:

```text
entry @ 0x007CD80F
  -> WinMain @ 0x006BB9A0
     -> CreateMainWindow @ 0x00777C30
     -> Main_Game @ 0x0048CCC0
        -> Init_Game @ 0x0052BA60
           -> Init_Mix_Files
           -> startup splash compositor @ 0x005312A0
           -> boot/preload work
           -> rules/type initialization
           -> wait until startup-splash deadline
        -> menu/state owner Main_Game @ 0x0052D9A0
```

`0x005312A0` has one caller: `Init_Game @ 0x0052BA60`. There is no game-mode,
skirmish, replay, or map-selection gate around this normal startup call.

The binary's current symbols are polluted:

- `0x005312A0` is labeled `Init_DropPodAssets`, but its body is the complete GLS
  startup splash compositor.
- `0x00531680` is labeled `Show_Loading_Screen`, but its body is a boot asset
  preload routine, including PIP/DPOD-related loads. It does not compose or present
  the startup splash.

Any implementation or future research must use the addresses and verified bodies,
not these two labels.

### Launcher boundary

`WinMain` contains launcher-handshake-shaped calls, but the two relevant helpers at
`0x0049F5C0` and `0x0049F620` are unconditional success stubs in the active binary.
The installed `ra2md.exe` / `RA2Launcher.exe` resources contain icons, dialogs,
strings, version data, and manifests but no matching splash bitmap. The active
startup image owner is therefore inside `gamemd.exe`.

## 2. Asset selection and palette conversion

### Palette

`0x005312A0` constructs a `CCFileClass` for:

```text
GLSMD.PAL @ 0x008268B0
```

If the file opens, it receives the 768-byte palette buffer and loops exactly 256
RGB triplets. Every component is shifted left by two bits in place:

```text
palette[i].r <<= 2
palette[i].g <<= 2
palette[i].b <<= 2
```

The byte operations do not clamp. The converted palette is supplied to a
`ConvertClass` together with the active display conversion globals.

### SHP choice

The selector is an exact equality, not a threshold:

```text
if DAT_008A00A4 == 640:
    file = "GLSSMD.SHP" @ 0x008268A4
else:
    file = "GLSLMD.SHP" @ 0x00826898
```

Thus 800 and every other non-640 width use the large asset.

### Retail asset verification

The installed EA retail archive
`C:/Program Files/EA Games/Command and Conquer Red Alert II/langmd.mix` is a
new-format, checksum-bearing, unencrypted-index MIX with ten entries.

| Asset | MIX ID | Size | Verified metadata |
|---|---:|---:|---|
| `GLSLMD.SHP` | `0xB11D7C29` | 481,232 bytes | 800x600 canvas; 1 frame; frame 0 at `(0,0)`, 800x600, format 2, data offset 32 |
| `GLSMD.PAL` | `0x366CBDFB` | 768 bytes | 256 RGB triplets |
| `GLSSMD.SHP` | binary-proven filename | unavailable in the installed `langmd.mix` | exact frame metadata remains unverified |

All 480,000 decoded pixels in `GLSLMD.SHP` frame 0 are nonzero palette indices, so
the retail large artwork is an opaque full-frame image. Visual decoding with the
verified `component << 2` palette conversion matches the user's reference crop.

The installed EA `langmd.mix` does not contain the `GLSSMD.SHP` hash under the
active CRC, plain CRC, or legacy Westwood hash. This does not change the binary's
verified exact-640 branch, but the small asset's dimensions and archive provenance
remain a documented residual.

## 3. Image placement

After `GetClientRect(g_hWnd)`, the compositor builds the logical client bounds:

```text
client_w = right - left
client_h = bottom - top
```

If the selected SHP loaded, frame 0 is placed at:

```text
x = trunc_toward_zero((client_w - shp_width) / 2)
y = trunc_toward_zero((client_h - shp_height) / 2)
```

The signed truncation is explicit in assembly:

```text
CDQ
SUB EAX, EDX
SAR EAX, 1
```

`CC_Draw_Shape @ 0x004AED70` draws frame 0 with scale argument `1000` and no
runtime scaling branch. For a normal 800x600 client and `GLSLMD.SHP`, the origin is
exactly `(0,0)`.

The draw destination used by the SHP and text calls is the global surface at
`0x0088730C`.

## 4. Text composition

`FUN_004A60D0` returns `g_GAME_FNT`, and the verified font has a 17-pixel line
height. Each text layer is loaded from the CSF and drawn through
`DrawText @ 0x004A60E0` with color `0xFFFF`, zero auxiliary parameter, and style
argument `0x16`.

The installed `ra2md.csf` values are:

| Key | Retail English value |
|---|---|
| `GUI:LoadingEx` | `Loading...` |
| `TXT_COPYRIGHT` | `© 2000, 2001 ELECTRONIC ARTS INC. ALL RIGHTS RESERVED` |
| `GUI:WWBrand` | `WESTWOOD STUDIOS™ IS AN ELECTRONIC ARTS™ BRAND` |
| `GUI:TradeMarkTop` | `Command & Conquer and Yuri's Revenge are trademarks or registered` |
| `GUI:TradeMarkBottom` | `trademarks of Electronic Arts Inc. in the U.S. and/or other countries.` |

### Visual composition ledger

| Draw order | Key | X | Y |
|---:|---|---|---|
| 1 | `TXT_COPYRIGHT` | `client_w - measured_width - 10` | `client_h - 40` |
| 2 | `GUI:WWBrand` | `client_w - measured_width - 10` | `client_h - 40 + 3 + font_height` |
| 3 | `GUI:LoadingEx` | `10` | `10` |
| 4 | `GUI:TradeMarkTop` | `10` | `client_h - 40` |
| 5 | `GUI:TradeMarkBottom` | `10` | `client_h - 40 + 3 + font_height` |

With retail `GAME.FNT`, the two bottom baselines are `client_h - 40` and
`client_h - 20`. At 800x600 these are y=560 and y=580.

The user's screenshot shows the large artwork's upper-left crop plus
`GUI:LoadingEx`; the `(10,10)` native anchor explains the observed inset.

### Failure behavior

- If `GLSMD.PAL` cannot open, the ConvertClass and SHP load path is skipped, but
  the compositor still attempts all text draws, presents the surface, and sets the
  deadline.
- If the SHP lookup fails after palette setup, only the background draw is skipped.
  Text, presentation, and deadline still run.
- `GAME.FNT` is required in practice. Width calculations are partly guarded, but
  the fifth layer reads `g_GAME_FNT + 0x1C` unconditionally; a null font can fault.

## 5. Presentation and minimum dwell

After the five text draws, assembly at `0x005315D7..0x005315E4` supplies:

```text
CL  = 1
EDX = surface 0x0088730C
stack argument = null source rect
call 0x004F4780
```

`0x004F4780`:

1. obtains and screen-translates the window client rectangle;
2. derives the source rectangle when none is supplied;
3. invokes the display-chain pre-copy callback because the boolean argument is
   true;
4. copies the composed surface into display surface `0x00887308`;
5. sleeps 50 ms; and
6. invokes the display-chain post-copy callback.

This is a synchronous hidden/back-surface-to-display blit path, not merely a draw
into an unpresented buffer.

Immediately after that helper returns, `0x005312A0` calls
`GetPerformanceTimestamp @ 0x004093B0`, adds exactly 5000 with 64-bit carry, and
stores the deadline at:

```text
low  = DAT_00A8F788
high = DAT_00A8F78C
```

`Timer__InitPerformanceCounter @ 0x00409360` proves the timestamp unit is
milliseconds:

- preferred path: `QueryPerformanceFrequency / 1000`, then
  `QueryPerformanceCounter / derived_divisor`;
- fallback path: `timeGetTime`.

After boot preloads, `Load_Game_Rules`, animation initialization, and building-type
initialization, `Init_Game` repeatedly samples the same timestamp until it is at
least the stored deadline. Work performed between the draw and this checkpoint
counts toward the five seconds. If the work is faster, the engine busy-waits the
remainder. The presentation helper's 50 ms sleep occurs before the timestamp
anchor, so the visible interval is at least approximately 5.05 seconds, plus any
later initialization before the main menu overwrites it.

Only after successful `Init_Game` completion does `Main_Game @ 0x0048CCC0` call the
menu/state owner at `0x0052D9A0`.

## 6. Distinction from scenario loading

This process-start splash is not `ScenarioClass__DrawLoadingScreen @ 0x00552D60`.

| Process-start splash | Scenario loading |
|---|---|
| `0x005312A0` | `0x00552D60` |
| `GLSLMD.SHP` / `GLSSMD.SHP` | `ls640*` / `ls800*` country SHPs |
| `GLSMD.PAL` | `MPLS*.PAL` family |
| five legal/loading text layers | country, special unit, briefing, loading text |
| no progress bar or player row | progress and player-row presentation |
| normal process initialization | selected scenario/map load |

`GUI:LoadingEx` has callsite evidence in both systems, but the asset families,
owners, coordinates, and lifecycle are different.

## 7. Current Rust divergence

Read-only source inspection found:

- `src/app.rs:524` declares `startup_splash_until: Option<Instant>`.
- `src/app.rs:4159` initializes it to `None`.
- No source assignment arms it; the only later assignment clears it at
  `src/app.rs:4259`.
- The dormant branch at `src/app.rs:4230` calls
  `main_menu::draw_loading_screen(..., "Initializing client")`.
- `src/ui/main_menu.rs:301` implements that generic egui loading panel.
- `src/ui/main_menu.rs:388` returns `None` from `loading_screen_image()`.

Therefore the retail splash is absent every normal startup, not intermittently
hidden. Trigger frequency is every normal launch.

There is no INI authority for this screen. The filenames, selector, coordinates,
palette conversion, string keys, present path, and deadline are all binary-owned.

## 8. Implementation handoff

The smallest faithful Rust slice should:

1. Add a dedicated process-start splash presentation state; do not reuse the
   scenario-loading compositor.
2. Start it only after asset archives, `GLSMD.PAL`, `GAME.FNT`, and CSF strings are
   available, but before the first main-menu frame.
3. Select `GLSSMD.SHP` only for logical width exactly 640; select `GLSLMD.SHP` for
   every other width.
4. Decode frame 0 with `GLSMD.PAL` after the exact per-component `<< 2` conversion.
5. Center the native SHP without scaling, preserving signed truncation toward zero.
6. Draw all five CSF layers with `GAME.FNT` in the verified order and coordinates.
7. Present the fully composed frame before continuing initialization.
8. Anchor a 5000 ms deadline after presentation. Allow initialization work to
   consume that interval, then hold the splash until the deadline if work finished
   early.
9. Transition directly to the real main-menu owner after startup initialization.
10. Remove or bypass the dead generic `"Initializing client"` startup placeholder;
    it is not a faithful fallback for a successful retail asset load.

### Acceptance checks

- At 800x600, frame 0 of `GLSLMD.SHP` starts at `(0,0)` and `Loading...` starts at
  `(10,10)`.
- The artwork is not stretched and contains no egui panel/background treatment.
- The legal/brand strings occupy the two verified bottom baselines.
- The frame is visibly presented during initialization and cannot disappear before
  the five-second deadline.
- The process-start state is independent of selected-map loading progress.
- Missing optional SHP art still leaves the text/present/deadline path active.

## 9. Open-question final state

| Question | State |
|---|---|
| Is the observed screen owned by the launcher? | RESOLVED: no; active compositor is in `gamemd.exe` at `0x005312A0`. |
| Which Ghidra loading label is correct? | RESOLVED: neither label is trustworthy; body/address evidence identifies `0x005312A0`. |
| Is `Loading...` baked into the image? | RESOLVED: no; it is `GUI:LoadingEx`, drawn at `(10,10)`. |
| Is the image scaled? | RESOLVED: no; frame 0 is centered at native size. |
| Which 800x600 asset is used? | RESOLVED: `GLSLMD.SHP`, one 800x600 frame, with `GLSMD.PAL`. |
| What controls screen duration? | RESOLVED: millisecond deadline of presentation timestamp + 5000, honored later in `Init_Game`. |
| Is it the same as the match loading screen? | RESOLVED: no; separate owner, assets, and lifecycle. |
| What are the exact `GLSSMD.SHP` dimensions/archive in the installed EA build? | DEFERRED: filename and exact-640 selector are binary-proven; the file was not present in installed `langmd.mix`, so metadata was not guessed. This does not block the observed 800x600 restoration. |

## Evidence log

Live Ghidra, read-only:

- `decompile_function` / `disassemble_function`:
  - `0x005312A0`
  - `0x0052BA60`
  - `0x0048CCC0`
  - `0x004F4780`
  - `0x00409360`
  - `0x004093B0`
  - `0x004093C0`
  - `0x004093F0`
  - `0x004A60D0`
- `get_xrefs_to 0x0087E848`
- `get_function_callees 0x0048CCC0`
- string/xref checks for `GLSLMD.SHP`, `GLSSMD.SHP`, `GLSMD.PAL`,
  `GUI:LoadingEx`, `TXT_COPYRIGHT`, `GUI:WWBrand`, and the trademark keys.

Retail file checks, read-only:

- parsed `langmd.mix` index and verified `GLSLMD.SHP`, `GLSMD.PAL`, and
  `ra2md.csf`;
- decoded all frames of `GLSLMD.SHP` (one frame total);
- decoded the five referenced English CSF strings;
- decoded the 800x600 artwork with the binary-proven palette conversion and
  visually compared it to the supplied screenshot.

No Rust source, INI file, binary, Ghidra database, or production asset was modified.
No Cargo command was run.
