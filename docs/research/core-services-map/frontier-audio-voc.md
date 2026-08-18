# Core Service Profile — frontier-audio-voc

**Slug:** `frontier-audio-voc`
**Service:** SFX engine (VocClass registry + DirectSound channel pool / mixer)
**Status:** promoted from frontier catalog stub (`_frontier.md` §C1) to full profile.
**Primary docs:**
`docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md` (DSound pool, eviction, struct layouts),
`docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md` (PlayAtPos/CalcVolumeAndPan, struct field map),
`docs/research/substrate/tables/REMAP_PALETTE_SOUND_SUBSTRATE_STUDY.md` (VERIFIED-THIS-SESSION name→index,
flag tables, Rust-port DRIFT ledger),
`docs/research/GLOBAL_SOUNDS_GHIDRA_REPORT.md` (RulesClass sound-index fields, ReadAudioVisual).
**This profile:** edge/graph extract for the core-services map. Long content lives in those primary docs.

---

## VERIFICATION STATUS — read first

**Ghidra was UNREACHABLE during this profiling session.** `list_instances` returned zero
running instances and `connect_instance` failed (UDS: 0 found; TCP 127.0.0.1:8089 actively
refused) on every retry across the session. I therefore could **not** re-verify the
representative address live this session.

Per the project's verification discipline (never invent a verification; if unknown, say so),
**every address below is `VERIFIED-FROM-PRIOR-DOC`** — taken from prior `docs/research/`
reports that recorded their own inline Ghidra-call citations when those reports were written
(dates 2026-03-22 / 2026-03-23 / 2026-06-04). They are **NOT re-verified this session.**
Where a prior report cited the exact Ghidra call (e.g. `decompile_function 0x00750ac0`,
`get_function_by_address 0x00750D40`), that citation is reproduced inline so the chain is
auditable. **A live `get_function_by_address` / `decompile_function` re-confirmation pass is
the one open verification item** (see §Open / unverified edges).

**Stub representative-address note (resolved against prior docs, not re-verified live):** the
`_frontier.md` C1 stub named `VocClass__PlayAtCoord @ 0x00750E20` as the representative SFX
function. Cross-doc reading shows there are **two distinct positional-play entries**, both
attested by prior reports:
- `VocClass::PlayAtPos @ 0x00750920` — the ~75-caller workhorse dispatch (3 args: vocIndex,
  coords, loopHandle). This is the **more representative** entry (highest caller count). Prior
  decompile-verified (`decompile_function 0x00750920`, REMAP_PALETTE_SOUND_SUBSTRATE_STUDY +
  SPATIAL_AUDIO §2).
