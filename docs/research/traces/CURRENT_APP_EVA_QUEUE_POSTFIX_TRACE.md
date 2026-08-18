# Current App EVA Queue Postfix Trace

Scenario: while one voice/EVA clip is already playing, Rust receives current app-routed EVA cues including `StructureAbandoned` and another current EVA event such as `CannotDeployHere` or `UnitReady`.

Scope is intentionally narrow: app-routed `GameSoundEvent` EVA arms after the minimal queue fix. Adjacent full VoxClass parity issues are listed separately.

## Pipeline

`SimSoundEvent` -> `app_sim_tick.rs` resolves local-player faction clip id -> `GameSoundEvent` -> `app_building_anim.rs::drain_sound_events` -> `SfxPlayer::queue_eva_sound` -> `queued_voice` FIFO -> `advance_voice_queue` starts next clip when `voice_player.empty()`.

Native active YR path: event-specific caller -> `VoxClass__PlayEVA @ 0x00752700` -> `VoxClass__QueueVoice @ 0x00752480` -> `VoxClass__InsertIntoQueue @ 0x00752590` -> `VoxClass__PlayNextQueued @ 0x00752760`.

## Stages

### Stage 1 - Current app EVA event coverage

Rust routes these current app EVA arms through `queue_eva_sound`: `BuildingReady`, `UnitReady`, `CannotDeployHere`, `StructureGarrisoned`, `StructureAbandoned`, and `BridgeRepaired.eva_sound_id`; see `src/app_building_anim.rs:603..613` and `src/app_building_anim.rs:650..654`.

Native `VoxClass__PlayEVA` dispatches EVA event names into `QueueVoice`; verified function table at `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:57..66`.

Verdict: PASS for the implementation claim that all current app-routed EVA arms use the queued entry point.

### Stage 2 - Faction clip resolution

For Allied examples, Rust resolves:

- `EVA_UnitReady` -> `ceva062` at `src/app_sim_tick.rs:435..440`; INI `ini/evamd.ini:1095..1100`.
- `EVA_CannotDeployHere` -> `ceva063` at `src/app_sim_tick.rs:454..459`; INI `ini/evamd.ini:1102..1107`.
- `EVA_StructureAbandoned` -> `ceva108` at `src/app_sim_tick.rs:493..498`; INI `ini/evamd.ini:1425..1431`.

Native chooses the side field in `PlayNextQueued`: Allied offset `0x3E`, Russian `0x35`, Yuri `0x2C`; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:203..209`.

Verdict: PASS for the sampled Allied clip ids.

### Stage 3 - Type=QUEUE non-interruption for StructureAbandoned

`EVA_StructureAbandoned` is `Type=QUEUE`, `Priority=NORMAL` in `ini/evamd.ini:1425..1431`. With a voice already playing, Rust `queue_eva_sound` calls `advance_voice_queue`, sees the voice player is non-empty, returns without stopping it, pushes one queued voice, and returns again without starting it; see `src/audio/sfx.rs:287..309` and `src/audio/sfx.rs:333..348`.

Native `QueueVoice` resolves default type/priority from INI, inserts Type 1 into `PriorityQueue[priority]`, then `PlayNextQueued` returns while the stream is currently playing; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:143..161`, `168..175`, and `179..185`.

Output for this concrete cue while current voice is active: current voice continues; one pending EVA is retained.

Verdict: PASS for the previous cutoff bug on `StructureAbandoned`.

### Stage 4 - Duplicate current/queued sound id suppression

Rust suppresses if `current_voice_id == sound_id` or the same `sound_id` is already in `queued_voice`; see `src/audio/sfx.rs:289..296`.

