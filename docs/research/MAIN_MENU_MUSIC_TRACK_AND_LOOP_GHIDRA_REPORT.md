# Main-Menu Music Track Selection and Loop (Ghidra Report)

**Scope:** identify the function(s) in `gamemd.exe` that pick, start, and loop the music
heard while the main menu / shell screen is displayed. Confidence: HIGH on the call chain,
HIGH on the theme key, HIGH on the loop mechanism, MEDIUM on the per-theme struct offset
above 0x28a, MEDIUM on the audio file extension actually opened (the `.WAV` literal at
0x00844768 is what is concatenated; the underlying mix loader is out of scope).

All addresses are RVA in `gamemd.exe` (RA2/YR retail, image base 0x00400000).
Per CLAUDE.md verification discipline, every offset and address below was read directly
from Ghidra in this session.

---

## TL;DR

The main menu music is the `INTRO` theme. In `thememd.ini` (YR) this is
`Sound=Drok`, `Repeat=yes`. It is started by a literal `"INTRO"` string lookup at
the top of the inner `Main_Game` function (RVA 0x0052d9a0), which converts the name
to a theme index via `Theme::From_Name` (FUN_00721210) and queues it for play via
`Theme::Play` (FUN_00720bb0). Looping is handled by per-tick `Theme::AI`
(FUN_007209d0) called from the audio pump (FUN_00406f70): when the stream is no
longer playing it pulls the pending song index from `g_Theme+0x8` and re-queues
it. Because `Repeat=yes` is set on `[INTRO]`, the play path stores the same index
back into `+0x8` after each play, producing an infinite loop.

The same `INTRO` string is re-played on every return to the shell from a game
(scenario exit, WOL exit, options exit — see the four xrefs to `s_INTRO_008263a8`).

YR override: there is no special YR-only branch. The override is purely INI:
`thememd.ini` re-defines `[INTRO]` with `Sound=Drok`, replacing whatever RA2's
`theme.ini` set. The loader (FUN_00720590) reads from the merged `CCINIClass`
([Themes] section), so YR's MD file takes priority via the normal RA2->YR INI
merge.

Active in YR: **Yes** — reached on every standard YR skirmish/campaign launch
through the shell. No `SpecialFlags` gate.

---

## Anchor strings

| Address     | Encoding | Value                         | Role                                          |
|-------------|----------|-------------------------------|-----------------------------------------------|
| 0x008263a8  | ASCII    | `INTRO`                       | Theme key looked up by `Theme::From_Name`     |
| 0x00844760  | ASCII    | `Themes`                      | INI section read by ThemeClass loader         |
| 0x00844768  | ASCII    | `.WAV`                        | Extension appended to per-theme `Sound=` name |
| 0x00844770  | ASCII    | `No theme`                    | Fallback label                                |
| 0x00844748  | ASCII    | `Sound`                       | Per-theme INI key                             |
| 0x0081bb60  | ASCII    | `Normal`                      | Per-theme INI key                             |
| 0x00844740  | ASCII    | `Repeat`                      | Per-theme INI key                             |
| 0x0081b1bc  | ASCII    | `Scenario`                    | Per-theme INI key                             |
| 0x00817854  | ASCII    | (Side)                        | Per-theme INI key (40-byte buffer)            |
| 0x008447b8  | ASCII    | `Theme::AI(Next song = %d)\n` | Debug print confirming ThemeClass identity    |
| 0x008447d4  | ASCII    | `Theme::QueueSong(%d)\n`      | Debug print                                   |
| 0x008447ec  | ASCII    | `Theme::PlaySong(%d) - %s\n`  | Debug print                                   |
| 0x0084481c  | ASCII    | `Theme::Stop(%d)\n`           | Debug print                                   |
| 0x00844830  | ASCII    | `Theme::Stop(%d) - Fading\n`  | Debug print                                   |
| 0x00844814  | ASCII    | `Playing`                     | Status string                                 |
| 0x00844808  | ASCII    | `Repeating`                   | Status string                                 |

---

## Global Theme instance

`g_Theme = 0x00a83d10` — verified at four xrefs to `s_INTRO_008263a8`. Each call
site loads ECX with `0xa83d10` before calling `Theme::From_Name` / `Theme::Play`,
which is the `this` pointer in MSVC `__thiscall` convention.

Two pieces of related Theme global state visible from the call-site assembly:
- `0x00a83d14` — last queued/playing song index (read by `Theme::Play`).
- `0x00a83d18` — `g_Theme_LastQueueIndex` (read in network code at 0x0077b2e2,
  0x0077e188 — outside shell scope, listed only because the xrefs to `INTRO`
  surfaced them).

---

## ThemeClass struct layout (offsets in bytes from `this`)

Read directly from FUN_00720590 (loader) and FUN_00720bb0 (play) and
FUN_007209d0 (AI). `int *` decompile cast warnings have been re-checked — these
are direct byte offsets (the functions sign-cast `param_1` to `char *` for buffer
math).

