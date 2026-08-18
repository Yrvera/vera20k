# Core Service Profile — EVA voice queue (VoxClass)

**Slug:** `frontier-audio-eva`
**Layer:** audio (announcer-voice queue; strictly DOWNSTREAM of `sim/` — consumes sim cues, never writes sim state)
**Tick/audio position:** OUT-OF-SIM. Sim emits EVA cue *names* (e.g. `"EVA_ConstructionComplete"`) during the per-tick spine; the queue is drained on the audio/frame pump, not inside `LogicClass::PerTickUpdate`. Not in the deterministic state hash. (Save/load DOES persist queued entries — see Owns.)
**Primary docs:** `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md`, `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` (both `[ghidra/verified]`, 2026-03-23), `docs/research/VOXCLASS_PLAYEVA_FFFFFFFF_SENTINEL_GHIDRA_REPORT.md` (disassembly-anchored `0x00752700-0x0075275c`), `docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md`, `docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md`.

**Provenance / verification status (READ THIS):** The Ghidra MCP server was reachable this session but **NO gamemd.exe program was loaded/connected** (`list_instances` returned empty; `connect_instance` refused; the decompile tool group could not load). I therefore **could NOT re-verify any address live this session.** Every address below is **doc-sourced**, corroborated across the four+ independent verified reports above — including the sentinel report's hand-anchored disassembly of `PlayEVA` (`0x00752700-0x0075275c`). The three stub addresses (`PlayEVA 0x00752700`, `InsertIntoQueue 0x00752590`, `PlayNextQueued 0x00752760`) all match those reports verbatim. **Re-confirm against a live Ghidra instance before treating any address as PROOFED for implementation.** Status of every address here: **LOCATED / doc-corroborated, NOT live-verified this session.**

---

## Purpose

The EVA ("Electronic Video Agent") announcer-voice queue. It owns the single dedicated voice stream channel and the **priority/dedup/sequential-playout queue** that guarantees announcer lines ("Construction complete", "Unit lost", "Our base is under attack", "Insufficient funds") play one-at-a-time, in priority order, without overlapping each other — with a hardcoded **500 ms inter-announcement gap** between lines.

It is a *registry + queue + drain* service layered on the shared streaming-audio back-end:
1. **VoxClass registry** — one 84-byte `VoxClass` entry per EVA event, loaded from `EVAMD.INI [DialogList]`. Each entry holds the event name (the CSF lookup key), per-faction `.aud` clip names (Allied/Russian/Yuri), a `Volume`, a `Priority` (LOW/NORMAL/IMPORTANT/CRITICAL), a `Type` (STANDARD/QUEUE/INTERRUPT/QUEUED_INTERRUPT), and a play-state.
2. **The multi-queue** — an InterruptQueue, a CriticalQueue, a single PendingImmediate slot, and four per-priority FIFO queues. Insertion routes by `(Type, Priority)`; the drain dequeues in a fixed precedence order.
3. **The drain/playout** — picks the next entry, resolves the faction-specific clip by `CurrentSide`, enforces the 500 ms gap, and streams it through the dedicated EVA `StreamPlayer` (a DirectSound streaming channel separate from SFX and from the taunt/speech channel).

The parity contract is **observable output**: which line plays, in what order, faction-correct clip selection, dedup (no double-queue of an already-playing/queued line), the 500 ms gap, and suspend/pause behavior — NOT a port of the linked-list internals.

---

## Owns (state / globals / structs)

**Globals — all addresses doc-sourced (`EVA_SYSTEM_*` global maps), NOT live-verified this session:**

