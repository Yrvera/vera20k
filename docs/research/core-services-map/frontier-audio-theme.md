# frontier-audio-theme — Music / theme player (ThemeClass)

**Slug:** `frontier-audio-theme`
**Status:** promoted from catalog stub (was UN-STUDIED, representative address UNVERIFIED).
**Layer:** audio (out-of-sim).
**Tick / render / audio plug point:** OUT-OF-SIM audio loop — `ThemeClass::AI`
polled from the audio pump `FUN_00406f70`, NOT `LogicClass::PerTickUpdate`. n/a to the
28-rung spine.

> **Verification provenance (read this first).** No live Ghidra instance was reachable
> this session (`list_instances` → 0 found; `connect_instance gamemd` → UDS 0, TCP
> 127.0.0.1:8089 refused). Every address below is **doc-verified** — taken from
> `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` (status `ghidra/verified`, every
> address read directly from Ghidra in *that* session) cross-checked against
> `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md`, `AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md`,
> and `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md`. They are **NOT re-verified
> live this session.** Per CLAUDE.md discipline they should be re-read from the binary
> (`decompile_function 0x007209d0` etc.) before any implementation. The stub's claim that
> "no `Theme`/`Music`-labeled functions exist" is **STALE** — the ThemeClass family is
> fully mapped in the report above.

---

## PURPOSE

In-game / shell music: pick a music theme from the `[Themes]` playlist (`thememd.ini`),
start it streaming, and re-queue/advance on completion (per-theme `Repeat=yes`, global
playlist shuffle, or scenario/side-filtered selection). One global singleton drives one
streaming channel. This is the "Score" / jukebox layer, distinct from VOC (SFX) and EVA
(announcer voice), though all three share the same DirectSound StreamPlayer back-end.

The main-menu `INTRO` track (`Sound=Drok`, `Repeat=yes`) is the most visible instance;
the same machinery cycles in-game battle music via the random/playlist path.

**Active in YR:** Yes — runs on every launch (shell `INTRO`) and every match. No
`SpecialFlags` gate, no TS-legacy gating. ThemeClass is alive and used every game.

---

## WHAT IT OWNS (globals / structs)

| Address | Name | Notes |
|---|---|---|
| `0x00a83d10` | `g_Theme` (ThemeClass singleton, BSS) | the `this` for all Theme calls; ECX at the four `INTRO` xref sites (doc-verified) |
| `g_Theme+0x00` | `Score` (current index, -1 none) | written by `Theme::Play` |
| `g_Theme+0x04` | last "queue song" index | written by `Theme::Play` |
| `g_Theme+0x08` | pending / next-song index | read by `Theme::AI`; sentinels -1/-2(AI-pick)/-3 |
| `g_Theme+0x0C` | theme array count | used by AI |
| `g_Theme+0x10` | global Repeat / playlist flag (bool) | one of two Repeat sources |
| `g_Theme+0x11` | Fading flag (bool) | |
| `g_Theme+0x18` | theme-entry pointer array | each entry is a 0x290-byte per-theme struct |
| `g_Theme+0x1C` | entry array capacity | |
| `g_Theme+0x24` | entry array count | |
| `g_Theme+0x2C` | **StreamPlayer\*** | the streaming channel this class owns; polled by AI |
| `0x00a83d14` | last queued/playing song index | call-site global |
| `0x00a83d18` | `g_Theme_LastQueueIndex` | read by network code (0x0077b2e2, 0x0077e188) — MP music sync, out of shell scope |
| `0x00a8ec74` | audio-system-initialized flag (inferred) | gates Theme functions |
| `0x00a8ed64` | "Theme suppressed" flag (inferred) | gates Theme functions (cutscene/loading?) |

**Per-theme entry struct (0x290 bytes)** — key offsets: `+0x000` theme key (char[256],
e.g. `INTRO`), `+0x100` Sound filename (char[256], from `Sound=`), `+0x200` Side filter
(char[64]), `+0x280` Scenario (int), `+0x288` Normal (bool, default true), `+0x289`
Repeat (bool, default false), `+0x28C` length-like int (default -1).

---

