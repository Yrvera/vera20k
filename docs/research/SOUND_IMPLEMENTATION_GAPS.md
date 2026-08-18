# Sound System — Implementation Gaps & Action Plan

**Date:** 2026-03-27
**Companion to:** [SOUND_SYSTEM_IMPLEMENTATION_REPORT.md](SOUND_SYSTEM_IMPLEMENTATION_REPORT.md)
**Purpose:** Every gap between the original gamemd.exe sound system and our Rust engine,
organized for implementation.

---

## 1. What Works Today

| Feature | File | Notes |
|---------|------|-------|
| 16-channel SFX pool | `audio/sfx.rs` | Matches original hard limit |
| Dedicated voice slot (cuts previous) | `audio/sfx.rs` | Immediate `.stop()`, no fade-out on interrupted voice |
| Spatial volume (linear falloff, iso Y x2, viewport-edge sub, 5% cutoff) | `audio/sfx.rs` | Faithful reproduction of `CalcVolumeAndPan` |
| audio.bag / audio.idx loading + binary search | `assets/audio_bag.rs` | Matches original |
| IMA ADPCM decoding (mono + stereo) | `audio/sfx.rs` | RFC-compliant, standard tables |
| .WAV decoding (8/16-bit, mono/stereo) | `audio/sfx.rs` | Works |
| sound.ini / soundmd.ini parsing (Volume, Priority, Range, MinVolume) | `rules/sound_ini.rs` | `$` and `#` prefix stripping handled |
| eva.ini / evamd.ini parsing with faction lookup | `rules/sound_ini.rs` | Allied/Soviet/Yuri variants |
| WeaponFired (Report= spatial SFX) | `sim/combat` → `app_sim_tick` | Works |
| EntityDestroyed (DieSound= spatial SFX) | `sim/combat` → `app_sim_tick` | Works |
| UnitSelected / UnitMoveOrder / UnitAttackOrder | `app_input.rs` | Voice slot |
| BuildingReady / UnitReady EVA events | `app_sim_tick.rs` | Events created, **playback temporarily disabled** |
| Random sound selection from Sounds= list | `audio/sfx.rs` | Counter-based (deterministic, acceptable) |
| Volume scale: INI 0–100 → rodio 0.0–1.0 | `audio/sfx.rs` | Correct 3-way multiply: (sound/100 * master * spatial) |
| 3ms fade-in/fade-out on all sounds | `audio/sfx.rs` | Prevents click artifacts |

---

## 2. Bugs in Current Implementation

### BUG 1: MinVolume applied unconditionally

**File:** `audio/sfx.rs` line ~91
**Problem:** `calc_spatial_volume()` always applies the MinVolume floor. The original
engine only applies MinVolume when the sound has the **GLOBAL type flag (0x10)**.
**Effect:** All sounds have a volume floor, making distant sounds too loud.
**Fix:** The caller must check the Type flag before passing `min_volume_pct`.
Since we don't parse Type flags yet, currently ALL sounds get the floor treatment.

### BUG 2: Priority parsed but never used for eviction

**File:** `audio/sfx.rs` lines 301–308
**Problem:** Eviction is pure FIFO — `active.pop_front()` always removes the oldest
sound. The Priority field from sound.ini is stored in `SoundEntry` but completely
ignored during channel management.
**Effect:** A CRITICAL weapon sound can be evicted by a LOWEST ambient sound simply
because the weapon started first. The original engine evicts lowest-priority first,
with age as tie-breaker.

### BUG 3: No per-sound instance limiting

**File:** `audio/sfx.rs`
**Problem:** No check against `Limit=` field (default 5). A rapid-fire weapon can
spawn unlimited concurrent instances of the same sound, consuming all 16 channels.
**Effect:** Gatling guns or Prism tanks can monopolize the entire SFX pool.

### BUG 4: Only DieSound plays on death — no VoiceDie

**File:** `sim/combat/mod.rs` line 320–322
**Problem:** Only checks `obj.die_sound` (DieSound= field). The original engine plays
TWO sounds on death:
1. `VoiceDie=` → voice channel (death scream)
2. `DieSound=` → SFX channel (explosion/impact)
**Effect:** Units die silently without voice screams.