| Address | Type | Name | Role |
|---|---|---|---|
| `0x00b1d4a4` | `VoxClass**` | VoxArray_Data | Base of the VoxClass-pointer array (the registry) |
| `0x00b1d4a8` | `int` | VoxArray_Capacity | Allocated capacity |
| `0x00b1d4b0` | `int` | VoxArray_Count | Loaded entry count (`g_VoxEntryCount`) |
| `0x00b1d4b8` | `QueueNode*` | PendingImmediate | Single pending immediate-play entry |
| `0x00b1d4bc` | `int` | SystemEnabled | 1 when EVA active (`g_StreamingReady`-adjacent) |
| `0x00b1d4c0` | `int` | SequenceCounter | Monotonic counter for FIFO ordering (`% 100` into node) |
| `0x00b1d4c4` | `VoxClass*` | CurrentlyPlaying | Entry currently streaming (or 0) |
| `0x00b1d4c8` | `int` | CurrentSide | 0=Allied, 1=Russian, 2=Yuri (faction clip selector) |
| `0x00b1d4cc` | `StreamPlayer*` | EVAStreamPlayer | Dedicated EVA DirectSound stream (`DAT_00b1d4cc`) |
| `0x00b1d4d0/d4` | `int64` | InterAnnouncementDelay | 64-bit delay value; set to **500** (ms) after each play |
| `0x00b1d4d8` | `StreamPlayer*` | SpeechStreamPlayer | SEPARATE stream for multiplayer taunts (NOT EVA) |
| `0x00b1d3d8` | `int` | SuspendCounter | Nested; new EVAs silently dropped when > 0 |
| `0x00b1d428` | `int` | PauseCounter | Nested; playback frozen when > 0 |
| `0x00b1d3b8` | `int` | CurrentPlayingType | Type of the currently-playing entry |
| `0x00b1d480` | `int` | TauntLock | Blocks taunts; always 0 in normal play (vestigial) |

**Queue heads (each a 12-byte linked-list head = 3 ptrs):**

| Address | Queue | Dequeue precedence |
|---|---|---|
| `0x00b1d3c8` | InterruptQueue (Type=QUEUED_INTERRUPT / Type 3) | 1st (highest) |
| `0x00b1d3f0` | CriticalQueue (Priority=CRITICAL, non-QUEUE types) | 2nd |
| `0x00b1d4b8` | PendingImmediate (single slot) | 3rd (discarded if CriticalQueue non-empty) |
| `0x00b1d474` | PriorityQueue[3] (Type=QUEUE, prio 3) | 4th |
| `0x00b1d468` | PriorityQueue[2] | 5th |
| `0x00b1d45c` | PriorityQueue[1] | 6th |
| `0x00b1d450` | PriorityQueue[0] (lowest) | 7th |

**`VoxClass` struct (size = 0x54 = 84 bytes)** — from `VoxClass__ReadINI 0x00752db0` / `VoxClass__ReadEVAINI 0x00753000`:

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0x00 | 40 | `Name` char[40] | EVA event name; also the CSF lookup key (no separate Text/Sound key) |
| 0x28 | 4 | `Volume` float | default 1.0 (0x3F800000); **parsed but NOT used at playout** (global VoiceVolume used instead) |
| 0x2C | 9 | `YuriSound` char[9] | side-2 clip name |
| 0x35 | 9 | `RussianSound` char[9] | side-1 clip name |
| 0x3E | 9 | `AlliedSound` char[9] | side-0 clip name |
| 0x48 | 4 | `Priority` int | LOW=0 / NORMAL=1(default) / IMPORTANT=2 / CRITICAL=3 |
| 0x4C | 4 | `Type` int | STANDARD=0(default) / QUEUE=1 / INTERRUPT=2 / QUEUED_INTERRUPT=3 |
| 0x50 | 4 | `PlayState` int | 0=PLAYING / 1=QUEUED / 2=DONE/FREE (default 2) |

**Save/load:** EVA queue state is persisted. `VoxClass__LoadFromSave 0x007533f0` reads a tagged stream (`"VoxS"` header, repeated `"VoxI"` 12-byte records of `{VoxArrayIndex, Priority, Type}`, `"VoxE"` end), stops playback, clears queues, and re-`InsertIntoQueue`s each record. So although the queue is out-of-sim, it is part of the savegame cross-cut (`frontier-saveload` edge).

---

## Key functions & globals (addresses — doc-corroborated, NOT live-verified this session)

