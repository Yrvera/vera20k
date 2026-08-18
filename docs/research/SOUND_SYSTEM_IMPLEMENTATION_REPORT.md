# gamemd.exe Sound System — Complete Implementation Report

**Date:** 2026-03-27
**Binary:** gamemd.exe (Yuri's Revenge 1.001)
**Confidence:** HIGH (all data verified from binary decompilation across 8 prior Ghidra reports)
**Purpose:** Unified reference for implementing the full sound system in the Rust engine

---

## 1. Architecture Overview

The sound system is **not** a single class. It's four independent subsystems sharing
some common infrastructure:

```
┌─────────────────────────────────────────────────────────────┐
│                    Audio Infrastructure                      │
│  DirectSound device (0x0087e728)                            │
│  AudioIndex (audio.idx/audio.bag) (0x0087e724)              │
│  Sound Thread (elevated priority, fills streaming buffers)  │
│  CRITICAL_SECTION (0x0087e7f8)                              │
└─────────────┬───────────┬───────────┬───────────┬───────────┘
              │           │           │           │
     ┌────────▼──┐  ┌─────▼─────┐ ┌──▼──────┐ ┌──▼─────┐
     │ SFX       │  │ Voice     │ │ EVA     │ │ Music  │
     │ 16 DS buf │  │ 1 stream  │ │ 1 stream│ │ (sep.) │
     │ 200 pool  │  │ (shared)  │ │ (shared)│ │        │
     │ VocClass  │  │ VoxClass  │ │ VoxClass│ │        │
     └───────────┘  └───────────┘ └─────────┘ └────────┘
```

**Key insight:** Voice and EVA share ONE StreamPlayer (`0x00b1d4cc`). Only one
voice/EVA line can play at a time. Taunts have a separate StreamPlayer (`0x00b1d4d8`).

### Initialization Order (`AudioSystem__Init` at `0x00406b10`)

1. Create DirectSound device (`0x00402c70`)
2. Set default format: 22050 Hz, stereo, 16-bit PCM
3. Create **16 DirectSound secondary buffers** (`DSoundChannel__CreateAll` at `0x00403530`)
4. Open `AUDIOMD.MIX` (fallback: `AUDIO.MIX`) → stored at `0x0087e734`
5. Build AudioIndex from `audio.idx` / `audio.bag` (`0x004011c0`)
6. Initialize **200-slot SoundEvent pool** (`SoundEventPool__Init` at `0x00403ed0`)
7. Spawn sound thread at `THREAD_PRIORITY_ABOVE_NORMAL` (`SoundThread__Init` at `0x00407550`)
8. Initialize voice system: 1 StreamPlayer, 3000ms buffer (`VoiceSystem__Init` at `0x00752290`)
9. Initialize speech system: 1 StreamPlayer, 3000ms buffer (`SpeechSystem__Init` at `0x00752ad0`)

### Master Enable Flags

| Address      | Purpose                          |
|-------------|----------------------------------|
| `0x008464ac` | Master sound enable (checked by every play call) |
| `0x0087e2a0` | Audio subsystem ready flag       |
| `0x0087e294` | DirectSound initialized flag     |

---

## 2. SFX System — The Core Sound Engine

### 2.1 VocClass: Sound Definitions (from `soundmd.ini`)

Every sound effect in the game is a **VocClass** entry. Parsed by `VocClass__ReadINI`
(`0x00750440`) from `[SoundList]` sections in `soundmd.ini`.

#### VocClass INI Fields

| INI Key    | Offset | Type      | Default   | Purpose |
|------------|--------|-----------|-----------|---------|
| `Sounds`   | 0xB4   | string[]  | (none)    | Space-separated `.aud` filenames; up to 32 samples per entry |
| `Volume`   | 0x1C   | float     | 80.0      | Base playback volume (0–100 scale) |
| `VShift`   | 0x68   | int       | 0         | Random pitch variation range (0–100) |
| `MinVolume` | 0x54  | float     | 20.0      | Volume floor for distance-attenuated sounds |
| `Priority` | 0x40   | enum      | NORMAL(2) | Eviction priority: LOWEST(0), LOW(1), NORMAL(2), HIGH(3), CRITICAL(4) |
| `Attack`   | 0x138  | int (ms)  | 0         | Fade-in duration (only if ATTACK control flag set) |
| `Decay`    | 0x13C  | int (ms)  | 0         | Fade-out duration (only if DECAY control flag set) |
| `Control`  | 0x10   | bitmask   | 0         | Playback behavior flags (see §2.5) |
| `Type`     | 0x14   | bitmask   | SCREEN    | Spatial/category flags (see §2.6) |
| `Limit`    | 0x48   | int       | 5         | Max concurrent instances of this sound |
| `Loop`     | 0x4C   | int       | 0         | Loop count (0 = no loop; -1 = infinite when LOOP flag set) |
| `Range`    | 0x50   | int       | 10        | Audible range in cells |
| `Delay`    | 0x58   | int[2]    | 0, 0      | Random pre-delay range: min, max (ms) |
| `FShift`   | 0x60   | int[2]    | 0, 0      | Frequency shift range: min, max |

Global defaults are read from `[AudioVisual]` before per-sound parsing:
- Default Volume: 80.0 (`0x008464B4`)
- Default MinVolume: 20.0 (`0x008464B8`)
- Default Range: 10 cells (`0x008464C0`)
- Default Limit: 5 (`0x008464C4`)

#### VocClass Lookup

`VocClass__FindByName` (`0x007514d0`): linear scan of the global VocClass array
(`DAT_00b1d37c`, count at `DAT_00b1d388`), case-insensitive string match against
entry name at offset +0x6C. Returns array index or -1.

### 2.2 The Main SFX Dispatcher: `VocClass__PlayAtPos` (`0x00750920`)

This is **THE** function. Nearly 100 call sites across the engine. Every positional
sound effect goes through it.

```c
int VocClass__PlayAtPos(int vocIndex, float volume, int loopSoundEventPtr)
```

**Parameters:**
- `vocIndex`: Index into global VocClass array
- `volume`: Usually `1.0f` (0x3f800000)
- `loopSoundEventPtr`: If non-zero, reuses/updates an existing SoundEvent handle

**Algorithm:**
1. Check `DAT_008464ac` (master sound enable). If disabled → return 0.
2. Validate `vocIndex` is in range `[0, array_count)`.
3. Resolve VocClass pointer from global array.
4. **Loop handle logic:** If `loopSoundEventPtr != 0`:
   - Check if a sound is already playing at that handle (`FUN_00406130`)
   - If playing with SAME VocClass → do nothing (sound continues)
   - If playing with DIFFERENT VocClass → stop old sound, start new one
   - If not playing → start new sound and store handle
5. Call `CalcVolumeAndPan` for spatial volume/pan.
6. Allocate SoundEvent from pool (`SoundEvent__AllocateFromPool` at `0x00405190`).
7. Configure volume, pan, pitch on the SoundEvent.
8. Return SoundEvent handle (or 0 on failure).

**Non-positional variant:** `VocClass__PlayGlobal` (`0x00406670`) — no spatial
calculation, full volume. Only 2 call sites (subtitle/caption playback).

### 2.3 Channel Management: 16 Buffers, 200 Events

#### Hard limit: 16 DirectSound buffers

Created at init. Each buffer struct is 0x1C0 bytes. This is the maximum number of
sounds that can be physically audible at the same time.

#### Soft limit: 200 SoundEvent pool slots

SoundEvents can exist in queued/waiting states without a buffer. The pool is a
doubly-linked list rooted at `0x0087e180`, with active count at `0x0087e28c`.

#### SoundEvent State Machine

Each SoundEvent progresses through states managed by `SoundEvent__UpdateState`
(`0x004055c0`):

```
  State 0: DELAY      → waiting for pre-delay timer
  State 1: READY      → ready to play, needs buffer allocation
  State 2: WAITING    → buffer requested, waiting for data load
  State 3: PLAYING    → actively playing through DirectSound buffer
  State 4: DONE       → finished, ready to be freed
```

### 2.4 Priority-Based Eviction Algorithm

When all 16 buffers are occupied and a new sound needs to play, the engine runs
eviction in `SoundSystem__UpdateTick` (`0x004041d0`):

**Step 1:** Try `DSoundChannel__FindAvailable` (`0x004035f0`):
1. First pass: find an idle channel of the same type → return immediately
2. Second pass among busy channels of same type:
   - Track lowest-priority locked channel (`status & 0x15`)
   - Track lowest-priority non-locked channel
   - For tied priorities: prefer the **oldest** channel (by timestamp at offset 0xDC)
   - Minimum age threshold: **0x665 ticks** (prevents thrashing)
3. Return best eviction candidate (prefer non-locked over locked)

**Step 2:** If a candidate is found:
1. Stop the candidate's SoundEvent via `SoundEvent__Stop`
2. Free its buffer
3. Assign buffer to new sound
4. Repeat until buffer available or no evictable sound exists

**Per-VocClass limiting:** Before eviction even happens, the engine checks the
VocClass `Limit` field (default 5). If this sound already has `Limit` instances
playing, the new request is dropped. This prevents a single rapid-fire weapon
from consuming all 16 channels.

**Dynamic priority adjustment:** During `SoundSystem__UpdateTick`, each active
SoundEvent's effective priority is computed as:
```
effective_priority = VocClass.priority + event.dynamic_bonus
```
When multiple events compete for the same VocClass, only the highest-priority one
retains its buffer.

### 2.5 Control Flags (INI key: `Control`)

Parsed from space-separated tokens. Lookup table at `0x008160C0`:

| Flag      | Value | Effect |
|-----------|-------|--------|
| `LOOP`    | 0x01  | Loop the sound. Respects `Loop=` count (0 = infinite). |
| `RANDOM`  | 0x02  | Pick a random sample from the `Sounds=` list each play. |
| `ALL`     | 0x04  | Play ALL samples simultaneously (layered). |
| `PREDELAY`| 0x08  | Random delay before playback (range from `Delay=` field). |
| `INTERRUPT`| 0x10 | Can be interrupted by higher-priority sounds. |
| `ATTACK`  | 0x20  | Fade-in envelope. First sample in Sounds= is the attack sample. If set and Attack=0, forces Attack=1. |
| `DECAY`   | 0x40  | Fade-out envelope. Last sample in Sounds= is the decay sample. If set and Decay=0, forces Decay=1. |
| `AMBIENT` | 0x80  | Ambient sound modifier. |

#### Sample Selection Logic (`SoundEvent__LoadSamples` at `0x004048b0`)

Based on control flags, samples from `Sounds=` list are selected differently:

- **No flags (default):** Load the first non-attack, non-decay sample.
- **RANDOM (0x02):** Pick a random sample from the main range (excluding attack/decay samples).
- **ALL (0x04):** Load ALL main samples (played simultaneously/layered).
- **ATTACK (0x20):** Prepend the attack sample before the main sample(s).
- **DECAY (0x40):** Append the decay sample after the main sample(s).

#### Playlist Advancement (`SoundEvent__AdvancePlaylist` at `0x004047b0`)

After the current playlist finishes:
- **RANDOM:** Pick a new random sample
- **LOOP:** Re-queue the same playlist (respecting `Loop=` count limit)
- **DECAY:** Play the decay sample once after main loop ends
- **Otherwise:** Sound finishes, SoundEvent is freed

### 2.6 Type Flags (INI key: `Type`)

Parsed from space-separated tokens. Lookup table at `0x00816048`.
Default: `SCREEN` (0x20).

| Flag       | Value  | Effect |
|------------|--------|--------|
| `VIOLENT`  | 0x0001 | Combat/explosion sound |
| `MOVEMENT` | 0x0002 | Movement/locomotion sound |
| `QUIET`    | 0x0004 | Reduced volume category |
| `LOUD`     | 0x0008 | Increased volume category |
| `GLOBAL`   | 0x0010 | MinVolume enforced; distance measured from screen center (no viewport-edge subtraction) |
| `SCREEN`   | 0x0020 | Normal positional sound relative to viewport |
| `LOCAL`    | 0x0040 | Like GLOBAL — no viewport-edge subtraction. Mutually exclusive with SCREEN |
| `PLAYER`   | 0x0080 | Only audible to owning player |
| `NOISE_SHY`| 0x0100 | Suppressed during combat noise |
| `GUN_SHY`  | 0x0200 | Suppressed during gunfire |
| `UNSHROUD` | 0x0400 | Reveals shroud at sound location. Mutually exclusive with SHROUD |
| `SHROUD`   | 0x0800 | Only audible if source cell is explored |
| `AMBIENT`  | 0x1000 | Ambient environmental sound |

**Mutual exclusion rules** (in `AudioEventClass::ParseTypeFlag` at `0x00406870`):
- Setting SCREEN (0x20) clears LOCAL (0x40), and vice versa
- Setting SHROUD (0x800) clears UNSHROUD (0x400), and vice versa

### 2.7 Volume Interpolation (Smooth Fading)

Volume, pan, and pitch changes are **never snapped** — they're always interpolated
over time via interpolator structs at SoundEvent offsets 0xB8 (volume), 0x90 (pan),
0x98 (pitch).

Each interpolator has:
- Current value (fixed-point, upper 16 bits = value)
- Target value
- Rate (units per tick)
- Flags (bit 0 = jump immediately, bit 1 = changed)

Functions `FUN_00407150` (set target) and `FUN_004071c0` (tick interpolation) handle
the ramping. This is what makes sound transitions feel smooth — no clicks or pops.

---

## 3. Spatial Audio Algorithm

### `VocClass__CalcVolumeAndPan` (`0x00750AC0`)

```c
float CalcVolumeAndPan(
    CoordStruct* coords,       // world position {X, Y, Z} in leptons
    int*         out_pan,       // output: 0 (full left) to 16384 (full right)
    AudioEventClass* event     // the sound's VocClass data
)
// Returns: volume 0.0..1.0
```

### Complete Algorithm (Pseudocode)

```c
float CalcVolumeAndPan(CoordStruct* coords, int* out_pan, AudioEventClass* event) {
    *out_pan = 0;

    // Step 1: Viewport dimensions
    float halfViewW = g_ViewportWidth * 0.5f;
    float halfViewH = g_ViewportHeight * 0.5f;

    // Step 2: Maximum audible distance
    float maxRange = (float)(event->Range * 60);   // 60 leptons per cell
    // ASM: LEA EAX,[EAX+EAX*2]; LEA EAX,[EAX+EAX*4]; SHL EAX,2 = × 60

    // Step 3: SHROUD check (Type flag 0x800)
    if (event->TypeFlags & 0x800) {
        short cellX = coords->X >> 8;
        short cellY = coords->Y >> 8;
        if (cellX == g_CameraCenterCellX && cellY == g_CameraCenterCellY)
            return 0.0f;   // edge case: at camera cell → silent
        CellClass* cell = Map.GetCell(cellX, cellY);
        if ((cell->ShroudBits & 0x18) == 0)
            return 0.0f;   // cell not explored → can't hear it
    }

    // Step 4: World → screen conversion
    PointStruct clientXY;
    Tactical->CoordsToClient2(coords, &clientXY);

    // Step 5: Signed offset from screen center
    float offsetX = (float)clientXY.x - halfViewW;   // positive = right of center
    float offsetY = (float)clientXY.y - halfViewH;

    // Step 6: Absolute distances
    float distX = fabsf(offsetX);
    float distY = fabsf(offsetY);

    // Step 7: Viewport-edge subtraction (unless GLOBAL/LOCAL flag)
    //   Sounds ON SCREEN have zero distance → full volume
    if (!(event->TypeFlags & 0x40)) {   // not LOCAL/GLOBAL
        distX = max(0, distX - halfViewW);
        distY = max(0, distY - halfViewH);
    }

    // Step 8: Double Y for isometric compensation
    //   Isometric projection compresses Y by 2:1
    //   Doubling Y makes attenuation isotropic in world space
    distY = distY * 2.0f;

    // Step 9: Volume = linear falloff using Chebyshev distance
    float volume = 0.0f;
    if (distX < maxRange && distY < maxRange && maxRange > 0.0f) {
        float effectiveDist = max(distX, distY);   // Chebyshev (max of axes)
        volume = (maxRange - effectiveDist) / maxRange;
    }

    // Step 10: MinVolume floor (if GLOBAL flag 0x10)
    if ((event->TypeFlags & 0x10) && volume < event->MinVolume) {
        volume = event->MinVolume;
    }

    // Step 11: Threshold — sounds below 5% are culled entirely
    if (volume < 0.05f)     // constant at 0x007E8AE8
        return 0.0f;

    // Step 12: Pan calculation
    //   Map horizontal screen offset to 0..16384 range (8192 = center)
    float panOffset = clamp(-offsetX, -g_ViewportWidth, g_ViewportWidth);
    float pan = (panOffset * 8192.0f) / g_ViewportWidth + 8192.0f;
    *out_pan = (int)pan;

    return volume;
}
```

### Key Properties

| Property | Value |
|----------|-------|
| Distance metric | Chebyshev (max of X, Y axes) |
| Falloff curve | **Linear**: `(range - dist) / range` |
| On-screen behavior | Full volume (viewport-edge subtracted first) |
| Y-axis compensation | Doubled (isometric 2:1 ratio) |
| Minimum audible | 5% volume threshold |
| Range units | Cells × 60 = leptons |
| Pan range | 0 (left) – 8192 (center) – 16384 (right) |

---

## 4. Voice System (Unit Responses)

### 4.1 Architecture

Unit voices (VoiceSelect, VoiceMove, VoiceAttack, etc.) use a completely separate
playback path from SFX. They go through a **StreamPlayer** — a streaming audio
buffer — not the 16 DirectSound SFX channels.

```
User clicks unit
  → TechnoClass__Select (0x006fbfa0)
    → vtable[0x360]()          // voice response handler
      → VoxClass__QueueVoice   // (0x00752480)
        → VoxClass__PlayNextQueued  // (0x00752760)
          → FUN_00407b60()     // load .aud into StreamPlayer
            → StreamPlayer plays via dedicated DirectSound buffer
```

**The voice StreamPlayer is at `DAT_00b1d4cc`.** This is shared with EVA.
Only one voice OR EVA line plays at a time.

### 4.2 Voice INI Keys (on TechnoTypeClass)

Each is a `DynamicVectorClass` of VocClass indices, parsed via `FUN_00478720`
(comma-separated VocClass name list):

| Offset | INI Key | When Triggered |
|--------|---------|----------------|
| 0x414  | `VoiceSelect=` | Unit clicked/selected |
| 0x430  | `VoiceSelectEnslaved=` | Selected while mind-controlled |
| 0x44C  | `VoiceSelectDeactivated=` | Selected while EMP'd/deactivated |
| 0x468  | `VoiceMove=` | Move order issued |
| 0x484  | `VoiceAttack=` | Attack order issued |
| 0x4A0  | `VoiceSpecialAttack=` | Special weapon attack ordered |
| 0x4BC  | `VoiceDie=` | Unit killed (death scream) |
| 0x4D8  | `VoiceFeedback=` | General feedback |

Additional voice keys: `VoiceHarvest`, `VoiceCapture`, `VoiceEnter`, `VoiceSinking`,
`VoiceCrashing`, `VoiceFalling`, `VoiceDeploy`, `VoiceUndeploy`.

### 4.3 Voice Selection from Multiple Entries

When a voice key has multiple entries (e.g., `VoiceSelect=GISelect1,GISelect2,GISelect3`),
the selection is done via `SoundEvent__SelectNextSample` (`0x00404bb0`).

The behavior depends on the VocClass's Control flags:
- **RANDOM (0x02):** Random selection from the list each time
- **No RANDOM flag:** Sequential/round-robin through the list

### 4.4 Voice Interruption & Priority

`VoxClass__QueueVoice` (`0x00752480`):

```c
void QueueVoice(int voxIndex, int priority, int queueSlot)
```

**Priority 2 = INTERRUPT:**
1. Immediately stop current StreamPlayer playback
2. Clear ALL queued voices
3. Flush the stream buffer
4. Insert new voice at queue head
5. Call `PlayNextQueued` to start immediately

**Lower priorities:**
1. Check guard: voice lock counter (`0x00b1d3d8`) must be 0
2. Skip if same VocClass is already playing
3. Skip if same VocClass is already queued (unless priority differs)
4. Allocate 0x20-byte queue node with VocClass pointer, priority, sequence number
5. Insert into appropriate priority queue
6. Wait for current voice to finish

**Result:** Clicking a new unit immediately cuts the previous voice.
EVA messages queue behind each other and play sequentially.

### 4.5 Voice Playback (`VoxClass__PlayNextQueued` at `0x00752760`)

Dequeues voices in priority order:
1. Wait for current voice to finish (stream completion + 500ms gap)
2. Check queues in priority order:
   - High-priority queue (category 3) first
   - Normal queue (category 1)
   - Pending immediate voice (`0x00b1d4b8`)
   - Per-priority queues (`0x00b1d450..0x00b1d474`)
3. Select .aud file based on faction:
   - VoxClass offset +0x2C: Allied sound name
   - VoxClass offset +0x35: Soviet sound name
   - VoxClass offset +0x3E: Yuri sound name
   - Faction index from `DAT_00b1d4c8` (0=Allied, 1=Soviet, 2=Yuri)
4. Append ".aud" extension
5. Start streaming via `FUN_00407b60`
6. Set inter-voice gap: `DAT_00b1d4d0 = 500` (500ms)

### 4.6 Voice Lock

`VoxClass__SuspendEVA` (`0x00753570`): increments lock counter at `0x00b1d3d8`
`VoxClass__ResumeEVA` (`0x00753580`): decrements lock counter

While locked (counter > 0), no new voices can be queued. Used during movies,
loading screens, etc.

---

## 5. EVA Announcement System

### 5.1 Architecture

EVA uses the VoxClass system. Each EVA event is a VoxClass entry loaded from
`EVAMD.INI` via `VoxClass__ReadEVAINI` (`0x00753000`).

**VoxClass struct** (84 bytes):

| Offset | Size | Field |
|--------|------|-------|
| 0x00   | 4    | VTable pointer |
| 0x04   | 40   | Name (null-terminated string) |
| 0x2C   | 9    | Allied sound filename (without extension) |
| 0x35   | 9    | Soviet sound filename |
| 0x3E   | 9    | Yuri sound filename |
| 0x48   | 4    | Priority (0–3) |
| 0x4C   | 4    | Type: 0=STANDARD, 1=QUEUE, 2=INTERRUPT, 3=QUEUED_INTERRUPT |
| 0x50   | 4    | PlayState / reference count |

### 5.2 EVA Event Types

| Type | Name | Behavior |
|------|------|----------|
| 0    | STANDARD | Fire-and-forget. If channel busy → **silently dropped**. |
| 1    | QUEUE | Inserted into priority queue. Plays when channel available. |
| 2    | INTERRUPT | Flushes all queued events, stops current, plays immediately. |
| 3    | QUEUED_INTERRUPT | Flushes queue, but waits for current to finish, then plays. |

### 5.3 EVA Dispatch: `VoxClass__PlayEVA` (`0x00752700`)

Wrapper around `VoxClass__QueueVoice` with:
1. **Duplicate checking:** Searches all queues for the same VoxClass. If found → skip.
2. Routes to `QueueVoice` with appropriate priority/slot based on VoxClass Type field.

### 5.4 Queue Structure

4 priority queues + 2 special slots:

| Queue | Address | Purpose |
|-------|---------|---------|
| Interrupt queue | special slot | Type=2 events (immediate) |
| Critical queue | `0x00b1d450` | Highest priority queued events |
| Priority 0 | `0x00b1d45C` | Normal priority |
| Priority 1 | `0x00b1d468` | Lower priority |
| Priority 2 | `0x00b1d474` | Lowest priority |
| Pending immediate | `0x00b1d4b8` | Single-slot for immediate play |

Queue nodes are 0x20 bytes with VoxClass pointer, priority, category, and sequence number.

### 5.5 Rate Limiting & Deduplication

1. **Duplicate suppression:** `VoxClass__PlayEVA` scans all queues via `VoxClass__FindInQueues`
   (`0x00752680`). If the same VoxClass is already queued → request is dropped.
2. **Inter-announcement delay:** Hardcoded **500ms** gap between consecutive EVA messages
   (`DAT_00b1d4d0 = 500`). Uses `QueryPerformanceCounter` in milliseconds.
3. **STANDARD events are fire-and-forget:** If the channel is busy, they're silently dropped
   (not queued). This prevents "Unit lost" spam from stacking up.

### 5.6 Faction Selection

`VoxClass__SetSide` (`0x007534e0`) maps game side to VoxClass field:
- Side 0 (Allied) → offset +0x2C
- Side 1 (Soviet/Russian) → offset +0x35
- Side 2 (Yuri) → offset +0x3E

Current side stored at `DAT_00b1d4c8`.

### 5.7 Common EVA Events & Their Triggers

| EVA Event | Trigger Function | Type |
|-----------|-----------------|------|
| EVA_ConstructionComplete | `BuildingClass__OnConstructionComplete` | QUEUE |
| EVA_UnitReady | `FactoryClass__CompletedProduction` | QUEUE |
| EVA_BuildingInfiltrated | `BuildingClass__Infiltrate` | INTERRUPT |
| EVA_BaseUnderAttack | `HouseClass__BaseUnderAttack` | STANDARD |
| EVA_UnitLost | `TechnoClass__ReceiveDamage` (on death) | STANDARD |
| EVA_AllyUnderAttack | `HouseClass__AllyUnderAttack` | STANDARD |
| EVA_OreRunningLow | `TiberiumClass__GrowthUpdate` | STANDARD |
| EVA_NuclearSiloDetected | `SuperWeapon__Launch` | INTERRUPT |
| EVA_IronCurtainReady | `SuperWeapon__IsReady` | QUEUE |

37 total call sites to `VoxClass__PlayEVA` documented in `SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md`.

---

## 6. Sound Trigger Categories

### 6.1 Weapon Sounds

**INI keys on `[WeaponType]`:** `Report=`, `DownReport=`

**Parsed:** `WeaponTypeClass__ReadINI` (`0x00772080`)
- `Report` → offset 0xCC (vector of VocClass indices)
- `DownReport` → offset 0xE8 (vector, used when target is below firer)

**Triggered:** `TechnoClass__Fire_At` (`0x006fdd50`)

**Selection algorithm:**
1. If Report count == 8 → select by facing direction (8 directional reports)
2. Otherwise → use first entry
3. Elite override: if elite, use offset 0x110 instead
4. If no report and unit has garrison flag (0x82) → use garrison fire sound (0x118)
5. Play via `VocClass__PlayAtPos`

### 6.2 Death Sounds

Two separate sounds play on unit death:
1. **VoiceDie** (voice scream) — goes through VoxClass voice channel
2. **DieSound** (SFX explosion) — goes through SFX channel via `VocClass__PlayAtPos`

Both select randomly from their sound lists.

### 6.3 Ambient / Working Sounds

**INI key:** `AmbientSound=` on `[ObjectType]`

**Algorithm:** In `TechnoClass__AI_Update` (`0x006f9e50`), every tick:
1. Check if `field_0x4f0` (current ambient VocClass index) != -1
2. If a sound is already playing at the handle → check if VocClass changed
3. If same VocClass → do nothing (let it keep playing)
4. If different → stop old, start new
5. If not playing → start new
6. Reset `field_0x4f0 = -1` for next tick (sim layer re-sets it each tick)

This is NOT a true DirectSound loop — it's re-triggered every frame with a
handle check to prevent duplicate overlapping playback.

### 6.4 Movement Sounds

**INI key:** `MoveSound=` on `[TechnoType]`

Movement sounds are **NOT true loops**. They fire once per cell transition:
- In `DriveLocomotionClass__Process_Movement` (`0x004b2630`): when unit enters a
  new cell, sets `field_0x68a` flag
- Sound plays once via `VocClass__PlayAtPos` with `loopSoundEventPtr = 0`
- Flag cleared after sound plays

Also: `EnterWaterSound`, `LeaveWaterSound` for terrain transitions.

### 6.5 Animation Sounds

**INI keys on `[AnimType]` (in art.ini):** `StartSound=`, `Report=`, `StopSound=`

`StartSound` and `Report` share the same field (offset 0x2F8). `StartSound` is
tried first; if -1, falls back to `Report`.

**Three playback modes:**

1. **Continuous (every tick in AI):** `AnimClass::AI` (`0x00423AC0`) checks
   `Type->StartSound != -1` every tick and calls `SpawnDetached` which maintains
   the sound, adjusting volume/pan based on distance, stopping if too far.

2. **On Middle (delay expires):** `AnimClass::Middle` (`0x00424CE0`) plays
   `StartSound` once when the animation's delay countdown hits zero or
   transitions to a `Next=` animation.

3. **On Cleanup:** When anim is destroyed, plays `StopSound` if set.

**Explosion sounds come from animations, not warheads.** The weapon's `Anim=` key
determines which explosion anim plays, and that anim's `StartSound=`/`Report=`
provides the explosion sound.

### 6.6 Building Fire Sounds

When a building is damaged below `ConditionYellow`:
1. `BuildingClass::CreateDamageFireAnims` (`0x0043C0D0`) creates fire/smoke anims
2. Fire anim types come from `RulesClass->BuildingDamageFireAnims`
3. Each fire anim has its own `StartSound=` in art.ini → continuous fire crackle
4. `BuildingDamageSound=` in `[AudioVisual]` is a **separate** one-shot sound for
   when a building first transitions to damaged state

### 6.7 Looping Sound Mechanism

At the DirectSound level, looping is controlled by VocClass control flag 0x01 (LOOP):
- Sets `DSBPLAY_LOOPING` in `SoundEvent__StartPlayback`
- When buffer finishes, `SoundEvent__AdvancePlaylist` either repeats (if loop count
  not exceeded) or stops

The `VocClass__PlayAtPos` loop handle (third parameter) is a higher-level mechanism:
- Stores the SoundEvent pointer so the caller can track/update it
- Prevents duplicate overlapping playback of the same looping sound
- Allows replacing one looping sound with another (e.g., building changes ambient sound)

---

## 7. Global Sound Definitions

### 7.1 `[AudioVisual]` Section

`RulesClass__ReadAudioVisual` (`0x006691e0`) reads **74 individual sound entries** +
**3 sound lists** from `[AudioVisual]`. Each is resolved via `VocClass__FindByName`
and stored as a VocClass index (-1 = no sound).

#### GUI Sounds
| INI Key | Default | Purpose |
|---------|---------|---------|
| `GUIMainButtonSound` | MenuClick | Main menu button click |
| `GUIBuildSound` | MenuClick | Sidebar build click |
| `GUITabSound` | MenuTab | Sidebar tab switch |
| `GUIOpenSound` | MenuACBOpen | Panel open |
| `GUICloseSound` | MenuACBClose | Panel close |
| `GUIMoveOutSound` | MenuSlideOut | Slide out |
| `GUIMoveInSound` | MenuSlideIn | Slide in |
| `GenericClick` | MenuClick | General click |
| `GenericBeep` | GenericBeep | General beep |
| `ScoldSound` | MenuScold | Invalid action |

#### Building Sounds
| INI Key | Default | Purpose |
|---------|---------|---------|
| `BuildingDieSound` | BuildingGenericDie | Building destroyed |
| `BuildingSlam` | PlaceBuilding | Building placed |
| `BuildingDamageSound` | BuildingDamaged | Building enters damaged state |
| `BuildingDrop` | PlaceBuilding | Building dropped |
| `BuildingGarrisonedSound` | BuildingGarrisoned | Infantry enters |
| `BuildingRepairedSound` | BuildingRepaired | Repair tick |
| `Construction` | Dummy | Construction sound |

#### Combat / Special
| INI Key | Default | Purpose |
|---------|---------|---------|
| `BombTickingSound` | CrazyIvanBombTick | Ivan bomb timer |
| `BombAttachSound` | CrazyIvanAttack | Ivan bomb placed |
| `YuriMindControlSound` | YuriMindControl | Mind control beam |
| `MindClearedSound` | MindCleared | Mind control broken |
| `TeslaCharge` | TeslaCoilPowerUp | Tesla charging |
| `CloakSound` | NavalUnitEmerge | Cloak/uncloak |
| `BaseUnderAttackSound` | BaseUnderAttackSiren | Base attack alert |

#### Credit Tick Sounds (Sound List)
`CreditTicks=` is a comma-separated list stored as a `DynamicVectorClass`. When the
credits counter ticks up/down in `CreditsClass::Draw` (`0x004a2480`), sounds from this
list play in sequence, creating the iconic credit-counting effect.

---

## 8. Sound Thread

A dedicated audio thread handles streaming buffer fills:

- Created: `SoundThread__Init` (`0x00407550`)
- Stack: 0x1000 (4KB)
- Priority: `THREAD_PRIORITY_ABOVE_NORMAL`
- Entry point: `0x00407680`
- Protected by `CRITICAL_SECTION` at `0x0087e7f8`

**Thread loop:**
1. Get high-resolution timestamp
2. Lock critical section
3. Iterate all active streaming channels:
   - If channel has "needs data" flag (bit 2 at offset 0x0C) → fill buffer from source
   - Track remaining bytes; clear flag when done
4. Unlock critical section
5. Wait on event object
6. Repeat until shutdown flag (`0x0087e770`) is set

---

## 9. Audio File Format (audio.idx / audio.bag)

### IDX Format

| Field | Size | Description |
|-------|------|-------------|
| Magic | 4    | File signature |
| Version | 4  | Format version |
| Entry count | 4 | Number of samples |

Each IDX entry (36 bytes):

| Field | Size | Description |
|-------|------|-------------|
| Name | 16   | Null-terminated, case-insensitive |
| Offset | 4  | Byte offset into .bag file |
| Size | 4    | Sample size in bytes |
| Sample rate | 4 | Usually 22050 |
| Flags | 4   | Bit 0=stereo, bit 2=16-bit, bit 3=IMA ADPCM |
| Chunk size | 4 | IMA ADPCM chunk size |

### Lookup

`AudioIndex__FindSample` (`0x004015c0`): **binary search** by name (case-insensitive).
Falls back to loose `.wav` file search if not found in bag.

### Compression

Two formats:
- **Raw PCM:** Direct playback (mono/stereo, 8/16-bit)
- **IMA ADPCM:** 4:1 compression, decoded via standard IMA ADPCM algorithm (`0x0040acd0`)

---

## 10. Complete Function Reference

### Core Playback
| Address | Function | Purpose |
|---------|----------|---------|
| `0x00750920` | `VocClass__PlayAtPos` | **THE** main SFX dispatcher (~100 call sites) |
| `0x00406670` | `VocClass__PlayGlobal` | Non-positional SFX (2 call sites: subtitles) |
| `0x00750AC0` | `VocClass__CalcVolumeAndPan` | Distance → volume + pan |
| `0x00752480` | `VoxClass__QueueVoice` | Queue voice/EVA for streaming playback |
| `0x00752700` | `VoxClass__PlayEVA` | EVA dispatch with duplicate check |
| `0x00752760` | `VoxClass__PlayNextQueued` | Dequeue and play next voice |

### SoundEvent Lifecycle
| Address | Function | Purpose |
|---------|----------|---------|
| `0x00405190` | `SoundEvent__AllocateFromPool` | Get free event from 200-slot pool |
| `0x004055C0` | `SoundEvent__UpdateState` | Per-frame state machine |
| `0x004048B0` | `SoundEvent__LoadSamples` | Load samples based on control flags |
| `0x00404700` | `SoundEvent__PreparePlayout` | Build playlist |
| `0x004047B0` | `SoundEvent__AdvancePlaylist` | Next sample in playlist |
| `0x00404BB0` | `SoundEvent__SelectNextSample` | Random/sequential selection |
| `0x004054A0` | `SoundEvent__StartPlayback` | Lock buffer, set vol/pan, call DS Play |
| `0x004052F0` | `SoundEvent__Stop` | Stop event, release buffer |

### Channel Management
| Address | Function | Purpose |
|---------|----------|---------|
| `0x004041D0` | `SoundSystem__UpdateTick` | Main per-frame update + eviction |
| `0x004035F0` | `DSoundChannel__FindAvailable` | Find idle or evictable channel |
| `0x00404E20` | `DSoundChannel__FindLowestPriority` | Find eviction candidate |
| `0x00403530` | `DSoundChannel__CreateAll` | Create 16 DS buffers |
| `0x00404E70` | `SoundSystem__StopAll` | Stop all sounds |

### INI Parsing
| Address | Function | Purpose |
|---------|----------|---------|
| `0x00750440` | `VocClass__ReadINI` | Parse soundmd.ini entry |
| `0x007510D0` | `VocClass__ReadSoundListINI` | Parse [AudioVisual] defaults + all sounds |
| `0x007514D0` | `VocClass__FindByName` | Name → VocClass index lookup |
| `0x004064A0` | `VocClass__AddSample` | Add .aud sample to VocClass (strips $# prefixes) |
| `0x00753000` | `VoxClass__ReadEVAINI` | Parse EVAMD.INI [DialogList] |

### Infrastructure
| Address | Function | Purpose |
|---------|----------|---------|
| `0x00406B10` | `AudioSystem__Init` | Master init |
| `0x00406D40` | `AudioSystem__Shutdown` | Master shutdown |
| `0x00407550` | `SoundThread__Init` | Spawn audio thread |
| `0x00403ED0` | `SoundEventPool__Init` | Create 200-slot pool |
| `0x004011C0` | `AudioIndex__Constructor` | Load audio.idx, open audio.bag |
| `0x004015C0` | `AudioIndex__FindSample` | Binary search in idx |
| `0x00407860` | `StreamPlayer__Create` | Create streaming player |

---

## 11. Design Summary

### What Makes It Sound Right

1. **On-screen = full volume.** Viewport-edge subtraction means anything visible is heard
   at max. Distance only accumulates off-screen. This is why the game sounds "present."

2. **Linear falloff, not inverse-square.** Simple `(range - dist) / range`. Sounds don't
   vanish abruptly — they fade predictably over the Range= distance.

3. **5% threshold.** Sounds below 5% volume are culled entirely. No ghost whispers of
   distant battles consuming channels.

4. **Per-sound instance limiting (Limit=5).** A rapid-fire weapon can't consume all 16
   channels. At most 5 instances of the same sound play simultaneously.

5. **Priority eviction with age tie-breaking.** When channels are full, the oldest
   lowest-priority sound dies. 0x665-tick minimum age prevents thrashing.

6. **Smooth interpolation.** Volume, pan, and pitch are ramped — never snapped. No clicks.

7. **Voice interruption.** Clicking a new unit immediately cuts the previous voice
   (priority 2 = flush queue). Feels responsive.

8. **EVA deduplication.** Same EVA message can't stack in the queue. "Unit ready" can
   only appear once at a time. STANDARD type events are fire-and-forget — if the
   channel is busy, they're silently dropped rather than queued.

9. **500ms inter-announcement gap.** EVA messages don't run into each other. There's
   always a beat between announcements.

10. **Separate channels for SFX vs Voice/EVA.** The 16 DirectSound buffers are for SFX
    only. Voices and EVA never compete with weapon/explosion sounds for channels.
    Only one voice/EVA plays at a time via the dedicated StreamPlayer.

11. **Explosion sounds come from animations, not warheads.** The visual explosion anim
    carries the sound. This is why different weapons with the same warhead can have
    different explosion sounds — it's the `Anim=` that matters.

12. **SHROUD gating.** Sounds with the SHROUD type flag are silenced if their source
    cell hasn't been explored. You can't hear enemy activities in fog.

12. **SHROUD gating.** Sounds with the SHROUD type flag are silenced if their source
    cell hasn't been explored. You can't hear enemy activities in fog.

---

**Part II of this report (gap analysis, bugs, action plan) is in
[SOUND_IMPLEMENTATION_GAPS.md](SOUND_IMPLEMENTATION_GAPS.md).**
