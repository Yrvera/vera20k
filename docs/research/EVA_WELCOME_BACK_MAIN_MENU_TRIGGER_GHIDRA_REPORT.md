# EVA "Welcome back, Commander" Main-Menu Trigger - Ghidra Report

Date: 2026-05-18

Scope: Identify the gamemd.exe function that plays an EVA "Welcome back,
Commander" voice cue on entering the main-menu shell. Identify the EVA
enum/event ID, the audio asset filename, the gate (first entry per
launch / per shell-enter / per session), and whether the cue is
YR-active by default.

Status: Read-only investigation. No writes to gamemd.exe. No edits
to Rust code or to other in-repo docs.

## Executive Summary

**There is no main-menu "Welcome back, Commander" EVA cue in
gamemd.exe.** The premise is incorrect: standard Yuri's Revenge
does not voice any line on main-menu entry. The only audio that
fires on entering the shell is the music theme `[INTRO]` (track key
`Drok`, per `thememd.ini`), kicked off by `Main_Game` immediately
before constructing dialog `0xE2`. No `VoxClass::PlayEVA` call exists
anywhere on the main-menu entry path, and no `EVA_Welcome*` string
exists anywhere in the binary's defined-string table.

The closest "welcome" voice line in YR audio (`EVA_EstablishBattlefieldControl`,
index 16 in `evamd.ini`) fires at scenario start, not on shell entry,
and is out of scope for this report.

## Evidence

### 1. No EVA_Welcome* string exists in gamemd.exe

`mcp__ghidra-mcp__search_strings` for `EVA_` returns 80 matches —
the full EVA name table at `0x008189c0..0x008425a4`. None of them
contain "Welcome." A separate substring search for `Welcome` /
`welcome` / `elcome` returns only seven hits, all CSF/STT labels
unrelated to voice playback:

| Address    | String                       | Role                                                    |
|------------|------------------------------|---------------------------------------------------------|
| `0x00833e00` | `STT:WelcomeUpdate`        | CSF status-tooltip label for WOL `Welcome Update` button |
| `0x00833e24` | `STT:WOLWelcomeBack`       | CSF status-tooltip label for WOL ctrl `0x686`            |
| `0x00833e38` | `STT:WOLWelcomeMyInformation` | CSF status-tooltip label for WOL ctrl `0x6e4`         |
| `0x00833e54` | `STT:WOLWelcomeWDT`        | CSF status-tooltip label for WOL ctrl `0x6e3`            |
| `0x00833e68` | `STT:WOLWelcomeCustomMatch`| CSF status-tooltip label for WOL ctrl `0x6e1`            |
| `0x00833e84` | `STT:WOLWelcomeQuickMatch` | CSF status-tooltip label for WOL ctrl `0x6e0`            |
| `0x0084a180` | `GUI:WelcomeUpdate`        | CSF label set on WOL Persona Info dialog static `0x797`  |

Xref consumers verified: all six `STT:WOL*` strings are returned by
`FUN_006040b0` as the hover-tooltip lookup keyed on dialog control
id. `GUI:WelcomeUpdate` is set as text on the WOL Persona Information
dialog by `FUN_0077a9f0`. None of these are passed to
`VoxClass::PlayEVA` or any audio function.

### 2. Main-menu entry path contains no PlayEVA call

The main-menu shell is created and pumped by `FUN_00531CC0` (dialog
`0xE2` proc; see `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`),
which is the only entry-point on the main-menu side. Its only
voice/sound call is `VocClass__PlayAtPos(0x3f800000, 0)`, reached
exclusively through the `PENGO`/cheat-code keystroke matcher — not a
welcome cue, just a click acknowledgement on a cheat string.

`FUN_00531CC0` is itself called from `Main_Game @ 0x0052D9A0`. Before
invoking the menu pump, `Main_Game` calls:

```text
uVar4 = FUN_00721210("INTRO");   // theme lookup
FUN_00720bb0(uVar4);              // ThemeClass::Queue / start
```

`"INTRO"` is at `0x008263a8` (verified via `read_memory`); it maps
through `thememd.ini` `[INTRO]` to `Sound=Drok` — i.e. the YR
main-menu music track. This is theme music played through
`ThemeClass`, not voice played through `VoxClass`/EVA. No
`VoxClass::PlayEVA` or `VoxClass::QueueVoice` call occurs on the
path from `Main_Game` entry through the `FUN_00531CC0` message-pump.