| Offset | Type    | Field                                              |
|--------|---------|----------------------------------------------------|
| 0x000  | int     | Score (current index, -1 if none) — written by Play|
| 0x004  | int     | Last "Queue song" index — written by Play          |
| 0x008  | int     | Pending/Next song index — read by AI               |
| 0x00C  | int     | Theme array count (used by AI)                     |
| 0x010  | bool    | Global Repeat / play-list flag                     |
| 0x011  | bool    | Fading flag                                        |
| 0x014  | void*   | (used by `Set_Volume`-style accessor at FUN_00407b40)|
| 0x018  | void**  | Theme entry pointer array                          |
| 0x01C  | int     | Theme entry array capacity                         |
| 0x020  | (unknown) | likely allocator state                           |
| 0x024  | int     | Theme entry array count (allocated entries)        |
| 0x028  | int     | Cap for Add (read by FUN_00720590 grow path)       |
| 0x02C  | void*   | StreamPlayer pointer                               |

## Per-Theme entry struct (size 0x290)

Read from FUN_00720590 (allocator/zero-init) and FUN_00720480 (per-section INI
read).

| Offset | Type     | Field          | Notes                                          |
|--------|----------|----------------|------------------------------------------------|
| 0x000  | char[256]| Theme key      | e.g. `INTRO` — copied from `[Themes]` value    |
| 0x100  | char[256]| Sound filename | from `Sound=`; `$` / `#` prefix stripped       |
| 0x200  | char[64] | Side filter    | from `Side=` (only read if Normal=yes)         |
| 0x280  | int      | Scenario       | from `Scenario=`                               |
| 0x288  | bool     | Normal         | default true; gates Side read                  |
| 0x289  | bool     | Repeat         | default false                                  |
| 0x28A  | (unknown)| —              | zeroed at init; unknown                        |
| 0x28C  | int      | Length-like int| set via FUN_004756f0(`&DAT_00817334`); default -1|
| 0x28F  | byte     | —              | -1 init                                        |
| 0x290  | —        | (end of struct)|                                                |

---

## Call chain for shell music

### Loader (one-time, during game-startup Init)

`FUN_00720590` is called from inside the game-startup function at 0x0052ba60
(Ghidra mislabels this `CCFileClass__Constructor` — verified via decompilation it
is actually `Init_Game` / startup sequence; the label is not trustworthy).

- Call site: 0x0052c92a (`CALL FUN_00720590`) — immediately follows
  `CALL FUN_00720770` which is `ThemeClass::Clear` / freeing the prior array.
- Reads section `Themes` (string at 0x00844760) from the merged CCINI.
- For each numbered key (`1=`, `2=`, ...), the value is the theme key name. The
  loader allocates a 0x290-byte struct, copies the name to offset 0x000, zero-
  inits the rest, sets `+0x288=1` (Normal default), `+0x28F-0x28C = 0xFFFFFFFF`
  (Length default -1), and appends to the entry pointer-array at `this+0x18`.
- After the loop, `StreamPlayer__Create()` is called (returns pointer stored at
  `this+0x2C`), and `Set_Volume`-style helpers (FUN_00408080, FUN_00407b40,
  FUN_00407b50) initialize the stream-player state.
- Per-theme INI section read is performed later by FUN_00720480 — it reads
  `Sound=`, `Normal=`, `Repeat=`, `Scenario=`, and (if Normal) `Side=` into the
  struct.

### Selection + start (every shell entry)

Inside the inner `Main_Game` (RVA 0x0052d9a0):

```text
0x0052d9af   PUSH    "INTRO"            ; s_INTRO_008263a8
0x0052d9b4   CALL    FUN_00721210       ; Theme::From_Name(name)  -> EAX = index
0x0052d9b9   MOV     ECX, 0xa83d10      ; this = g_Theme
0x0052d9be   PUSH    EAX
0x0052d9bf   CALL    FUN_00720bb0       ; Theme::Play(this, index)
```

The same pattern repeats at 0x0052dcb6, 0x0052dee2, 0x0052e29e — all of which
re-enter the shell from a non-shell state (post-scenario, post-WOL, post-options).

### Loop / re-queue

`Theme::Play` (FUN_00720bb0) ends with:

- If global Repeat (`g_Theme+0x10 == 1`) **or** per-theme Repeat
  (`theme+0x289 == 1`), log `"Repeating"`, fall through, and write
  `g_Theme[2] = param_2` (i.e. `g_Theme+0x8 = index`).
- Otherwise log `"Playing"` and jump to `LAB_00720e08` (no pending set).

`Theme::AI` (FUN_007209d0) is called from the audio tick FUN_00406f70 (which is
itself called from Network_ServiceLoop @ 0x0048d080 and FUN_006071e0). Each tick:

- If `g_Theme+0x2C` (StreamPlayer) is non-null and `StreamPlayer__IsPlaying()`
  returns non-zero, return (still playing — don't interrupt).
- Otherwise, read pending index from `g_Theme+0x8`. If valid (not -1, not -3,
  and `g_MapEditorMode == 0`):
  - If pending == -2, pick a new song via FUN_00720a80 (random-from-playlist).
  - Call `Theme::Play(g_Theme+0x8)`.
  - Set `g_Theme+0x8 = 0xFFFFFFFE` (sentinel "AI will pick next").

Because `[INTRO].Repeat=yes` makes `Theme::Play` store the same index back into
`+0x8`, the next AI tick re-plays `INTRO`. This is the loop mechanism. There is
no explicit "on stream completion" callback — completion is polled.

### Stop (on leaving the shell)

`Theme::Stop` (FUN_00720ea0) takes a `fade` bool. It is not called on every
non-shell transition — the shell music typically keeps playing under the loading
screen until the next `Theme::Play("INTRO")` (or scenario theme) replaces it.
The explicit teardown call is FUN_00720770 (ThemeClass::Clear), only called
during full-engine shutdown / re-init.

---

## INI source of truth

In-repo files:

- `ini/thememd.ini:54-58` — `[INTRO] Name=THEME:Intro Sound=Drok Normal=no Repeat=yes`
- `ini/thememd.ini:8-9` — `[Themes] 1=INTRO`
- `ini/theme.ini` — RA2 base (would set `[INTRO]` with the RA2 shell track;
  superseded by `thememd.ini` at INI-merge time).

The loader reads from the merged CCINI, so the YR `*md` values are what reach
the per-theme struct. No code path special-cases YR vs RA2 — the override is
purely data.

`Sound=Drok` → at play time, the engine builds a filename by concatenating
`Drok` + `.WAV` (literal at 0x00844768). The actual file opened depends on the
mix archive lookup (out of scope); in retail this resolves to the audio file
shipped in `themesmd.mix`.

---

## YR vs RA2 vs TS

- **TS legacy:** no. ThemeClass is alive and used every game. The shell music
  call sites are reached on every shell entry. No `SpecialFlags` gate.
- **RA2 vs YR:** the INTRO key exists in both. YR's `thememd.ini` redefines the
  `Sound=` value to point at the YR track (`Drok`). The control-flow path is
  identical.
- **Active in YR (standard skirmish): Yes.** Every player who launches the game
  hears the INTRO theme on the main menu, and again every time they return to
  the main menu from a game.

---

## Open questions / out-of-scope

These are listed so the next investigation can pick them up; they are NOT
required for the shell-music path.

1. **Exact file extension actually opened.** The `.WAV` literal at 0x00844768
   is what's appended, but the path goes through `FUN_007c9ff0` (sprintf-like)
   then `StreamPlayer__PlayFile(1)` (FUN_00407... family). The mix loader may
   accept multiple extensions or strip/replace `.WAV`. Verifying the on-disk
   file requires tracing into StreamPlayer.
2. **FUN_00720a80 (random-pick).** Called by `Theme::AI` when pending = -2 (in-
   game theme cycling). Out of scope for shell music.
3. **Side/Scenario filtering.** Read into per-theme struct but only consulted
   by the playlist UI / in-game theme picker, not by the shell entry path.
4. **What writes `g_Theme+0x10`** (global Repeat). For the shell path, the per-
   theme Repeat at offset 0x289 is sufficient (and is `1` for `INTRO`).
5. **0x00a83d18 (`g_Theme_LastQueueIndex`)** is read by two network functions
   (0x0077b2e2, 0x0077e188). Unrelated to shell music but worth a note when
   investigating MP music sync.
6. **DAT_00a8ec74 and DAT_00a8ed64.** These flags gate every Theme function.
   `0xa8ec74` reads like "audio system initialized"; `0xa8ed64` like a "Theme
   suppressed" flag (set during cutscenes? loading?). Not investigated further.

---

## Verified-fact summary

1. Shell theme key = `INTRO` — string at 0x008263a8, pushed at four xref sites
   (0x0052d9af, 0x0052dcb6, 0x0052dee2, 0x0052e29e), all in inner `Main_Game`.
2. `g_Theme` instance lives at `0x00a83d10` — ECX value at all four call sites.
3. Trigger function: inner `Main_Game` (0x0052d9a0) — calls `Theme::From_Name`
   (FUN_00721210, 0x00721210) then `Theme::Play` (FUN_00720bb0, 0x00720bb0).
4. Loop mechanism: `Theme::Play` writes index back to `g_Theme+0x8` when
   per-theme Repeat byte at struct offset 0x289 (or global `g_Theme+0x10`) is 1;
   per-tick `Theme::AI` (FUN_007209d0, 0x007209d0) re-plays from `+0x8` when
   the stream is no longer playing. Pump is FUN_00406f70 at 0x00406f70.
5. YR override is INI-only: `thememd.ini:[INTRO] Sound=Drok Repeat=yes` — no
   YR-specific code branch.