Native suppresses the currently playing Vox entry and uses `FindInQueues(vox)` with matching type before insertion; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:140..155` and `docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md:57..59`.

For repeating the same current app EVA cue in the same faction, both suppress the duplicate insertion.

Verdict: PASS for same-event same-faction duplicate suppression.

### Stage 5 - UnitReady/CannotDeployHere are not native Type=QUEUE

Rust routes `UnitReady` and `CannotDeployHere` through the same queued FIFO path as `StructureAbandoned`; see `src/app_building_anim.rs:603..613`.

Native `EVA_UnitReady` and `EVA_CannotDeployHere` have `Priority=LOW` but no `Type=QUEUE` key in `ini/evamd.ini:1095..1107`. `VoxClass__ReadEVAINI` defaults missing `Type` to STANDARD=0 and missing priority to NORMAL before `ReadINI`; per-section Type/Priority fields are documented at `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:225..244`. `STANDARD`/`INTERRUPT` entries are not normal queued announcements and are discarded/pending-immediate under active queue/playback conditions rather than appended to the Type=QUEUE priority queues; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:168..175`.

Player-visible difference: while a voice is playing, Rust can retain and later play `UnitReady` or `CannotDeployHere`; active YR does not treat those sampled entries as Type=QUEUE retained messages.

Verdict: FAIL.

### Stage 6 - Priority queues and type-specific ordering are collapsed

Rust has one FIFO `queued_voice: VecDeque<QueuedVoice>` and stores only `sound_id`, decoded audio, and volume; see `src/audio/sfx.rs:134..137` and `src/audio/sfx.rs:303..307`.

Native has `InterruptQueue`, `CriticalQueue`, `PendingImmediate`, and four `PriorityQueue[n]` heads, and `PlayNextQueued` drains in priority/type order; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:197..201` and `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md:1006..1028`.

Player-visible difference: mixed current app EVA cues can play in Rust FIFO order when active YR would route by Type/Priority or discard STANDARD entries.

Verdict: FAIL.

### Stage 7 - Inter-announcement delay

Rust starts the next queued EVA as soon as `voice_player.empty()` is true; see `src/audio/sfx.rs:333..348`. There is no stored end timestamp and no delay check.

Native waits until `currentTime >= StreamPlayer.EndTime + InterAnnouncementDelay` and sets `InterAnnouncementDelay = 500` after successful playback; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:187..215` and global delay fields at `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md:1027..1028`.

Player-visible difference: Rust can start the next queued EVA 500 ms earlier than active YR.

Verdict: FAIL.

### Stage 8 - Pause/suspend/audio-off guards

Native `PlayNextQueued` checks game pause, audio off, stream existence, stream active, and pause counter before dequeueing; `QueueVoice` checks suspend counter; see `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:136..139` and `179..185`.

This concrete scenario did not include paused/suspended/audio-off state, and Rust behavior for those states was not measured here.

Verdict: UNCHECKED.

### Stage 9 - Vox entry identity vs sound-id identity

Rust duplicate suppression keys on resolved clip `sound_id`; native duplicate suppression keys on the `VoxClass*` entry and type. In this concrete same-event scenario the keys coincide, but equivalence across all current app-routed EVA entries was not proven.

Verdict: UNCHECKED.

## Failures

1. `UnitReady` and `CannotDeployHere` are routed as retained FIFO queued EVA in Rust even though the sampled INI entries are native STANDARD, not Type=QUEUE. Root cause: Rust resolves to clip id and loses VoxClass Type/Priority metadata.
2. Type/Priority ordering is collapsed to FIFO. Root cause: `SfxPlayer` has one `VecDeque` and no Vox priority queues.
3. Native 500 ms inter-announcement delay is absent. Root cause: Rust does not store stream end time or `InterAnnouncementDelay`.

## Adjacent Findings

- `StructureGarrisoned` and `StructureAbandoned` are the current app garrison EVA events that are verified `Type=QUEUE`, `Priority=NORMAL`.
- Full native priority, interrupt, queued-interrupt, pause/suspend, global EVA volume, and Vox save/load state remain outside this trace.
- A future fix should pass EVA event identity plus parsed `evamd.ini` Type/Priority into audio, not only a faction-resolved clip id.

## Verdict Tally

PASS: 4 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0