### 3. No EVA cue is gated on "main-menu entry" anywhere

`VoxClass::PlayEVA` at `0x00752700` has ~70+ callers (see
`VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md`). None of
those callers reside in `FUN_00531CC0`, `Main_Game`, or any of the
shell-dialog construction functions (`FUN_00622B50`, `FUN_0060CF00`,
`FUN_00622650`, `WM_PAINT_Handler @ 0x00621E90`). The EVA dispatcher
is not invoked from the menu path.

### 4. Theme.ini confirms shell audio is music, not voice

```ini
[INTRO]
Name=THEME:Intro
Sound=Drok
Normal=no
Repeat=yes
```

`Drok` is the music track. There is no "Welcome" entry in either
`sound.ini`/`soundmd.ini` (verified by grep — zero matches for
`Welcome|Wlcm|welcm`).

## Per-Question Answers

| Question                                          | Answer                                                                                  |
|---------------------------------------------------|-----------------------------------------------------------------------------------------|
| Function that plays welcome EVA on shell entry    | Does not exist. No such function.                                                       |
| EVA event ID / enum value                         | None. No `EVA_Welcome*` entry in `evamd.ini`'s index list or binary string table.       |
| Audio asset filename                              | None for a "welcome" cue. The shell plays theme music `Drok` via `[INTRO]`.             |
| Condition gate (first entry / per shell / session)| Not applicable — no cue exists to gate.                                                 |
| YR-active by default?                             | Not applicable — no cue exists.                                                         |

## TS-vs-YR Filter Note

The premise also fails the TS-legacy filter prospectively: even if a
"welcome" voice file existed in some retail asset archive (it does
not in any `evamd.ini` or `soundmd.ini` entry shipped with YR), there
is no code path that would invoke it on main-menu entry. The Drok
theme is the entire audio surface of shell entry.

## Confidence Axes

| Axis                                                          | Confidence | Basis |
|---------------------------------------------------------------|------------|-------|
| Content (no EVA call on shell-entry path)                     | HIGH       | Full decompile of `FUN_00531CC0` and `Main_Game @ 0x0052D9A0` reviewed; no `VoxClass__PlayEVA` / `VoxClass__QueueVoice` invocations on the main-menu construction or message-pump path. Only `VocClass__PlayAtPos` is the PENGO keystroke ACK, a `Main_Game` case-8 load-screen voice, and a campaign post-game pump - none reached during initial shell entry. |
| Identity (these are the right functions)                      | HIGH       | `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` already established `FUN_00531CC0` + dialog proc `0x00531F60` as the standard YR main-menu shell. `Main_Game` is the only caller of `FUN_00531CC0` (verified via `get_function_callers`). |
| Binding (no EVA_Welcome string in the binary)                 | HIGH       | `search_strings("EVA_")` returned 80 hits, exhaustive of the EVA name table; `search_strings("Welcome"/"welcome"/"elcome")` returned only 7 CSF-label hits, all xref-consumed by tooltip/text-field code, not by audio dispatch. |
| Caller-trace (no PlayEVA caller is on shell path)             | HIGH       | `VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md` already enumerated PlayEVA's 40+ callers, none in shell construction. Cross-checked: shell-entry functions are not in that caller list. |

## Open Questions / Adjacencies (not chased)

- `EVA_EstablishBattlefieldControl` (index 16) is the closest
  "welcome"-feeling EVA cue in YR, fired at scenario start. Its
  invocation site is not investigated here (out of scope).
- `VocHandle__Init` is called in `Main_Game` case 8 (load-screen
  scenario init), and `VocClass__PlayAtPos(0x3f800000, &DAT_00a8f300)`
  fires while the scenario CD is being read. This is a load-screen
  voice clip (not a menu welcome) - not investigated here.
- The Drok theme's exact asset chain (`Drok` -> mix archive -> stream)
  is not investigated here; only the absence of a welcome EVA was
  required.

## Implication for the Rust Port

The current Rust port (`app_main_menu_shell_render.rs`) should not
play any EVA voice line on entering the main menu. The only audio
that should fire on shell entry is the `INTRO` theme music (resolved
through `thememd.ini` to the `Drok` track). Any "Welcome back,
Commander"-style cue currently planned or implemented would be a
parity *over-shoot* relative to gamemd.exe, not a correction.

## Status

COMPLETE - the requested cue does not exist in gamemd.exe; reported as
a verified negative finding.