| Symbol | Address | Role | Stub claim? |
|---|---|---|---|
| `VoxClass__PlayEVA` | `0x00752700` | **Representative fn.** Main entry: name lookup (case-insensitive linear scan), then `QueueVoice(index, priority, voiceIdx)`. `__fastcall(LPCSTR name, int priority, int voiceIdx)`; `(name,-1,-1)` is the standard idiom (-1 = "use entry's own priority/voice"). Disassembly-anchored `0x00752700-0x0075275c` in the sentinel report. | **YES — matches stub** |
| `VoxClass__QueueVoice` | `0x00752480` | Resolves default Type/Priority from the entry, dedup-checks (`FindInQueues`, skip if already playing/queued same type), then calls `InsertIntoQueue`. Guard: rejects if not streaming-ready, index out of range, or SuspendCounter>0. **(Central function the stub omitted — add it.)** | added |
| `VoxClass__InsertIntoQueue` | `0x00752590` | Allocates a 32-byte QueueNode, sets `PlayState=1`, routes by `(Type,Priority)` into the correct queue head; STANDARD/INTERRUPT non-critical can only take PendingImmediate (else discarded). | **YES — matches stub** |
| `VoxClass__PlayNextQueued` | `0x00752760` | **The drain.** Guards (pause/suspend/audio-off/stream-active/500 ms gap), dequeues by precedence, selects faction clip by `CurrentSide`, streams via EVAStreamPlayer; sets delay=500 ms after play. | **YES — matches stub** |
| `VoxClass__PumpAndCheckActive` | `0x007529e0` | Pumps playback and returns 1 if anything active (the per-pump heartbeat hook). | — |
| `VoxClass__FindInQueues` | `0x00752680` | Searches all queues for a given VoxClass (dedup). | — |
| `VoxClass__FindByName` | `0x007532d0` | Name lookup; explicitly filters `<none>` (`0x817474`). | — |
| `VoxClass__GetByIndex` | `0x00752460` | Array index → VoxClass*. | — |
| `VoxClass__RemoveFromQueues` | `0x00752a40` | Removes all queued instances of one VoxClass. | — |
| `VoxClass__ClearAllQueues` | `0x00752370` | Empties all heads + PendingImmediate. | — |
| `VoxClass__SetSide` | `0x007534e0` | Sets `CurrentSide` (0/1/2; -1→0). | — |
| `VoxClass__SetGlobalVolume` | `0x00752ab0` | Sets g_EVAVolume (0-255). | — |
| `VoxClass__SuspendEVA` / `ResumeEVA` | `0x00753570` / `0x00753580` | Nested suspend counter (drop new EVAs). | — |
| `VoxClass__PauseEVA` / `UnpauseEVA` | `0x007535b0` / `0x00753620` | Nested pause counter + StreamPlayer Pause/Resume (freeze playback). | — |
| `VoxClass__ResetAll` | `0x007535d0` | Stop + clear + zero counters (game-exit). | — |
| `VoxClass__ReadEVAINI` | `0x00753000` | Parses `EVAMD.INI [DialogList]`, constructs entries. | — |
| `VoxClass__ReadINI` | `0x00752db0` | Per-entry: Volume/Type/Priority/Allied/Russian/Yuri keys. | — |
| `VoxClass__ClearAllEntries` | `0x007531a0` | Destroys all VoxClass objects. | — |
| `VoxClass__LoadFromSave` | `0x007533f0` | Restore queue from savegame. | — |
| `VoiceSystem__Init` | `0x00752290` | Inits queues + creates EVAStreamPlayer (`0xb1d4cc`). | — |
| `SpeechSystem__Init` / `PlayTaunt` | `0x00752ad0` / `0x00752b70` | SEPARATE taunt channel (not EVA). | — |

**Note on label drift:** `AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md` labels `0x00752480` / `0x00752760` as `VoiceSystem__QueueVoice` / `VoiceSystem__PlayNextQueued` ("unit voice response"), while the `EVA_SYSTEM_*` reports label them `VoxClass__QueueVoice` / `VoxClass__PlayNextQueued`. These are the **same functions** — the voice stream is dual-purpose (announcer EVA lines AND unit voice responses share the one EVAStreamPlayer path). Treat the `VoxClass__*` naming as canonical for this service; the `VoiceSystem__*` labels are the same addresses viewed from the audio-back-end side.

---

## Tick / audio position