## KEY FUNCTIONS (doc-verified; re-verify live before implementing)

| Address | Symbol | Role |
|---|---|---|
| **`0x007209d0`** | **`ThemeClass::AI`** (FUN_007209d0) | **REPRESENTATIVE** — per-poll loop driver: if StreamPlayer still playing, return; else read pending index from `+0x8`, resolve (-2 → random pick), call Play, set `+0x8 = -2` |
| `0x00720bb0` | `ThemeClass::Play` (FUN_00720bb0) | start/queue a track by index; writes index back to `+0x8` if per-theme `Repeat` (`+0x289`) or global Repeat (`+0x10`) is set → the loop |
| `0x00721210` | `ThemeClass::From_Name` (FUN_00721210) | theme key string → index (e.g. `"INTRO"` → idx) |
| `0x00720ea0` | `ThemeClass::Stop` (FUN_00720ea0) | stop with `fade` bool; rarely called on transitions |
| `0x00720590` | `ThemeClass loader` (FUN_00720590) | one-time: read `[Themes]`, alloc 0x290 entries, create StreamPlayer (stored at `+0x2C`) |
| `0x00720480` | `ThemeClass per-section INI read` (FUN_00720480) | read `Sound=`/`Normal=`/`Repeat=`/`Scenario=`/`Side=` into entry |
| `0x00720770` | `ThemeClass::Clear` (FUN_00720770) | free entry array; only at full engine shutdown/re-init |
| `0x00720a80` | `ThemeClass next-track random pick` (FUN_00720a80) | playlist shuffle; draws `g_MainRng` at `0x00720ab5` (cross-service RNG edge) |
| `0x00406f70` | audio pump tick | calls `ThemeClass::AI` (the plug point) |

**StreamPlayer back-end (shared, owned by frontier-audio-voc):** `StreamPlayer__Create
@ 0x00407860`, `StreamPlayer__PlayFile @ 0x00407b60`, `StreamPlayer__Stop @ 0x00407f40`,
`StreamPlayer__IsPlaying @ 0x00408070`. Theme owns one instance (`g_Theme+0x2C`); EVA and
Speech own their own instances (`0x00b1d4cc`, `0x00b1d4d8`).

---

## PLUG POINT (where it runs)

**OUT-OF-SIM audio loop — not a PerTickUpdate rung.** `ThemeClass::AI (0x007209d0)` is
polled from the audio pump `FUN_00406f70`, which is itself invoked from
`Network_ServiceLoop @ 0x0048d080` and `FUN_006071e0` (the per-frame service loop), NOT
from `LogicClass::PerTickUpdate @ 0x0055AFB0`. So it ties to **no spine rung** — like
`frontier-audio-voc` and `frontier-audio-eva`, the sim emits cues (scenario start, return
to shell) and the audio loop drains them. Cue *timing* must match gamemd, but the engine
is render/audio-side and tick-decoupled. Loop semantics are **poll-on-completion**, not a
stream callback: AI checks `StreamPlayer__IsPlaying` each pass and re-queues when idle.

**Shell entry point (cue source):** inner `Main_Game @ 0x0052d9a0` pushes `"INTRO"`
(`0x008263a8`) → `From_Name` → `Play`, on every shell entry (four xrefs: 0x0052d9af,
0x0052dcb6, 0x0052dee2, 0x0052e29e).

---

## OUTGOING EDGES (this service depends on)