### BUG 5: DownReport never checked

**File:** `sim/combat/mod.rs` line 704
**Problem:** Always uses `weapon.report` regardless of firing angle. The field
`weapon.down_report` IS parsed in `rules/weapon_type.rs` but never read in combat.
**Effect:** Infantry/buildings firing downward use the wrong sound.

### BUG 6: No directional report selection (8-facing)

**File:** `sim/combat/mod.rs` line 704
**Problem:** Always uses `weapon.report[0]`. The original engine, when a weapon has
exactly 8 Report entries, selects by firer's facing direction (N=0, NE=1, ..., NW=7).
**Effect:** Weapons with directional sounds always play the north-facing sound.

---

## 3. SFX Player Gaps (`audio/sfx.rs`)

### 3.1 No Stereo Pan

**Original:** Pan computed as 0–16384 (0=left, 8192=center, 16384=right) based on
horizontal screen offset. Formula:
```
pan = 8192 + (-offsetX * 8192 / viewportWidth)
```
**Current:** Mono volume only. All sounds come from center.
**Impact:** Sounds lack spatial width. Explosions on the left sound identical to right.
**Fix:** Scale left/right channels separately. With rodio, apply a channel-volume
source wrapper after decode.

### 3.2 No Pitch Variation (VShift)

**Original:** `VShift=` (0–100) applies random pitch shift per play instance, making
repeated sounds feel varied.
**Current:** Not parsed, not applied. All playback at native sample rate.
**Impact:** Repeated weapon fire sounds mechanical and monotonous.

### 3.3 No Pre-Delay (Delay field)

**Original:** `Delay= min max` — random delay in ms before playback starts. Used with
PREDELAY control flag.
**Current:** Not parsed. All sounds start immediately.
**Impact:** Missing timing variation for ambient/environmental sounds.

### 3.4 No Frequency Shift (FShift)

**Original:** `FShift= min max` — random frequency shift per play.
**Current:** Not parsed, not applied.

### 3.5 No Attack/Decay Envelopes

**Original:** `Attack=` and `Decay=` (ms) with ATTACK/DECAY control flags. First sample
in Sounds= is the attack sample, last is the decay sample.
**Current:** Not parsed. The global 3ms fade is not the same thing.
**Impact:** Sounds that should ramp in/out (engine startup, ambient wind) start/stop
abruptly.

### 3.6 No Loop Handle System

**Original:** `VocClass__PlayAtPos` third parameter is a loop handle. If the same sound
is already playing at that handle, it continues. If a different sound, the old one stops
and the new one starts. This is how ambient/working sounds persist across ticks.
**Current:** Pure fire-and-forget. No way to track, update, or manage persistent sounds.
**Impact:** Cannot implement ambient building sounds, working sounds, or any continuous
positional audio.

### 3.7 No Control Flag Processing

**Original:** LOOP, RANDOM, ALL, PREDELAY, INTERRUPT, ATTACK, DECAY, AMBIENT flags
determine playback behavior.
**Current:** Not parsed. Only RANDOM is approximated via counter-based selection.
**Impact:** No looping sounds (LOOP), no layered sounds (ALL), no envelope sounds
(ATTACK/DECAY).

**Key interaction:** When LOOP is set, `SoundEvent__AdvancePlaylist` repeats the playlist.
When ALL is set, ALL samples from Sounds= play simultaneously (layered). These are
fundamentally different playback modes.

### 3.8 No Volume Interpolation

**Original:** Volume, pan, and pitch are smoothly interpolated (ramped) over time via
per-channel interpolator structs. Changes are never snapped.
**Current:** Volume set directly with `player.set_volume()`. No ramping.
**Impact:** Volume transitions (spatial fading, ambient changes) may click or feel
abrupt.

---

## 4. Missing INI Fields on ObjectType (`rules/object_type.rs`)

### Currently parsed (5 of 35):