- `VocClass::PlayAtCoord @ 0x00750E20` — a higher-level entry ("copies coords into a sound
  event struct," SPATIAL_AUDIO §7 function table). Matches the stub address; **lower** caller
  attestation.

This profile adopts `0x00750920` (PlayAtPos) as the primary representative and keeps
`0x00750E20` (PlayAtCoord) as a sibling. Both addresses are PRIOR-DOC, not re-verified live.
There is also `VocClass::PlayAt @ 0x007509E0` (a third sibling, attested by
ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING).

---

## Purpose

The **sound-effect engine**: it owns the VocClass sample registry (name→index→sample-list
table parsed from `soundmd.ini` `[SoundList]`/`[AudioVisual]`), the positional volume/pan
math that turns a world coordinate into a screen-relative volume + stereo pan, the fixed pool
of **16 DirectSound secondary buffers** ("channels") with **priority-based eviction**, the
**200-slot SoundEvent pool** (the queued/playing event state machine), and the per-frame
**mixer update tick** that resolves priorities, evicts, starts playback, and advances
playlists. It runs on a **dedicated above-normal-priority audio thread**, not the sim tick.

What it does NOT own: the **EVA announcer queue** (VoxClass / speech stream — that is
`frontier-audio-eva`, a separate 1-buffer streaming player), unit **voice responses** (a
separate 1-buffer voice stream), **music/theme** playback (`frontier-audio-theme`, separate
system), and the *decision* of which cue to emit (sim systems decide; this service only
plays). It is the **shared DirectSound back-end** that EVA/voice depend on for device init,
but the EVA/voice **queues** are their own services.

---

## Owns

Globals/structs/pools (addresses VERIFIED-FROM-PRIOR-DOC; offsets from AUDIO_CHANNEL_MANAGEMENT
§2/§12 and SPATIAL_AUDIO §8):

- **The VocClass / AudioEventClass registry** — `g_VocArray @ 0x00b1d37c` (ptr to array of
  VocClass*), `g_VocCount @ 0x00b1d388`. Each VocClass is a thin wrapper over AudioEventClass;
  array order **is** the index domain (the i32 indices stored in RulesClass sound fields and
  the CreditTicks/LightningSounds DVCs). Name stored at `entry+0x6c`.
- **The AudioEventClass struct fields** (SPATIAL_AUDIO §1, field map): `Control@0x10`,
  `Type@0x14`, `Volume@0x18`, `Priority@0x40`, `Limit@0x48` (default 5), `Loop@0x4C`,
  `Range@0x50` (default 10 cells), `MinVolume@0x54`, `Delay@0x58`[2], `FShift@0x60`[2],
  `VShift@0x68`, `Attack@0x138`, `Decay@0x13C`, sample index array `@0xB4` (32 entries),
  total sample count `@0x134`.
- **The 16 DirectSound secondary buffers** ("channels"), each `0x1C0` bytes
  (`DSoundBuffer__Create @ 0x00402040`), created at init by `DSoundChannel__CreateAll
  @ 0x00403530` (arg EDX=0x10). DSoundDevice global `@ 0x0087e728`. Key buffer fields:
  priority `@0xA0`, status `@0xA4`, parent SampleTracker `@0xAC`, owner SoundEvent `@0xC0`,
  timestamp (tie-break) `@0xDC`. **16 = the hard ceiling on simultaneously audible sounds.**
- **The 200-slot SoundEvent pool** (`SoundEventPool__Init @ 0x00403ed0`, first arg 200), each
  event `0x280` bytes; doubly-linked active list rooted at `@0x0087e180`, active count
  `@0x0087e28c`, high-water `@0x0087e290`. SoundEvent fields: flags `@0x18`, state machine
  `@0x1C` (0=delay,1=ready,2=waiting,3=playing,4=done), VocClass ptr `@0x24`, sample handles
  `@0x28`, DSound buffer ptr `@0xB0`, volume interpolator `@0xB8`, playlist `@0x160`.
- **The three INI flag-parse tables** (embedded `{name,bit}` tables, VERIFIED-THIS-SESSION in
  REMAP_PALETTE_SOUND_SUBSTRATE_STUDY §2.3): Control table `@0x008160c0` (ALL=0x04, LOOP=0x01,
  RANDOM=0x02, PREDELAY=0x08, INTERRUPT=0x10, ATTACK=0x20, DECAY=0x40, AMBIENT=0x80), Type
  table `@0x00816048` (VIOLENT/MOVEMENT/QUIET/LOUD/GLOBAL=0x10/SCREEN=0x20/LOCAL=0x40/PLAYER/
  SHROUD=0x800/UNSHROUD=0x400/AMBIENT=0x1000/GUN_SHY=0x200/NOISE_SHY=0x100, exclusion groups
  0x60 and 0xc00 last-wins), Priority table `@0x00816018` (LOWEST=0..CRITICAL=4, unknown→2).
  (NOTE: AUDIO_CHANNEL §6 originally swapped PREDELAY/INTERRUPT bits; the substrate study's
  byte-read at `0x008160c0` corrected them to PREDELAY=0x08 / INTERRUPT=0x10 — bytes are
  authoritative.)
- **Default-constant globals** (SPATIAL_AUDIO §1, REMAP study §2.2): `g_DefaultVolume
  @0x008464b4` (80.0), `g_DefaultMinVolume @0x008464b8` (20.0), `g_DefaultRange @0x008464c0`
  (10), `g_DefaultLimit @0x008464c4` (5), `g_SoundEnabled @0x008464ac` (master enable gate),
  inaudibility threshold `@0x007e8ae8` (0.05), half-viewport factor `@0x007e5168`, pan
  constant 8192.0 `@0x007f68e8`, silent sentinel `FLOAT_007e1748`.
- **The dedicated audio thread + critical section** (AUDIO_CHANNEL §13): `SoundThread__Init
  @ 0x00407550`, entry `@0x00407680`, `THREAD_PRIORITY_ABOVE_NORMAL`, `CRITICAL_SECTION
  @0x0087e7f8`, shutdown flag `@0x0087e770`.

It does **not** own: RulesClass sound-index fields themselves (those live in the RulesClass
singleton `@0x008871e0`, owned by `rules-class` / `factory-house`; this service only resolves
names→indices for them), the EVA/voice stream players (`@0x00b1d4cc`/`@0x00b1d4d8`), or cell
shroud bits (it reads `CellClass+0x12C & 0x18` but `cell-map` owns them).

---

## Key functions & globals (addresses — VERIFIED-FROM-PRIOR-DOC, not re-verified live)

| Symbol | Address | Role | Prior-doc citation |
|---|---|---|---|
| `VocClass::PlayAtPos` | 0x00750920 | **Representative** — positional SFX dispatch (vocIndex, coords, loopHandle); ~75 callers; silent no-op on disabled/OOB | `decompile_function 0x00750920` (REMAP study §2.2, SPATIAL §2) |
| `VocClass::PlayAtCoord` | 0x00750E20 | Higher-level play — copies coords into a sound-event struct (stub's named rep) | SPATIAL_AUDIO §7 fn table |
| `VocClass::PlayAt` | 0x007509E0 | Sibling positional play (anim path) | ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING |
| `VocClass::CalcVolumeAndPan` | 0x00750AC0 | World coord → volume(0..1) + pan(0..16384); Range×60, isometric Y-doubling, SHROUD/LOCAL/GLOBAL gates, 0.05 threshold | `decompile_function 0x00750ac0` (REMAP §5.8, SPATIAL §3) |
| `VocClass::ReadINI` | 0x00750440 | Parse one VocClass entry from soundmd.ini | `decompile`/contract (SPATIAL §1, REMAP §2.2) |
| `VocClass::ReadSoundListINI` | 0x007510D0 | Parse `[AudioVisual]` defaults then iterate all sound entries | SPATIAL §1 |
| `VocClass::FindByName` | 0x007514D0 | Name→index; case-sensitive strcmp, first-match (lowest idx), `"Invalid Voc"` for null-named, else -1 | `decompile_function 0x007514d0` (REMAP §2.2) |
| `VocClass::FindPtrByName` | 0x00751520 | Name→VocClass* (`<none>`→0) | REMAP §2.2 (DOC-HIGH) |
| `VocClass::FindIndexByPtr` | 0x007515C0 | Ptr→index or -1 | REMAP §2.2 |
| `VocClass::GetName` | 0x00405170 | Returns `entry+0x6c` (or `"<no events>"` if subsystem not ready) | REMAP §2.2 |
| `VocClass::PlayGlobal` | 0x00406670 | Non-positional UI sound | AUDIO_CHANNEL §11 |
| `SoundSystem__UpdateTick` | 0x004041D0 | **Mixer tick** — priority resolution, eviction loop, playback start; runs on the audio thread | AUDIO_CHANNEL §4.2/§11 |
| `SoundEvent__PreparePlayout` | 0x00404700 | Allocate a DSound buffer for an event | AUDIO_CHANNEL §4.2/§11 |
| `DSoundChannel__FindLowestPriority` | 0x00404E20 | Eviction candidate among busy channels | AUDIO_CHANNEL §4.2/§11 |
| `DSoundChannel__FindAvailable` | 0x004035F0 | Idle-or-lowest-priority channel select (2-pass, age tie-break ≥0x665 ticks) | AUDIO_CHANNEL §4.3 |
| `DSoundChannel__CreateAll` | 0x00403530 | Create N (=16) DirectSound buffers | AUDIO_CHANNEL §1 |
| `SoundEvent__StartPlayback` | 0x004054A0 | Lock buffer, set vol/pan, DSoundBuffer::Play | AUDIO_CHANNEL §11 |
| `SoundEvent__Stop` | 0x004052F0 | Stop event, release its buffer | AUDIO_CHANNEL §11 |
| `SoundEvent__LoadSamples` | 0x004048B0 | Sample select per Control flags (RANDOM/ALL/ATTACK/DECAY) | AUDIO_CHANNEL §6.1 |
| `AudioSystem__Init` | 0x00406B10 | Master init: DSound device, 16 buffers, thread, pools | AUDIO_CHANNEL §1 |
| `AudioEventClass::ParseControlFlag` | 0x00406820 | Parse one Control token (OR, unknown→noop) | SPATIAL §7, REMAP §2.2 |
| `AudioEventClass::ParseTypeFlag` | 0x00406870 | Parse one Type token (exclusion 0x60/0xc00 then OR) | SPATIAL §7, REMAP §2.2 |
| `TacticalClass::CoordsToClient2` | 0x006D2140 | World→screen pixel (used by CalcVolumeAndPan) | SPATIAL §7 |
| `RulesClass::ReadAudioVisual` | 0x006691E0 | Resolves 74 sound names + 3 DVC lists into RulesClass i32 fields via FindByName | GLOBAL_SOUNDS (1168-line decompile) |
| `CCINIClass::ReadSoundList` | 0x00525430 | `[SoundList]`-style DVC builder (strtok, skip-NULL, indices, INI order) | `decompile_function 0x00525430` (REMAP §2.2) |

**Sentinels (REMAP §2.3, VERIFIED-THIS-SESSION there):** `"Invalid Voc" @0x846574`,
`"<no events>" @0x816204`, Voc delim `@0x00846570`, ReadSoundList delim `@0x00817f70`,
`<none>` sentinel `@0x00817474`.

---

## Tick / render / audio plug point

**OUT-OF-SIM — not a PerTickUpdate rung owner.** Two distinct timelines:

1. **Cue emission (inside the per-tick spine).** Sim systems call `PlayAtPos`/`PlayAtCoord`
   during their normal rungs — combat/weapon fire (Fire_At path), AnimClass AI (rung N,
   `AnimClass::UpdateLoopingSound @ 0x00750D40` maintains positional looping sound each tick),
   building/production, mission/radio events. So **cue timing is gated by the sim tick** (and
   must match gamemd's tick ordering), but the call only *enqueues* a SoundEvent.
2. **Mixer resolution (off the sim tick).** `SoundSystem__UpdateTick @ 0x004041d0` (priority
   resolve → eviction → playback start → playlist advance) and the streaming fill run on the
   **dedicated above-normal audio thread** (`SoundThread__Init @ 0x00407550`, AUDIO_CHANNEL
   §13), protected by `CRITICAL_SECTION @0x0087e7f8`. This is decoupled from
   `LogicClass::PerTickUpdate`.

**Spine touch-points** (per `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` + `_spine-anchor.md`):
- The SFX engine is **not** any of the 28 PerTickUpdate rungs (A–AB). It has **no** rung of
  its own — it is a callee reached transitively from rungs that fire cues (notably rung N, the
  object/anim AI pass, via UpdateLoopingSound, and the combat path).
- In `Main_Tick`'s **postlude** (after PerTickUpdate returns, before the frame-counter bump),
  the engine builds an **audio/ambient sound-volume value** (`FUN_0054f5c0` + `Math__ftol`) and
  fires **up to 4 ambient-loop updates** (`FUN_004a9840`) gated by `DAT_00abce14` bits
  (0x100/0x1000/0x1/0x10) — the ambient-loop maintenance that ties this service into the frame
  loop. (`_spine-anchor.md` Postlude, lines 326–328.)
- **Lockstep note:** SFX volume/pan is presentation-layer float and is **not** part of the
  state hash. `RANDOM` sample selection *does* consume an RNG draw in gamemd (REMAP §4.2i) —
  but a correct port must take that draw from the **non-synchronized UI/cosmetic RNG**
  (`g_MainRng`), NOT `Scen->Random`, or it desyncs. This is the one determinism-adjacent edge.

---

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence (all PRIOR-DOC this session).

1. **lookup-tables** — the coord→pan/volume distance math + the embedded flag/priority tables.
   - via: Control/Type/Priority `{name,bit}` tables `@0x008160c0`/`@0x00816048`/`@0x00816018`;
     `Range×60` scale; pan const `8192.0 @0x007f68e8`; half-viewport factor `@0x007e5168`;
     inaudibility threshold `@0x007e8ae8`; silent sentinel `FLOAT_007e1748`.
   - evidence: REMAP study §2.3 (`read_memory 0x008160c0`/`0x00816048`/`0x00816018`,
     VERIFIED-THIS-SESSION there); SPATIAL §3/§4/§5. These are static substrate tables/consts
     the parser and CalcVolumeAndPan index into.

2. **rules-class** — the sound list itself + global sound-index fields.
   - via: `VocClass::ReadINI @ 0x00750440` / `ReadSoundListINI @ 0x007510D0` read `[SoundList]`/
     `[AudioVisual]` from rulesmd.ini; `RulesClass::ReadAudioVisual @ 0x006691e0` resolves 74
     sound names + 3 DVC lists into RulesClass i32 fields (singleton `@0x008871e0`); per-sound
     defaults come from `[AudioVisual]` globals `@0x008464b4/b8/c0/c4`.
   - evidence: GLOBAL_SOUNDS report (1168-line decompile of `0x006691e0`); SPATIAL §1.

3. **cell-map** — listener position + shroud-audibility gate.
   - via: `CalcVolumeAndPan` reads camera-center cell `@0x00b1d310/0x0312` and, for SHROUD-typed
     (0x800) sounds, the source `CellClass+0x12C & 0x18` revealed/visible bits (silences enemy
     sounds in unexplored cells).
   - evidence: SPATIAL §3 step 3 (`decompile 0x00750ac0`); REMAP §5.8. NOTE: this is **cell
     visibility**, present in stock YR — NOT the TS FogOfWar darkening (default off).

4. **ini-parsing** — the CCINIClass accessors that feed ReadINI/ReadSoundList.
   - via: `CCINIClass::ReadString`/`ReadSoundList @ 0x00525430` (strtok by delim `@0x00817f70`,
     skip-NULL, append index, INI order); per-key reads in `VocClass::ReadINI`.
   - evidence: `decompile_function 0x00525430` (REMAP §2.2, VERIFIED-THIS-SESSION there).

5. **frontier-render-tactical** (un-studied sibling) — the world→screen projection.
   - via: `TacticalClass::CoordsToClient2 @ 0x006D2140` (called inside CalcVolumeAndPan to turn
     world leptons into screen-relative pixels); `g_TacticalPtr @0x00887324`; viewport dims
     `g_RadarViewportWidth/Height @0x00886fa8/0xac`.
   - evidence: SPATIAL §3 step 4 + §8 globals. The spatial mix is computed in the tactical
     view's client space, so it depends on the tactical projection. (Render→audio coupling; the
     pure-data parts stay sim-independent per the architecture invariant.)

6. **abstract-object / techno-foot / frontier-anim** (cue sources, weak — see used-by) — the
   coordinate passed to PlayAtPos comes from the emitting object/anim's world coord. Listed as
   used-by below (incoming), but the coordinate dependency is bidirectional in practice.

---

## Used-by (incoming edges)

Services that call into / depend on this one (all PRIOR-DOC this session):

1. **frontier-anim** (AnimClass) — positional + looping anim sounds.
   - via: `AnimClass::UpdateLoopingSound @ 0x00750D40` (called at the top of `AnimClass::AI
     @ 0x00423AC0`, rung N, before the first-AI guard) maintains a positional looping sound
     each tick via the VocClass/SoundEvent pool; `VocClass::PlayAt @ 0x007509E0` for one-shot
     anim sounds.
   - evidence: ANIMATION_SOUNDS report + ANIMCLASS_CONSTRUCTOR_MIDDLE_SOUND_TIMING
     (`get_function_by_address 0x00750D40` confirmed `AnimClass::UpdateLoopingSound`,
     RTTI_LABEL_DRIFT correction 2026-05-29).

2. **techno-foot** (TechnoClass / FootClass) — weapon-fire, locomotion, selection SFX.
   - via: Fire_At / weapon paths and locomotion call PlayAtPos with the unit's coord; ~75
     PlayAtPos callers span weapon fire, movement, UI selection.
   - evidence: REMAP §1 (~75 PlayAtPos callers enumerated as weapon/locomotion/UI/radar/credit).

3. **factory-house / rules-class** — credit-tick + global event SFX.
   - via: CreditTicks DVC (`Rules+0x6CC`, needs count ≥2: [0]=up,[1]=down), SellSound, build/UI
     sounds resolved through FindByName into RulesClass fields and played on house events.
   - evidence: GLOBAL_SOUNDS report; REMAP §2.4.

4. **frontier-audio-eva** (VoxClass EVA queue) — shares the DirectSound device + init.
   - via: `AudioSystem__Init @ 0x00406b10` creates the device this service owns; EVA's speech
     stream (`@0x00b1d4d8`, `SpeechSystem__Init @ 0x00752ad0`) and unit-voice stream
     (`@0x00b1d4cc`, `VoiceSystem__Init @ 0x00752290`) are separate 1-buffer streaming players
     that do **not** compete for the 16 SFX channels but rely on the same device/thread init.
   - evidence: AUDIO_CHANNEL §1/§9. (EVA queue logic itself is `frontier-audio-eva`; this is
     the back-end-sharing edge.)

5. **frontier-audio-theme** (music) — shares the device/thread back-end (separate mixer).
   - via: music uses its own system (AUDIO_CHANNEL §10) but is initialized alongside in
     AudioSystem__Init.
   - evidence: AUDIO_CHANNEL §10 (explicitly "Music has its own system entirely").

6. **logicclass** (indirect, structural) — cue emission is gated by the per-tick spine.
   - via: rungs that fire cues (object/anim AI rung N, combat path) reach PlayAtPos transitively;
     the postlude ambient-loop update (`FUN_004a9840` ×4) ties ambient SFX into the frame loop.
   - evidence: `_spine-anchor.md` Postlude (lines 326–328); spine spec rung N. Not a direct
     LogicClass→SFX call.

---

## Active-in-YR / TS-legacy

- **The whole SFX engine is LIVE in every stock YR skirmish.** ~75 PlayAtPos callers (weapon
  fire, locomotion, UI, radar, credits, lightning); 16 DSound channels + 200-event pool +
  priority eviction all run every match. (REMAP §3.)
- **SHROUD type gate (0x800)** is LIVE — it reads `CellClass+0x12C & 0x18` cell-visibility, NOT
  the TS FogOfWar darkening (default off in YR). Present in stock YR. (REMAP §3.)
- **NOISE_SHY (0x100) / GUN_SHY (0x200)** type flags: the tables parse them, but whether the
  suppression branch is reachable in stock YR was **NOT traced** → default **UNCHECKED/DRIFT**,
  do not assume dormant. (REMAP §3.)
- **`DeploySound` as a global `[AudioVisual]` key is DEAD** (not read by ReadAudioVisual; per-type
  only). (REMAP §3.)
- No tunnel/subterranean intersection.

---

## Open / unverified edges

- **LIVE Ghidra re-verification of the representative address (BLOCKING for "verified-this-
  session" status):** Ghidra was unreachable this session. The next pass must run
  `get_function_by_address 0x00750920` (confirm PlayAtPos), `0x00750E20` (confirm PlayAtCoord),
  `0x004041d0` (confirm SoundSystem__UpdateTick), `0x00404e20` (confirm
  DSoundChannel__FindLowestPriority) to upgrade these from PRIOR-DOC to VERIFIED-THIS-SESSION.
- **RANDOM-sample RNG stream binding:** REMAP §4.2i flags that gamemd RANDOM consumes an RNG
  draw; the exact instance (must be `g_MainRng`, the cosmetic stream, not `Scen->Random`) needs
  a live callsite read in `SoundEvent__LoadSamples @ 0x004048b0` / `__SelectNextSample
  @ 0x00404bb0` before any Rust port wires it, or it risks desync. UNVERIFIED.
- **Silent-sentinel value `FLOAT_007e1748` / threshold `@0x007e8ae8` / half-viewport factor
  `@0x007e5168`:** named globals, NOT byte-read in the source studies ("presumed 0.0 / 0.05 /
  0.5"). A `read_memory` of each is the precise remaining query before any exact volume/pan
  bit-test. (REMAP §5.8.)
- **Voc/ReadSoundList delimiter bytes (`@0x00846570`, `@0x00817f70`):** not byte-dumped; needed
  to confirm the `Sounds=` tokenizer parity (REMAP §4.2d).
- **Stub mixer-tick framing:** the stub put `SoundSystem__UpdateTick` in the same bullet as
  PlayAtCoord; this profile separates cue-emit (spine-gated) from mixer-tick (audio-thread).
  Confirmed by AUDIO_CHANNEL §13 (dedicated thread) but the thread-vs-mainloop call site of
  `0x004041d0` was not live-traced this session.