- **→ `frontier-audio-voc`** — via the shared **StreamPlayer / DirectSound streaming
  back-end** (`StreamPlayer__Create 0x00407860`, `StreamPlayer__PlayFile 0x00407b60`,
  `StreamPlayer__IsPlaying 0x00408070`). Theme owns its own StreamPlayer at `g_Theme+0x2C`
  but the *class and the DirectSound device* are the audio engine VOC owns. Evidence:
  `EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` StreamPlayer table; `MAIN_MENU_MUSIC...` loader
  calls `StreamPlayer__Create`. **(Highest dependency — the stub's "most-depends-on".)**
- **→ `rules-class`** — via `ThemeClass loader 0x00720590` reading the `[Themes]` section
  (string `Themes @ 0x00844760`) and per-theme `0x00720480` reading `Sound=`/`Repeat=`/
  `Scenario=`/`Side=` from the merged CCINIClass (`thememd.ini`). The theme list is data,
  not code. Evidence: `MAIN_MENU_MUSIC...` loader section; `ini/thememd.ini:[Themes]`,
  `[INTRO]`.
- **→ `ini-parsing`** — via the CCINIClass reads in the loader (`0x00720590` / `0x00720480`)
  that resolve `[Themes]`/per-theme sections from the merged INI. (CCINIClass is the
  parser layer rules-class sits on.)
- **→ `random-scenario`** — via `ThemeClass next-track random pick 0x00720a80` drawing
  `g_MainRng (0x00886B88)` at `0x00720ab5` for in-game playlist shuffle. Evidence:
  `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md` ("FUN_00720a80 0x00720ab5 — music
  theme next-track" listed under "Draw from g_MainRng"). Note: only the in-game random/
  shuffle path consumes RNG; the shell `INTRO` `Repeat=yes` loop does **not** draw.

## INCOMING EDGES (who depends on this service)

- **← `shell-dialog`** — inner `Main_Game @ 0x0052d9a0` (shell driver) starts/loops the
  `INTRO` theme on every shell entry via `From_Name`+`Play`. Evidence: four `INTRO` xrefs;
  `EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` (the only shell-entry audio is the
  INTRO theme).
- **← `frontier-net-eventqueue` / net loop** — the audio pump that ticks `ThemeClass::AI`
  is reached from `Network_ServiceLoop @ 0x0048d080`; MP music sync reads
  `g_Theme_LastQueueIndex (0x00a83d18)` in network code (0x0077b2e2, 0x0077e188). This is
  the call-path host for the pump, not a state dependency on net.
- **← `factory-house` / sim cue sources (loose)** — defeat/victory paths stop/swap theme
  music (e.g. `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md` checks current theme id `0x73`
  and stops it). Scenario start swaps to the battle theme. These are cue emitters, not
  hard couplings.

---

## YR vs RA2 vs TS

- **TS legacy:** No. ThemeClass is live every game; no `SpecialFlags` gate. The shell call
  sites fire on every shell entry.
- **RA2 vs YR:** Same control flow; the `[INTRO]` `Sound=` override (`Drok`) is purely
  INI (`thememd.ini` patches `theme.ini` at merge). No YR-specific code branch.
- **Active in YR (standard skirmish):** Yes.

---

## OPEN / TO RE-VERIFY (next session, live Ghidra)

1. **Re-verify all addresses live** — `decompile_function 0x007209d0` (AI), `0x00720bb0`
   (Play), `0x00721210` (From_Name), `0x00720590` (loader), `0x00720a80` (random pick) —
   confirm they are the ThemeClass family and the `g_Theme = 0x00a83d10` receiver.
2. **In-game (non-shell) theme selection path** — how the battle theme is chosen at
   scenario start and how Side/Scenario filtering on `+0x200`/`+0x280` is consulted by the
   in-game picker (the report flagged this as out-of-scope for shell music).
3. **Exact on-disk file resolution** — `Sound=` + `.WAV` (`0x00844768`) through the
   sprintf `FUN_007c9ff0` and `StreamPlayer__PlayFile`; whether the mix loader accepts
   `.aud`. (Ties into `frontier-mix-vfs`.)
4. **`g_Theme+0x10` writer** (global Repeat) and the `0x00a8ec74`/`0x00a8ed64` gate flags.
5. **MP music sync** via `g_Theme_LastQueueIndex (0x00a83d18)` — relevant to lockstep
   only if music selection is event-driven across the wire.

---

## Sources

- `docs/research/MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` (primary; ghidra/verified)
- `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` (StreamPlayer back-end table)
- `docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md` (four-subsystem audio map; Music = separate system)
- `docs/research/TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md` (g_MainRng music-theme next-track draw)
- `docs/research/EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` (shell entry = INTRO theme, no EVA)
- `docs/research/MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md` (defeat path stops current theme)
- `ini/thememd.ini` (`[Themes]`, `[INTRO] Sound=Drok Repeat=yes`)