| Field | Parsed | Used in Code |
|-------|--------|-------------|
| `VoiceSelect=` | Yes | Yes — `app_input.rs` |
| `VoiceMove=` | Yes | Yes — `app_input.rs` |
| `VoiceAttack=` | Yes | Yes — `app_input.rs` |
| `DieSound=` | Yes | Yes — `sim/combat` |
| `MoveSound=` | Yes | **Never used** — parsed but no emission site |

### Missing voice fields (13):

| INI Key | Original Offset | When Triggered |
|---------|----------------|----------------|
| `VoiceSelectEnslaved=` | 0x430 | Selected while mind-controlled |
| `VoiceSelectDeactivated=` | 0x44C | Selected while EMP'd |
| `VoiceSpecialAttack=` | 0x4A0 | Special weapon attack |
| `VoiceDie=` | 0x4BC | Death scream (voice channel) |
| `VoiceFeedback=` | 0x4D8 | General feedback |
| `VoiceHarvest=` | — | Harvester delivers ore |
| `VoiceCapture=` | — | Engineer captures building |
| `VoiceEnter=` | — | Enters transport/building |
| `VoiceSinking=` | — | Ship sinking |
| `VoiceCrashing=` | — | Aircraft crashing |
| `VoiceFalling=` | — | Paradropping / falling |
| `VoiceDeploy=` | — | MCV/unit deploys |
| `VoiceUndeploy=` | — | Undeploys |

### Missing sound fields (17):

| INI Key | When Triggered |
|---------|----------------|
| `DeploySound=` | Unit deploys (SFX, not voice) |
| `UndeploySound=` | Unit undeploys |
| `CreateSound=` | Unit/building created |
| `EnterTransportSound=` | Enters transport |
| `LeaveTransportSound=` | Exits transport |
| `TurretRotateSound=` | Turret rotation tick |
| `DamageSound=` | Takes damage |
| `CrashingSound=` | Aircraft crash |
| `AuxSound1=` | Auxiliary sound 1 |
| `AuxSound2=` | Auxiliary sound 2 |
| `AmbientSound=` | Continuous ambient loop |
| `CrushSound=` | Crushes infantry (on ObjectType) |
| `WorkingSound=` | Building powered + active |
| `NotWorkingSound=` | Building unpowered / idle |
| `BuildupSound=` | Building construction anim |
| `EnterWaterSound=` | Vehicle enters water |
| `LeaveWaterSound=` | Vehicle exits water |

---

## 5. Missing Sound INI Fields (`rules/sound_ini.rs`)

### Currently parsed on SoundEntry (6 of 13):

| Field | Parsed | Used |
|-------|--------|------|
| `Sounds=` | Yes | Yes |
| `Volume=` | Yes | Yes |
| `Priority=` | Yes | **Stored but ignored** (BUG 2) |
| `Range=` | Yes | Yes |
| `MinVolume=` | Yes | Yes (but BUG 1) |
| `Limit=` | **No** | — |

### Missing fields (7):

| INI Key | Type | Default | Purpose |
|---------|------|---------|---------|
| `Control=` | bitmask | 0 | LOOP, RANDOM, ALL, PREDELAY, INTERRUPT, ATTACK, DECAY, AMBIENT |
| `Type=` | bitmask | SCREEN (0x20) | VIOLENT, MOVEMENT, SCREEN, GLOBAL, LOCAL, SHROUD, etc. |
| `Loop=` | int | 0 | Loop count (0 = infinite when LOOP flag set) |
| `Delay=` | int int | 0 0 | Random pre-delay range min, max (ms) |
| `FShift=` | int int | 0 0 | Frequency shift range min, max |
| `VShift=` | int | 0 | Pitch variation (0–100) |
| `Attack=` / `Decay=` | int | 0 | Envelope ramp times (ms) |

---

## 6. Missing Event Types

### Sim Events (`sim/world/mod.rs` — SimSoundEvent)

Currently 5 variants. Missing:

| Event | Emission Site | Original Trigger |
|-------|--------------|-----------------|
| `MoveSoundTick` | `movement/movement_step.rs` on cell entry | `DriveLocomotionClass::ProcessMovement` |
| `CrushKill` | `movement/bump_crush.rs` on crush | `DriveLocomotionClass::ProcessMovement` |
| `AmbientLoop` | `sim/game_entity.rs` AI tick | `TechnoClass::AI_Update` |
| `DeploySound` | deploy state machine | `TechnoClass::Deploy` |
| `UndeploySound` | undeploy state machine | `TechnoClass::Undeploy` |
| `PromotionGained` | veterancy transition | `TechnoClass::GainVeterancy` |
| `BuildingDamaged` | damage handler | `TechnoClass::ReceiveDamage` |
| `BuildingSold` | sell handler | `BuildingClass::Sell` |
| `BuildingPlaced` | placement handler | `BuildingClass::Place` |
| `TeleportWarpOut` | `movement/teleport_movement.rs:174` | Chrono warp-out phase |
| `TeleportWarpIn` | `movement/teleport_movement.rs:189` | Chrono warp-in phase |
| `TunnelDigIn` | `movement/tunnel_movement.rs:197` | Tunnel burrow start |
| `TunnelDigOut` | `movement/tunnel_movement.rs:249` | Tunnel emerge |
| `DropPodLand` | `movement/droppod_movement.rs:~104` | Drop pod impact |
| `GarrisonEnter` | `sim/passenger.rs` | Infantry enters building |
| `GarrisonExit` | `sim/passenger.rs` | Last occupant leaves |
| `EnterTransport` | transport loading | Unit boards transport |
| `ExitTransport` | transport unloading | Unit exits transport |
| `VoiceDie` | `sim/combat/mod.rs:320` | Death scream (separate from DieSound) |
| `WaterTransition` | movement on amphibious transition | Enter/leave water |

### App Events (`audio/events.rs` — GameSoundEvent)

Currently 8 variants. Need to add generalized variants rather than one per trigger:

| New Variant | Purpose |
|-------------|---------|
| `PositionalSound { sound_id, screen_pos }` | Generic spatial SFX — replaces per-type spatial events |
| `LoopingSound { entity_id, sound_id, screen_pos }` | Continuous sound tied to entity |
| `StopLoop { entity_id }` | Stop an entity's looping sound |
| `EvaEvent { event_name }` | Route to EVA system (not SFX player) |

---

## 7. Missing EVA System

No EVA infrastructure exists beyond the `EvaRegistry` parser. The entire playback
and queue system needs to be built.

### What's needed (`audio/eva.rs` — new file):

| Component | Purpose |
|-----------|---------|
| `EvaSystem` struct | Queue, current playback, timing |
| Priority queue (VecDeque) | 4 levels, dequeue highest first |
| Event type dispatch | STANDARD (drop if busy), QUEUE, INTERRUPT, QUEUED_INTERRUPT |
| Duplicate suppression | Same event can't stack in queue |
| 500ms inter-announcement gap | `Instant`-based timer |
| Suspend/pause counters | Block EVA during movies/cutscenes |
| `tick()` method | Called each frame to advance queue |
| Faction-aware playback | Allied/Soviet/Yuri sound selection |

### Missing EVA trigger sites:

| EVA Event | Where to Emit | Priority |
|-----------|--------------|----------|
| `EVA_ConstructionComplete` | `production_queue.rs` | HIGH — already emitted, playback disabled |
| `EVA_UnitReady` | `production_queue.rs` | HIGH — already emitted, playback disabled |
| `EVA_BaseUnderAttack` | damage handler when player building hit | HIGH |
| `EVA_UnitLost` | `combat/mod.rs` when player unit dies | MEDIUM |
| `EVA_BuildingLost` | `combat/mod.rs` when player building dies | MEDIUM |
| `EVA_LowPower` | `power_system.rs` on power transition | HIGH |
| `EVA_SilosNeeded` | ore storage full detection | MEDIUM |
| `EVA_BuildingInfiltrated` | spy infiltration | HIGH |
| `EVA_AlliedBuildingInfiltrated` | spy infiltration (ally) | MEDIUM |
| `EVA_OreRunningLow` | ore field depletion | LOW |
| `EVA_NuclearSiloDetected` | superweapon detected | HIGH |
| `EVA_IronCurtainReady` | superweapon ready | MEDIUM |
| `EVA_ChronoSphereReady` | superweapon ready | MEDIUM |
| `EVA_AllyUnderAttack` | ally building hit | LOW |