- **OUT-OF-SIM.** Producers call `VoxClass__PlayEVA` (or `QueueVoice` directly) from many points *inside* the per-tick spine — e.g. `HouseClass::Update` (rung U-adjacent, InsufficientFunds/LowPower), `SuperClass::AI_Ready/Charging` (rung U super sub-pass, `*Ready` cues), `StripClass::AI` (sidebar, ConstructionComplete), `TechnoClass::AI` (UnitPromoted), `BuildingClass::Sell` (StructureSold). These calls only **enqueue**; they do not play.
- **The drain (`PlayNextQueued`) runs on the audio/frame pump, not the sim tick.** It is gated on real-time milliseconds (`GetPerformanceTimestamp`, QPC/1000) for the 500 ms gap, and on the StreamPlayer being idle — wall-clock-paced, NOT frame-counter-paced. The shared streaming-audio back-end (`StreamPlayer__*`, and the SFX side's `SoundSystem__UpdateTick 0x004041D0` / sound mixing thread `SoundThread__Init 0x00407550`) is the layer that services it. **Exact per-frame caller of `0x00752760` was NOT pinned this session** (Ghidra unavailable) — see Open edges.
- **Spine rung tie-in (for the map):** the highest-frequency *producer* sites land on **rung U** (`HouseClass +0x5C` AI — InsufficientFunds, LowPower, and the per-house `SuperClass` Ready cues) and on the **render/sidebar pass** (StripClass production → ConstructionComplete/NewConstructionOptions). The queue/drain itself is **out-of-sim**, downstream of all of them. (Spine reference: `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`, 28 rungs A-AB.)
- **Not in the state hash.** Faction clip selection, the 500 ms gap, and pause/suspend are all presentation — no deterministic sim coupling. (Save/load persistence is a serialization cross-cut, not a sim-tick dependency.)

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `frontier-audio-voc` | shared `StreamPlayer__*` back-end (`Create 0x00407860`, `PlayFile 0x00407b60`, `GetEndTime 0x00408140`, `Pause/Resume 0x00407fb0/0x00408000`) + the same DirectSound device/mixer (`AudioSystem__Init 0x00406b10`, `SoundSystem__UpdateTick 0x004041D0`, `SoundThread__Init 0x00407550`) | EVA owns a *dedicated* StreamPlayer (`0xb1d4cc`) but it is the same streaming-audio primitive and DirectSound device the SFX engine owns. Drain/playout/pause/end-time all route through the VOC service's stream/mixer layer. Doc: `AUDIO_CHANNEL_MANAGEMENT` §11; `EVA_SYSTEM_DEEP_DIVE` §6. |
| `factory-house` | producers `HouseClass::Update` (`0x004f8ba0` EVA_InsufficientFunds, `0x004f8d14` EVA_LowPower), `HouseClass::BaseUnderAttack 0x004f93e0`, `BuildingClass::Sell 0x00449c30`, `StripClass::AI` (ConstructionComplete/NewConstructionOptions), `SuperClass::AI_Ready 0x006cbca0` / `AI_Charging 0x006cc080` | The bulk of EVA cue *sources* are HouseClass economy/production state and per-house superweapon readiness — i.e. the factory/house service drives most announcer lines. (Edge direction: factory-house is a producer → see also Used-by.) Doc: `EVA_SYSTEM_DEEP_DIVE` §4 call-site table. |
| `mission-radio` | combat/mission producers — `HouseClass::BaseUnderAttack`, harvester `UnitClass__Mission_Harvest 0x00737c90` (EVA_OreMinerUnderAttack), unit-lost/promoted cues from `TechnoClass::AI` | Mission/combat state transitions emit EVA cues (base-under-attack, unit lost/ready/promoted). The radio/mission layer is a major cue source alongside factory-house. Doc: `EVA_SYSTEM_DEEP_DIVE` §4. |
| `rules-class` / `ini-parsing` | `VoxClass__ReadEVAINI 0x00753000` → `CCINIClass` over `EVAMD.INI [DialogList]`; per-entry keys via `VoxClass__ReadINI 0x00752db0` (Volume/Type/Priority/Allied/Russian/Yuri) | The whole registry is built at load-time by parsing `EVAMD.INI` through the CCINIClass parser. Side-MIX selection (`InitSideMixFiles 0x00534fa0`) also feeds `SetSide`. Doc: `EVA_SYSTEM_DEEP_DIVE` §2. |
| `frontier-mix-vfs` | `.aud` clip files resolved by name (e.g. `ceva062`) from MIX (`langmd.mix`/`audiomd.mix`), `.WAV` extension appended cosmetically (real files are IMA-ADPCM `.aud`) | Clip names in VoxClass resolve to `.aud` samples loaded from the MIX VFS at playout. Side MIX files chosen by `InitSideMixFiles`. Doc: `EVA_SYSTEM_DEEP_DIVE` §5, Impl note 7. |
| `frontier-super` | `SuperClass::AI_Ready 0x006cbca0` / `AI_Charging 0x006cc080` emit `EVA_*Ready`/`*Detected`/`*Launched` cues | Superweapon readiness/charge state machine is a dedicated EVA producer (nuke/iron-curtain/chrono/etc. ready/detected lines). Doc: `EVA_SYSTEM_DEEP_DIVE` §4 Superweapon Events. |
| `random-scenario` | `*g_ScenarioClass... & g_MapEditorMode (0x00A8E7AC)` gates EVA at producer sites (editor/silent-spawn suppression) | `g_MapEditorMode` is checked at `VoxClass::PlayEVA` producer sites to suppress cues during map-editor / silent paradrop spawns. DORMANT in normal skirmish (editor mode off). Doc: `PARADROP_SUPERWEAPON` §24, §22. |

---

## Used-by (incoming edges)

| Source slug | Via symbol | Evidence |
|---|---|---|
| `factory-house` | `HouseClass::Update`, `BaseUnderAttack 0x004f93e0`, `BuildingClass::Sell 0x00449c30`, sidebar `StripClass::AI` → `VoxClass__PlayEVA 0x00752700` | Production-complete, new-options, insufficient-funds, low-power, structure-sold, base-under-attack all call into this service. The single largest caller cluster (`PlayEVA` has ~70-75 xrefs). Doc: `EVA_SYSTEM_DEEP_DIVE` §4; `EVA_WELCOME_BACK` §3 (~70+ callers). |
| `mission-radio` | `TechnoClass::AI` (UnitPromoted `0x006fa0cb/0x006fa139`, unit-ready/lost), `UnitClass__Mission_Harvest 0x00737c90` (OreMinerUnderAttack) | Mission/combat outcomes (promotion, unit lost/ready, harvester under attack) call `PlayEVA`. Doc: `EVA_SYSTEM_DEEP_DIVE` §4. |
| `frontier-super` | `SuperClass::AI_Ready/AI_Charging` → `PlayEVA` | Superweapon ready/detected/launched cues (G1 frontier-super lists this service as its readiness-cue dependency). Doc: `_frontier.md` G1; `EVA_SYSTEM_DEEP_DIVE` §4. |
| `frontier-trigger` | map trigger action 0x15 (Play Speech/EVA) → `VoxClass__QueueVoice(speechIndex, -1)` in `TriggerAction__Execute 0x006dd8b0` | The map-scripting engine can fire EVA lines as a trigger action (campaign). Doc: `SOUND_TRIGGERS_COMPLETE` §22. |
| `frontier-radar` | `RadarClass__PlaceBeacon 0x00430ba0` (BeaconPlaced/Detected), `RadarClass__PlayRadarMovie 0x006579c0` (suspends EVA during radar movie) | Radar beacon placement fires EVA; radar movies suspend EVA via `SuspendEVA`. EVA and the radar-event (visual ping) queues are correlated-but-separate. Doc: `RADAR_EVENT_CLASS` §1; `EVA_SYSTEM_DEEP_DIVE` §7. |
| `frontier-net-eventqueue` | `Process_QueuedEvents_WithSuspend 0x0053b460` brackets event processing with `SuspendEVA`/`ResumeEVA` (`0x0053b714`) | Lockstep event processing suspends EVA queueing across the batch so command-driven cues don't fire mid-execution. Doc: `EVA_SYSTEM_DEEP_DIVE` §7. |
| `frontier-saveload` | `VoxClass__LoadFromSave 0x007533f0` (and the matching Save walk) | Savegame serializes the EVA queue (VoxS/VoxI/VoxE tagged stream of `{index,priority,type}`). Doc: `EVA_SYSTEM_DEEP_DIVE` Save/Load. |
| `shell-dialog` / pause | `GamePause__Enter 0x00406f00` / `Exit 0x00406f40` (PauseEVA/UnpauseEVA), movie playback `Audio__PauseForMovie 0x005bf580` / resume `0x005bf450` | Game-pause screen and Bink/VQA movies pause+resume the EVA stream. Doc: `EVA_SYSTEM_DEEP_DIVE` §7. |

---

## Active-in-YR & TS-legacy

- **Active in stock YR:** YES — the core path (`PlayEVA → QueueVoice → InsertIntoQueue → PlayNextQueued`) is the standard, always-live announcer pipeline. The `(name, -1, -1)` call idiom is verified across many active call sites (sentinel report §lists four+; ~70-75 total `PlayEVA` xrefs).
- **Faction selection is per-SIDE, not per-country.** EVA clips are chosen by `CurrentSide` (0/1/2) — there is no per-country `VoxFile=`/`EVAFile=` key. `Side=` indirectly picks the EVA set. (`HOUSE_TYPE_CLASS` §5.4.)
- **`EVAMD.INI` only.** The "md" variant is the only file parsed (base `EVA.INI` is not loaded in YR). Strings: `"EVAMD.INI"`, `"Failed to find/load EVAMD.INI!"`.
- **Vestigial / dormant:**
  - `TauntLock (0xb1d480)` — initialized to 0, no code path sets it non-zero; vestigial.
  - `g_MapEditorMode (0xA8E7AC)` EVA gate — DORMANT in skirmish (editor mode off); only suppresses cues in the map editor / silent spawns.
  - Per-event `Volume` (0x28) — parsed but NOT applied at playout (global VoiceVolume used). Faithful only if Rust likewise ignores per-event volume.
- **Taunts are a SEPARATE service**, not part of this one: own StreamPlayer (`0xb1d4d8`), own init (`SpeechSystem__Init 0x00752ad0`), driven by network message type 0x78 — independent audio channel that cannot interrupt EVA. Listed here only to disambiguate; it is not a `frontier-audio-eva` edge.
- **No TS-only dead branch identified inside the EVA path itself** beyond the dormant flags above; the queue/priority machinery is all live in YR.

---

## Open / unverified edges

- **Per-frame drain caller of `PlayNextQueued 0x00752760` NOT pinned this session** (Ghidra unreachable). The drain is wall-clock-paced (500 ms gap via QPC) and stream-idle-gated; the exact pump function that calls it each frame (likely the shared audio-service tick / `SoundSystem__UpdateTick`-adjacent path or a `Main_Tick` audio-service step) needs a live `get_function_callers 0x00752760` to confirm. This is the one structural edge to `frontier-audio-voc` / the spine that is LOCATED-by-inference, not address-verified here.
- **ALL addresses in this profile are doc-corroborated, NOT live-verified this session.** Before implementation, run `get_function_by_address` / `decompile_function` on at least the four key entry points (`0x00752700`, `0x00752480`, `0x00752590`, `0x00752760`) and the global map (`0x00b1d4a4`-`0x00b1d4d8`) against a connected gamemd.exe instance. The corroboration is strong (four independent verified reports, one disassembly-anchored) but the project rule is binary→Ghidra→docs, and docs are the weakest tier.
- **`frontier-saveload` Save (write) side** — only `VoxClass__LoadFromSave 0x007533f0` is documented; the matching Save-walk address was not located. Needs an xref pass.
- **Carried-forward Rust DRIFT (from `CURRENT_APP_EVA_QUEUE_POSTFIX_TRACE.md`, for the implementation handoff, NOT part of the gamemd map):**
  - Rust collapses the 7-queue priority/type structure into one FIFO `VecDeque` (`src/audio/sfx.rs`); mixed cues play in FIFO order where gamemd routes by Type/Priority or discards STANDARD entries. (Trace FAIL.)
  - Rust routes STANDARD entries (UnitReady, CannotDeployHere) as retained queued EVA; gamemd treats non-`Type=QUEUE` entries as PendingImmediate/discard. (Trace FAIL.)
  - Rust has no 500 ms inter-announcement delay; can start the next line ~500 ms early. (Trace FAIL.)
  - Rust keys dedup on resolved clip `sound_id`; gamemd keys on the `VoxClass*` entry + type — equivalence across all entries UNCHECKED.
