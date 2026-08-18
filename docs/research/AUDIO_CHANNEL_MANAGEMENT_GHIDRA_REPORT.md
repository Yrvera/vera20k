# Audio Channel Management — Ghidra Research Report

**Binary**: gamemd.exe (Yuri's Revenge)
**Date**: 2026-03-22
**Confidence**: HIGH (verified from binary decompilation)

---

## 1. Architecture Overview

The audio system has four distinct subsystems, each with its own buffer/channel management:

| Subsystem | Purpose | Buffer Type | Global |
|-----------|---------|-------------|--------|
| **SFX (Sound Events)** | In-game sound effects | 16 DirectSound buffers | `0x0087e728` (DSoundDevice) |
| **Voice Queue** | Unit voice responses | 1 streaming buffer | `0x00b1d4cc` (VoiceStreamPlayer) |
| **Speech/EVA** | EVA announcements | 1 streaming buffer | `0x00b1d4d8` (SpeechStreamPlayer) |
| **Music** | Score playback | Separate system | Not covered here |

### Initialization Order (AudioSystem__Init @ 0x00406b10)
1. DirectSound device created (`0x00402c70`)
2. **16 DirectSound buffers** created (`DSoundChannel__CreateAll` @ 0x00403530, arg EDX=0x10)
3. Sound thread spawned (`SoundThread__Init` @ 0x00407550) — runs at elevated priority
4. Sound event pool initialized (`SoundEventPool__Init` @ 0x00403ed0) — 200 events, 0x2000 byte chunks
5. Voice system initialized (`VoiceSystem__Init` @ 0x00752290) — 1 streaming buffer (3000ms)
6. Speech/EVA system initialized (`SpeechSystem__Init` @ 0x00752ad0) — 1 streaming buffer (3000ms)

---

## 2. Channel/Tracker System

### 2.1 DirectSound Buffers (Hardware Channels)

There are exactly **16 DirectSound secondary buffers** created at init. Each buffer struct is **0x1C0 bytes** (`DSoundBuffer__Create` @ 0x00402040).

Key fields of the DirectSound buffer struct (offset from buffer base):
| Offset | Size | Field |
|--------|------|-------|
| 0x03   | 4    | Status/type (1 = active) |
| 0x0C   | 4    | Category/type ID (used for channel selection) |
| 0x14   | 4    | Playing flag |
| 0x88   | 4    | Locked flag |
| 0x90   | 4    | Volume interpolator pointer |
| 0x94   | 4    | Pan interpolator pointer |
| 0x98   | 4    | Pitch interpolator pointer |
| 0x9C   | 4    | Additional interpolator |
| 0xA0   | 4    | Priority value |
| 0xA4   | 4    | Status flags (bit 0=playing, bit 2=streaming, bit 4=locked) |
| 0xA8   | 4    | Repeat count (-1 = infinite) |
| 0xAC   | 4    | Parent SampleTracker pointer |
| 0xB0   | 4    | Callback: fill buffer |
| 0xB4   | 4    | Callback: on complete |
| 0xB8   | 4    | Callback: on loop |
| 0xBC   | 4    | Callback: on stop |
| 0xC0   | 4    | Owner SoundEvent pointer |
| 0xD4   | 4    | VTable pointer (for Lock/Unlock/Play/Stop) |
| 0xDC   | 4    | Timestamp (for priority tie-breaking) |

### 2.2 Sound Event Pool

Sound events are allocated from a **fixed pool of 200 slots** (`SoundEventPool__Init` @ 0x00403ed0, first arg = 200). Each sound event is **0x280 bytes** (0xa0 dwords, zeroed on allocation in `SoundEvent__AllocateFromPool`).

Sound events are managed in a **doubly-linked list** rooted at `0x0087e180`. Active event count tracked at `0x0087e28c`, high-water mark at `0x0087e290`.

Key fields of the SoundEvent struct:
| Offset | Size | Field |
|--------|------|-------|
| 0x00   | 4    | Next pointer (linked list) |
| 0x04   | 4    | Prev pointer (linked list) |
| 0x18   | 4    | Flags (bit 0=dead, bit 1=started, bit 3=has_buffer, bit 4=played_outro, bit 5=suspended, bit 6=stop_requested) |
| 0x1C   | 4    | State (0=delay, 1=ready, 2=waiting, 3=playing, 4=done) |
| 0x20   | 4    | Next state after delay |
| 0x24   | 4    | Pointer to VocClass entry |
| 0x28   | 32×4 | Sample handle array (up to 32 loaded samples) |
| 0xA8   | 4    | Sample count |
| 0xAC   | 4    | Current sample index |
| 0xB0   | 4    | DirectSound buffer pointer |
| 0xB8   | 8×4  | Volume interpolator struct (current, target, rate, flags) |
| 0x138  | 4    | Sequence counter |
| 0x140  | 8    | Delay timestamp (64-bit) |
| 0x148  | 4    | Saved buffer handle |
| 0x14C  | 4    | Random delay value |
| 0x150  | 4    | Random pitch shift |
| 0x158  | 4    | External tick mode flag |
| 0x160  | 32×4 | Playlist index array |
| 0x1E0  | 4    | Playlist length |
| 0x1E4  | 4    | Loop iteration count |
| 0x1E8  | 32×4 | Next-round sample indices |
| 0x268  | 4    | Next-round count |
| 0x26C  | 4    | Total iterations played |
| 0x270  | 4    | Next-round current index |
| 0x274  | 4    | Saved random sample index |
| 0x278  | 4    | Back-pointer to source tracker |

---

## 3. Concurrent Sound Limit

### 3.1 Hard Limit: 16 DirectSound Buffers

The engine creates exactly **16 DirectSound secondary buffers**. This is the hard ceiling on simultaneously audible sounds. Verified from disassembly at 0x00406c25:
```asm
MOV EDX, 0x10        ; 16 buffers
MOV ECX, EAX         ; DSoundDevice
CALL DSoundChannel__CreateAll
CMP EAX, 0x10        ; all 16 must succeed
```

### 3.2 Soft Limit: 200 Sound Event Slots

The sound event pool has **200 slots**. Events can exist in queued/waiting states without needing a DirectSound buffer, so more than 16 sounds can be "active" but only 16 play simultaneously.

### 3.3 Per-VocClass Limit

Each VocClass entry has a `Limit` field (INI key: `Limit`, offset 0x48, default 5) that caps how many concurrent instances of that specific sound can play.

### 3.4 Per-VocClass Range

Each VocClass entry has a `Range` field (INI key: `Range`, offset 0x50, default 10) used for distance-based audibility.

---

## 4. Priority System

### 4.1 Priority Levels

Parsed from INI key `Priority` (default: "NORMAL"). String lookup table at `0x00816018`:

| String | Value | Meaning |
|--------|-------|---------|
| LOWEST | 0 | Background/ambient, easily evicted |
| LOW | 1 | Low priority |
| NORMAL | 2 | Default priority |
| HIGH | 3 | Important sounds |
| CRITICAL | 4 | Never evicted (e.g., EVA warnings) |

### 4.2 Eviction/Preemption Algorithm

When a new sound needs to play but all 16 DirectSound buffers are occupied, the engine runs an eviction loop in `SoundSystem__UpdateTick` (@ 0x004041d0):

1. Attempt to allocate a DirectSound buffer via `SoundEvent__PreparePlayout` (@ 0x00404700)
2. If no buffer available, call `DSoundChannel__FindLowestPriority` (@ 0x00404e20)
3. The found channel's owning sound event is stopped via `SoundEvent__Stop`
4. Its VocClass is located by scanning the active event list for matching VocClass pointers
5. The freed buffer is reassigned to the new sound
6. Repeat until a buffer is available or no evictable sound exists

### 4.3 Channel Selection Algorithm (DSoundChannel__FindAvailable @ 0x004035f0)

When allocating a channel for playback:

1. **First pass**: Search for an idle channel of the same type (category). If found, return immediately.
2. **Second pass**: Among busy channels of the same type:
   - Track the **lowest priority** among locked channels (`status & 0x15 != 0` means locked/playing/streaming)
   - Track the **lowest priority** among non-locked channels
   - For tied priorities, prefer the **oldest** channel (by timestamp at offset 0xDC, with a minimum age threshold of 0x665 ticks)
3. Return the best eviction candidate (prefer non-locked over locked)

### 4.4 Dynamic Priority Adjustment

During `SoundSystem__UpdateTick`, each active sound event's effective priority is computed as:

```
effective_priority = VocClass.priority (offset 0x40) + event.dynamic_bonus (offset 0x55*4)
```

The engine tracks the highest priority per VocClass across all active events (stored at VocClass offsets 0x9C-0xB0). When multiple events compete for the same VocClass, only the highest-priority one retains its buffer; lower ones are stopped.

---

## 5. Volume and Distance

### 5.1 Volume Calculation (VocClass__CalcVolumeAndPan @ 0x00750ac0)

The function computes volume attenuation based on distance from the camera viewport:

1. **Get audible range**: `range = VocClass.Range (offset 0x50) * 60` (in leptons → pixels)
2. **Convert source position to screen coordinates** via `TacticalClass__CoordsToClient2`
3. **Compute absolute offsets** from screen center (X and Y)
4. **Apply viewport margin**: Unless `PLAYER` type flag (0x40) is set, subtract half the viewport width/height from the offsets (sounds within viewport are at full volume)
5. **Distance attenuation**:
   ```
   distance = max(abs_x, abs_y * 2)   // Y is doubled (isometric compensation)
   if distance < range:
       volume = (range - distance) / range   // linear falloff
   else:
       volume = 0.0
   ```
6. **MinVolume floor**: If type flag 0x10 (`GLOBAL`) is set and calculated volume < MinVolume, use MinVolume (VocClass offset 0x54)
7. **Minimum audibility threshold**: If volume < threshold (global at `0x007e8ae8`), sound is inaudible — return 0.0

### 5.2 Volume Interpolation

Volume changes are **interpolated over time**, not applied instantly. The interpolation system uses structs with fields:
- Flags (bit 0 = jump immediately, bit 1 = changed)
- Current value (fixed-point, upper 16 bits = value)
- Target value
- Rate (units per tick)
- Timestamp

Functions `FUN_00407150` (set target) and `FUN_004071c0` (tick interpolation) handle smooth fading. Three interpolators per sound event: volume, pan, pitch.

### 5.3 SHROUD Type (0x800)

If the sound has the SHROUD type flag, it checks whether the source cell is under fog of war. If the cell is shrouded (not revealed), the sound is silenced (returns volume 0.0). This prevents hearing enemy sounds you shouldn't know about.

---

## 6. Control Modes

Parsed from INI key `Control` as a comma-separated list of flags. Lookup table at `0x008160c0`:

| String | Bit | Effect |
|--------|-----|--------|
| LOOP | 0x01 | Sound loops after finishing. Respects `LoopCount` (VocClass offset 0x4C) if set. |
| RANDOM | 0x02 | Randomly select which sample variation to play (from `Sounds=` list) |
| ALL | 0x04 | Play ALL sample variations simultaneously (layered) |
| INTERRUPT | 0x08 | Used with PREDELAY; allows predelay to be interrupted |
| PREDELAY | 0x10 | Random delay before playback starts (range from VocClass offsets 0x58-0x5C) |
| ATTACK | 0x20 | Has attack envelope (fade-in). Uses `Attack` value (VocClass offset 0x138). The first sample in the Sounds list is the attack sample. |
| DECAY | 0x40 | Has decay envelope (fade-out). Uses `Decay` value (VocClass offset 0x13C). The last sample in the Sounds list is the decay sample. |
| AMBIENT | 0x80 | Marks as ambient sound (affects how volume is applied) |

### 6.1 Sample Selection Logic (SoundEvent__LoadSamples @ 0x004048b0)

Based on control flags, the engine selects samples from the VocClass `Sounds=` list differently:

- **No flags (sequential)**: Load the first non-attack, non-decay sample
- **RANDOM (0x02)**: Pick a random sample from the main range (excluding attack/decay)
- **ALL (0x04)**: Load ALL main samples (played simultaneously/layered)
- **ATTACK (0x20)**: Load the attack sample as a prefix
- **DECAY (0x40)**: Load the decay sample as a suffix

### 6.2 Playlist Advancement (SoundEvent__AdvancePlaylist @ 0x004047b0)

After the current playlist finishes:
- **RANDOM**: Pick a new random sample from the playlist
- **LOOP**: Re-queue the same playlist (respecting LoopCount limit)
- **DECAY**: Play the decay sample once after the main loop ends
- Otherwise: Sound finishes and the event is freed

---

## 7. Type/Category Flags

Parsed from the second `Control`-like field in soundmd.ini (the unnamed field parsed at 0x00406870). Lookup table at `0x00816048`:

| String | Bit | Effect |
|--------|-----|--------|
| VIOLENT | 0x01 | Combat/weapon sound |
| MOVEMENT | 0x02 | Movement/locomotion sound |
| QUIET | 0x04 | Reduced volume category |
| LOUD | 0x08 | Increased volume category |
| GLOBAL | 0x10 | Audible everywhere (uses MinVolume as floor) |
| SCREEN | 0x20 | Position-relative to screen (stereo panning) |
| LOCAL | 0x40 | PLAYER-positioned (no viewport margin subtracted) |
| PLAYER | 0x80 | Same as LOCAL — positioned relative to player |
| AMBIENT | 0x1000 | Ambient sound (special handling in type field) |
| NOISE_SHY | 0x100 | Suppressed during combat noise |
| GUN_SHY | 0x200 | Suppressed during gunfire |
| UNSHROUD | 0x400 | Reveals shroud at source location |
| SHROUD | 0x800 | Only audible if source is unshrouded |
| NORMAL | 0x00 | Default (no special flags) |

### 7.1 Mutual Exclusion

The type parser (@ 0x00406870) has special logic: bits 0x20 and 0x40 are mutually exclusive (SCREEN vs LOCAL) — setting one clears the other. Similarly for 0x400 and 0x800 (UNSHROUD vs SHROUD).

---

## 8. Positional Audio / Stereo Panning

### 8.1 Pan Calculation

The engine performs **stereo panning** based on screen position. In `VocClass__CalcVolumeAndPan`:

1. Source coordinates are converted to screen-space via `TacticalClass__CoordsToClient2`
2. The X offset from screen center is computed
3. Pan value is output through `param_2` (range: presumably -10000 to +10000 matching DirectSound pan range)

### 8.2 Pan Application

Pan is applied through the volume interpolation system alongside volume. The `FUN_00402220` function applies volume, pan, and pitch to the DirectSound buffer via the interpolator struct chain:
- Volume interpolator (offset 0x90)
- Pan interpolator (offset 0x94)
- Pitch interpolator (offset 0x98)
- Additional modifier (offset 0x9C)

### 8.3 VShift (Pitch Variation)

The `VShift` INI key (VocClass offset 0x68, clamped 0-100) applies random pitch variation to each playback instance, adding variation to repeated sounds.

---

## 9. Voice Response System

### 9.1 Architecture

The voice system uses a **dedicated streaming buffer** (created via `StreamPlayer__Create` at 0x00407860, with 3000ms buffer size) separate from the 16 SFX channels.

Key globals:
| Address | Purpose |
|---------|---------|
| 0x00b1d4cc | Voice stream player pointer |
| 0x00b1d4c4 | Currently playing voice entry |
| 0x00b1d4b8 | Pending immediate voice |
| 0x00b1d3d8 | Voice lock counter (>0 = voice playback blocked) |
| 0x00b1d428 | Voice pause flag |
| 0x00b1d4c0 | Voice sequence counter |
| 0x00b1d4a4 | VocClass array pointer |
| 0x00b1d4b0 | VocClass array count |
| 0x00b1d450..0x00b1d480 | Per-priority voice queues (4 queues, 0xC bytes each) |

### 9.2 Voice Queuing (VoiceSystem__QueueVoice @ 0x00752480)

When a unit voice is triggered:

1. **Check guard**: Voice lock counter (`0x00b1d3d8`) must be 0
2. **Check duplicate**: Skip if the same VocClass entry is already the currently playing voice
3. **Priority interrupt**: If `param_2 == 2` (high priority), **immediately stop** the current voice, clear all queued voices, and flush the stream buffer
4. **Check existing**: If this VocClass is already queued, skip re-queuing (unless priority differs)
5. **Queue entry**: Allocate a 0x20-byte queue node with fields:
   - Offset 0x0C: VocClass pointer
   - Offset 0x14: Priority value
   - Offset 0x18: Queue category
   - Offset 0x1C: Sequence number (modulo 100)
6. **Route to queue** based on category:
   - Category 3: High-priority queue
   - Category 1: Normal queue
   - Category 3 (param_3): Urgent queue
   - Otherwise: Only plays if no other voices are queued AND has higher priority than current pending voice

### 9.3 Voice Playback (VoiceSystem__PlayNextQueued @ 0x00752760)

Dequeues and plays voices in priority order:

1. Wait for current voice to finish (checks stream completion timestamp + 500ms gap)
2. Dequeue from highest-priority queue first:
   - High-priority queue (category 3) checked first
   - Then normal queue (category 1)
   - Then pending immediate voice (`0x00b1d4b8`)
   - Then per-priority queues (`0x00b1d450..0x00b1d474`)
3. Select the audio file based on country (3 variants at VocClass offsets 0x2C, 0x35, 0x3E — indexed by `DAT_00b1d4c8` for Allied/Soviet/Yuri)
4. Append ".wav" extension
5. Start streaming via `FUN_00407b60`
6. Set inter-voice gap to **500ms** (`DAT_00b1d4d0 = 500`)

### 9.4 Previous Voice Behavior

**Yes, previous voices are cut off** when a new voice with priority 2 arrives. The code at `VoiceSystem__QueueVoice` explicitly:
- Stops the current playing voice
- Clears all queued voices
- Flushes the stream buffer

For **lower-priority voices**, the previous voice plays to completion, then the next queued voice starts after a 500ms gap.

### 9.5 Voice Lock

Functions at `0x00753570` (increment lock) and `0x00753580` (decrement lock) control a lock counter. While locked (counter > 0), no new voices can be queued. This prevents voice spam during rapid selections.

---

## 10. Sound Categories (Separate Volume Controls)

The engine does NOT have separate runtime volume sliders per category in the way modern engines do. Instead:

1. **SFX sounds** share the 16 DirectSound buffers with a single master SFX volume
2. **Voice responses** use a dedicated stream buffer — effectively a separate "voice channel"
3. **EVA/Speech** uses another dedicated stream buffer — a separate "speech channel"
4. **Music** has its own system entirely

The `Type` flags (VIOLENT, MOVEMENT, QUIET, LOUD, etc.) affect **which channel** is selected and **priority-based eviction**, but not independent volume control. QUIET and LOUD flags may affect the effective volume multiplier during the volume application step.

Global volume controls (from RulesClass/options):
- SFX volume multiplier
- Voice volume (affects the voice stream)
- Music volume (separate system)

---

## 11. Key Functions Summary

| Address | Name | Purpose |
|---------|------|---------|
| 0x00406b10 | `AudioSystem__Init` | Master init: DSound device, buffers, thread, pools |
| 0x00406d40 | `AudioSystem__Shutdown` | Teardown all audio subsystems |
| 0x00407550 | `SoundThread__Init` | Creates audio mixing thread at elevated priority |
| 0x00403ed0 | `SoundEventPool__Init` | Creates 200-slot event pool |
| 0x00405190 | `SoundEvent__AllocateFromPool` | Allocates an event from the free pool |
| 0x004052f0 | `SoundEvent__Stop` | Stops an event, releases its buffer |
| 0x004041d0 | `SoundSystem__UpdateTick` | **Main per-frame update**: priority resolution, eviction, playback start |
| 0x004055c0 | `SoundEvent__UpdateState` | Per-event state machine (delay→ready→playing→done) |
| 0x004048b0 | `SoundEvent__LoadSamples` | Loads sample data based on control flags |
| 0x00404700 | `SoundEvent__PreparePlayout` | Builds playlist from loaded samples |
| 0x004047b0 | `SoundEvent__AdvancePlaylist` | Moves to next sample in playlist (loop/random/sequential) |
| 0x00404bb0 | `SoundEvent__SelectNextSample` | Selects which sample variation to play next |
| 0x004054a0 | `SoundEvent__StartPlayback` | Locks buffer, sets volume/pan, calls DSoundBuffer::Play |
| 0x00404e20 | `DSoundChannel__FindLowestPriority` | Finds eviction candidate among busy channels |
| 0x004035f0 | `DSoundChannel__FindAvailable` | Finds idle or lowest-priority channel |
| 0x00403530 | `DSoundChannel__CreateAll` | Creates N DirectSound secondary buffers |
| 0x00402040 | `DSoundBuffer__Create` | Creates one DirectSound buffer (0x1C0 bytes) |
| 0x00750ac0 | `VocClass__CalcVolumeAndPan` | Distance-based volume attenuation + stereo pan |
| 0x00406670 | `VocClass__PlayGlobal` | Play a sound without position (UI sounds) |
| 0x00750920 | `AnimClass__SpawnAtCoord` (labeled CreditUpDown_Sound) | Play positional sound at world coordinates |
| 0x00750da6 | `AnimClass__SpawnDetached` | Play positional sound (detached from anim) |
| 0x00752480 | `VoiceSystem__QueueVoice` | Queue a unit voice response |
| 0x00752760 | `VoiceSystem__PlayNextQueued` | Dequeue and stream next voice |
| 0x00752290 | `VoiceSystem__Init` | Init voice subsystem (1 stream buffer) |
| 0x00752ad0 | `SpeechSystem__Init` | Init EVA/speech subsystem (1 stream buffer) |
| 0x00750440 | `VocClass__ReadINI` | Parse VocClass entry from soundmd.ini |
| 0x004064a0 | `VocClass__AddSample` | Add a sample file to a VocClass entry |
| 0x00401c00 | `SampleTracker__LoadSample` | Load sample data into memory |
| 0x00405b50 | `StreamBuffer__Allocate` | Allocate a streaming playback buffer |
| 0x00407860 | `StreamPlayer__Create` | Create a streaming player (for voice/speech) |

---

## 12. VocClass Struct Layout

Each VocClass entry (from soundmd.ini). Size unknown exactly but fields verified up to 0x140:

| Offset | Size | INI Key | Description |
|--------|------|---------|-------------|
| 0x00   | 4    | —       | VTable/type pointer |
| 0x0C   | 4    | —       | Valid flag (0 if sample missing) |
| 0x10   | 4    | Control | Control flags bitmask |
| 0x14   | 4    | (type)  | Type/category flags bitmask |
| 0x1C   | 4    | —       | Volume (converted from float, stored as fixed) |
| 0x24   | 4    | —       | Internal state pointer |
| 0x40   | 4    | Priority | Priority level (0-4) |
| 0x44   | 4    | —       | Instance count (per-tick) |
| 0x48   | 4    | Limit   | Max concurrent instances (default 5) |
| 0x4C   | 4    | LoopCount | Max loop iterations (0=infinite when LOOP set) |
| 0x50   | 4    | Range   | Audible range in cells (default 10) |
| 0x54   | 4    | MinVolume | Minimum volume floor (float) |
| 0x58   | 4    | Delay (min) | Minimum pre-delay (ms) |
| 0x5C   | 4    | Delay (max) | Maximum pre-delay (ms) |
| 0x60   | 4    | FShift (min) | Minimum frequency shift |
| 0x64   | 4    | FShift (max) | Maximum frequency shift |
| 0x68   | 4    | VShift  | Volume shift / pitch variation (0-100) |
| 0x8C   | 8    | —       | Linked list of active instances |
| 0x98   | 4    | —       | Last update tick |
| 0x9C   | 4    | —       | Highest priority tick |
| 0xA0   | 4    | —       | Highest priority value |
| 0xB0   | 4    | —       | Highest priority distance |
| 0xB4   | 32×4 | Sounds  | Sample index array (up to 32 entries) |
| 0x134  | 4    | —       | Total sample count |
| 0x138  | 4    | Attack  | Attack sample count (prefix samples) |
| 0x13C  | 4    | Decay   | Decay sample count (suffix samples) |

---

## 13. Sound Thread

The engine runs a **dedicated audio thread** (created in `SoundThread__Init` @ 0x00407550):

- Stack size: 0x1000 (4KB)
- Priority: `THREAD_PRIORITY_ABOVE_NORMAL` (2)
- Entry point: 0x00407680
- Protected by a `CRITICAL_SECTION` at `0x0087e7f8`

The thread loop:
1. Get high-resolution timestamp
2. Lock critical section
3. Iterate all active streaming channels:
   - If channel has the "needs data" flag (bit 2 at offset 0x0C): fill buffer from source data via `FUN_00408f80`
   - Track remaining bytes; clear "needs data" flag when done
4. Unlock critical section
5. Wait on an event object
6. Repeat until shutdown flag (`0x0087e770`) is set

---

## 14. Summary of Key Design Decisions

1. **16 channels is the hard limit** — matches typical DirectSound hardware of the era
2. **Priority-based eviction** — when all channels are full, the lowest-priority sound is stopped to make room
3. **Priority tie-breaking uses age** — older sounds of equal priority are evicted first (with a 0x665 minimum age to prevent thrashing)
4. **Voice responses use a separate stream** — they never compete with SFX for DirectSound buffers
5. **High-priority voices interrupt** — priority 2 voices immediately stop the current voice and clear the queue
6. **Linear distance falloff** — volume = (range - distance) / range, with isometric Y-doubling
7. **Per-VocClass instance limiting** — the `Limit` field prevents too many copies of the same sound
8. **Smooth volume transitions** — all volume/pan/pitch changes are interpolated over time, never snapped