---

## 8. Missing Global Sound Definitions (`[AudioVisual]`)

The original engine reads **74 individual sound entries** + **3 sound lists** from
`[AudioVisual]` in rules.ini. We parse **zero** of them.

### Tier 1 — Critical (affect gameplay feel)

| INI Key | Default | When Played |
|---------|---------|-------------|
| `BuildingSlam=` | PlaceBuilding | Building placed on map |
| `SellSound=` | SellBuilding | Building sold |
| `ScoldSound=` | MenuScold | Invalid action (can't build here, insufficient funds) |
| `BuildingDamageSound=` | BuildingDamaged | Building transitions to damaged state |
| `BuildingDieSound=` | BuildingGenericDie | Building destroyed (fallback if no per-type DieSound) |
| `BaseUnderAttackSound=` | BaseUnderAttackSiren | Base attack alert SFX (not EVA) |
| `GenericClick=` | MenuClick | Any click without specific sound |

### Tier 2 — Important (common events)

| INI Key | Default | When Played |
|---------|---------|-------------|
| `GUIMainButtonSound=` | MenuClick | Main menu button |
| `GUIBuildSound=` | MenuClick | Sidebar build click |
| `GUITabSound=` | MenuTab | Sidebar tab switch |
| `GUIOpenSound=` | MenuACBOpen | Panel open |
| `GUICloseSound=` | MenuACBClose | Panel close |
| `CloakSound=` | NavalUnitEmerge | Unit cloaks/uncloaks |
| `ChronoInSound=` | ChronoMinerTeleport | Chrono arrival |
| `ChronoOutSound=` | ChronoMinerTeleport | Chrono departure |
| `UpgradeVeteranSound=` | UpgradeVeteran | Unit reaches veteran |
| `UpgradeEliteSound=` | UpgradeElite | Unit reaches elite |
| `BombTickingSound=` | CrazyIvanBombTick | Ivan bomb timer ticking |
| `BombAttachSound=` | CrazyIvanAttack | Ivan bomb placed |
| `YuriMindControlSound=` | YuriMindControl | Mind control beam |
| `MindClearedSound=` | MindCleared | Mind control broken |
| `TeslaCharge=` | TeslaCoilPowerUp | Tesla coil charging |
| `BuildingGarrisonedSound=` | BuildingGarrisoned | Infantry enters garrison |
| `BuildingRepairedSound=` | BuildingRepaired | Repair tick |

### Tier 3 — Nice to have

| INI Key | Default | When Played |
|---------|---------|-------------|
| `CreditTicks=` | (list) | Credit counter tick |
| `CrateMoneySound=` | CrateMoney | Money crate collected |
| `CrateRevealSound=` | CrateReveal | Reveal crate collected |
| `CrateFireSound=` | CrateFirePower | Firepower crate |
| `CrateArmourSound=` | CrateArmor | Armor crate |
| `CrateSpeedSound=` | CrateSpeed | Speed crate |
| `CrateUnitSound=` | CrateFreeUnit | Unit crate |
| `CratePromoteSound=` | CratePromoted | Promotion crate |
| `RadarOn=` | RadarOn | Radar activated |
| `RadarOff=` | RadarOff | Radar deactivated |
| `ImpactWaterSound=` | ExplosionWaterLarge | Projectile hits water |
| `SinkingSound=` | GenLargeWaterDie | Ship sinking |
| `ChuteSound=` | ParachuteDrop | Parachute deployed |
| `DigSound=` | NukeSiren | Tunnel locomotion |
| `StormSound=` | WeatherIntro | Lightning storm |
| All deploy/slave/bunker/grinder/bioreactor/spy sounds | various | YR-specific mechanics |

---

## 9. Missing Sound Emission Sites

Places in the code where game events happen but no sound is produced.

### 9.1 Sidebar / UI (`app_input.rs`)

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Build tab click | `apply_sidebar_action()` | `GUIBuildSound` |
| Tab switch | `apply_sidebar_action()` | `GUITabSound` |
| Invalid placement | placement validation | `ScoldSound` |
| Pause/unpause queue | sidebar action | click sound |
| Cancel build | sidebar action | click sound |
| Sell (Delete key) | sidebar action | `SellSound` |

### 9.2 Movement (`sim/movement/`)

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Unit enters new cell | `movement_step.rs` | `MoveSound=` (one-shot, not loop) |
| Vehicle crushes infantry | `bump_crush.rs` | `CrushSound=` |
| Amphibious enters water | movement layer transition | `EnterWaterSound` |
| Amphibious exits water | movement layer transition | `LeaveWaterSound` |
| Chrono warp-out | `teleport_movement.rs:174` | `ChronoOutSound` |
| Chrono warp-in | `teleport_movement.rs:189` | `ChronoInSound` |
| Tunnel burrow start | `tunnel_movement.rs:197` | `DigSound` |
| Tunnel emerge | `tunnel_movement.rs:249` | `DigSound` |
| Drop pod impact | `droppod_movement.rs:~104` | explosion sound |

### 9.3 Combat (`sim/combat/mod.rs`)

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Unit death | line 320 | `VoiceDie=` (voice channel, in addition to DieSound) |
| Downward fire | line 704 | `DownReport=` instead of `Report=` |
| Directional fire (8-way) | line 704 | Select Report by facing index |
| Elite weapon fire | line 704 | Elite weapon's Report instead of regular |
| Garrison fire | line 704 | Garrison-specific fire sound |

### 9.4 Production (`sim/production/`)

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Building placed | `production_placement.rs:224` | `BuildingSlam` / `BuildingDrop` |
| Building sold | `production_sell.rs:207` | `SellSound` |
| Unit spawned (vehicle) | `production_spawn.rs` | `CreateUnitSound` |
| Unit spawned (infantry) | `production_spawn.rs` | `CreateInfantrySound` |
| Unit spawned (aircraft) | `production_spawn.rs` | `CreateAircraftSound` |

### 9.5 Building State

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Power goes low | `power_system.rs:146` | EVA `EVA_LowPower` |
| Power restored | `power_system.rs:155` | EVA (implicit, or power-up SFX) |
| Building ambient (continuous) | entity AI tick | `AmbientSound=` loop |
| Building working (powered) | entity AI tick | `WorkingSound=` loop |
| Building not working (unpowered) | entity AI tick | `NotWorkingSound=` loop |
| Building enters damaged state | damage handler | `BuildingDamageSound` |
| Garrison entered | `sim/passenger.rs` | `BuildingGarrisonedSound` |
| Garrison emptied | `sim/passenger.rs` | `BuildingAbandonedSound` |
| Deploy (MCV etc.) | deploy handler | `DeploySound=` |
| Undeploy | undeploy handler | `UndeploySound=` |
| Radar activated | radar system | `RadarOn` |
| Radar deactivated | radar system | `RadarOff` |

### 9.6 Promotions

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Veteran gained | veterancy transition | `UpgradeVeteranSound` |
| Elite gained | veterancy transition | `UpgradeEliteSound` |

### 9.7 Animation Sounds (CRITICAL architectural gap)

| Action | Location | Sound Needed |
|--------|----------|-------------|
| Damage fire anim spawned | `app_building_anim.rs` | Fire anim's `StartSound=` (continuous) |
| Damage fire anim ticking | `app_building_anim.rs` | Fire anim's `StartSound=` (looping) |
| Crane/buildup anim playing | `app_building_anim.rs` | Buildup anim's `StartSound=` |
| Explosion anim spawned | (not yet in codebase) | Explosion anim's `StartSound=` |
| Anim chain transition | (not yet in codebase) | `StartSound=` on Next= anim |
| Anim cleanup | (not yet in codebase) | `StopSound=` |

**This is the biggest architectural gap.** The original engine's `AnimClass::AI` plays
a continuous looping `StartSound=` every tick via `SpawnDetached`. Our engine advances
animation frames but has zero sound hooks in the animation system.

---

## 10. Architectural Gap: Event-Driven vs State-Based Sounds

The current sound system is **event-driven** — things happen once and a sound is queued.
But the original engine has a large category of **state-based** sounds that must be
**maintained continuously**:

| Sound Type | Original Behavior | Required Architecture |
|------------|-------------------|----------------------|
| `AmbientSound=` | Re-triggered every tick if entity alive | Loop handle + per-entity tracking |
| `WorkingSound=` | Plays while building powered + functional | Loop handle + power state check |
| `NotWorkingSound=` | Plays while building unpowered | Loop handle + power state check |
| Fire anim `StartSound=` | Continuous while fire overlay exists | Loop handle + animation tracking |
| `TurretRotateSound=` | Plays while turret rotating | Loop handle + rotation state |
| `BombTickingSound=` | Continuous while bomb attached | Loop handle + bomb entity tracking |

### What a loop handle system needs:

```
Per entity: HashMap<u64, LoopState>
  LoopState {
      current_sound_id: String,
      player_index: usize,  // into SFX pool (must be stable, not FIFO)
      last_screen_pos: (f32, f32),
  }

Each frame:
  1. For each entity with a looping sound:
     a. If entity dead → stop sound, remove entry
     b. If sound_id changed → stop old, start new
     c. If same sound_id → update volume/pan from new screen position
  2. For entities that LOST their looping sound (e.g., unpowered):
     → stop sound, remove entry
```

This requires SFX pool entries to be **addressable** (not just fire-and-forget).
The current `VecDeque<Player>` must become something with stable indices.

---

## 11. Implementation Priorities

### Phase 1: Fix Core SFX Engine Bugs

| Task | File | Effort |
|------|------|--------|
| Priority-based eviction | `audio/sfx.rs` | Store `(priority, start_time, sound_id)` per player. Evict lowest-priority oldest. |
| Per-sound instance limit | `audio/sfx.rs` | `HashMap<String, usize>` tracking counts. Check `Limit` before play. |
| MinVolume conditional on GLOBAL | `audio/sfx.rs` | Pass type_flags to `calc_spatial_volume`, only apply floor when flag 0x10. |
| Parse `Limit=` field | `rules/sound_ini.rs` | Add to `SoundEntry`, default 5. |
| Parse `Control=` field | `rules/sound_ini.rs` | Bitmask with LOOP/RANDOM/ALL/etc. flag table. |
| Parse `Type=` field | `rules/sound_ini.rs` | Bitmask with SCREEN/GLOBAL/LOCAL/SHROUD/etc. flag table. |
| Parse `VShift=`, `Delay=`, `FShift=`, `Loop=` | `rules/sound_ini.rs` | Add to `SoundEntry`. |

### Phase 2: Stereo Pan

| Task | File | Effort |
|------|------|--------|
| Compute pan from screen X offset | `audio/sfx.rs` | `pan = 0.5 + (-offsetX / viewportWidth * 0.5)` |
| Apply pan as L/R channel scaling | `audio/sfx.rs` | Wrap decoded audio in a pan source. |
| Pass screen_pos through to play calls | `app_building_anim.rs` | Currently only volume is passed, need pan too. |

### Phase 3: Loop Handle System

| Task | File | Effort |
|------|------|--------|
| Replace `VecDeque<Player>` with stable-index pool | `audio/sfx.rs` | Slab/generational-index or `Vec<Option<PlayerSlot>>` |
| Add `LoopHandle` type | `audio/sfx.rs` | Maps entity_id → player slot |
| Add `play_or_update_loop()` method | `audio/sfx.rs` | Check handle, update vol/pan or start new |
| Add `stop_loop()` method | `audio/sfx.rs` | Stop by handle |

### Phase 4: Wire Missing Combat Sounds

| Task | File | Effort |
|------|------|--------|
| Emit VoiceDie on death | `sim/combat/mod.rs` | Add alongside DieSound emission |
| Check DownReport | `sim/combat/mod.rs` | Compare firer Z vs target Z |
| 8-directional report | `sim/combat/mod.rs` | If weapon.report.len()==8, select by facing |
| Add 13 missing voice fields to ObjectType | `rules/object_type.rs` | Parse from INI |
| Add 17 missing sound fields to ObjectType | `rules/object_type.rs` | Parse from INI |

### Phase 5: Wire Missing Movement Sounds

| Task | File | Effort |
|------|------|--------|
| Emit MoveSound on cell transition | `sim/movement/movement_step.rs` | One-shot per cell entry |
| Emit CrushSound | `sim/movement/bump_crush.rs` | On crush kill |
| Emit teleport warp sounds | `sim/movement/teleport_movement.rs` | At phase transitions |
| Emit tunnel dig sounds | `sim/movement/tunnel_movement.rs` | At phase transitions |

### Phase 6: EVA System

| Task | File | Effort |
|------|------|--------|
| Create `EvaSystem` struct | `audio/eva.rs` (new) | Queue, dedup, 500ms gap, suspend |
| Add `EvaEventType` enum | `audio/eva.rs` | Standard, Queue, Interrupt, QueuedInterrupt |
| Parse Type field from evamd.ini | `rules/sound_ini.rs` | Add to EvaRegistry entries |
| Re-enable EVA playback | `app_building_anim.rs` | Route EVA events to EvaSystem |
| Add EVA trigger sites | various sim files | BaseUnderAttack, UnitLost, LowPower, etc. |

### Phase 7: Global Sounds & UI

| Task | File | Effort |
|------|------|--------|
| Parse `[AudioVisual]` sounds | `rules/ruleset.rs` | GlobalSounds struct, 74 entries |
| Wire sidebar clicks | `app_input.rs` | Push UiSound events |
| Wire ScoldSound | `app_input.rs` | On invalid actions |
| Wire building placement slam | production placement | Push positional event |
| Wire sell sound | sell handler | Push event |

### Phase 8: Animation Sounds

| Task | File | Effort |
|------|------|--------|
| Add sound hooks to damage fire system | `app_building_anim.rs` | Loop handle per fire overlay |
| Parse StartSound/StopSound from art.ini AnimTypes | animation type parser | Read the fields |
| Connect anim frame advancement to sound emission | animation tick | Continuous looping sound |

---

## 12. Files to Create or Modify

| File | Action | What |
|------|--------|------|
| `src/audio/sfx.rs` | **Major rewrite** | Priority eviction, instance limiting, pan, loop handles, stable-index pool |
| `src/audio/eva.rs` | **Create** | EVA queue, dedup, 500ms gap, suspend, event types |
| `src/audio/events.rs` | **Modify** | Add PositionalSound, LoopingSound, StopLoop, EvaEvent |
| `src/audio/mod.rs` | **Modify** | Add `pub mod eva;` |
| `src/rules/sound_ini.rs` | **Modify** | Parse Control, Type, Loop, Delay, FShift, VShift, Attack, Decay, Limit |
| `src/rules/object_type.rs` | **Modify** | Add 30 missing voice/sound fields |
| `src/rules/ruleset.rs` | **Modify** | Add GlobalSounds struct, parse [AudioVisual] |
| `src/sim/world/mod.rs` | **Modify** | Add ~20 new SimSoundEvent variants |
| `src/sim/combat/mod.rs` | **Modify** | VoiceDie, DownReport, directional report |
| `src/sim/movement/movement_step.rs` | **Modify** | Emit MoveSound on cell entry |
| `src/sim/movement/bump_crush.rs` | **Modify** | Emit CrushSound |
| `src/sim/movement/teleport_movement.rs` | **Modify** | Emit warp sounds at phase transitions |
| `src/sim/movement/tunnel_movement.rs` | **Modify** | Emit dig sounds |
| `src/sim/production/production_placement.rs` | **Modify** | Emit BuildingSlam |
| `src/sim/production/production_sell.rs` | **Modify** | Emit SellSound |
| `src/sim/power_system.rs` | **Modify** | Emit LowPower EVA |
| `src/sim/passenger.rs` | **Modify** | Emit garrison sounds |
| `src/app_sim_tick.rs` | **Modify** | Convert new sim events, integrate EVA |
| `src/app_building_anim.rs` | **Modify** | Re-enable EVA, loop management, animation sounds |
| `src/app_input.rs` | **Modify** | Wire GUI/sidebar sounds, ScoldSound |
