# Animation Sounds System — Ghidra Reverse Engineering Report

**Date:** 2026-03-23
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH (verified from binary decompilation)

---

## 1. AnimTypeClass Sound Fields

**Function:** `AnimTypeClass::ReadINI` at `0x00427D00`
**Note:** `param_1` is `int*` — multiply field indices by 4 for byte offsets.

### Sound INI Keys Parsed

| INI Key       | Field Index | Byte Offset | Description |
|---------------|-------------|-------------|-------------|
| `StartSound=` | `0xBE`      | `0x2F8`     | VocClass index; played when anim starts (first frame). Fallback: if -1, tries `Report=` instead. |
| `Report=`     | `0xBE`      | `0x2F8`     | VocClass index; **same field** as StartSound — only read if StartSound was not found or returned -1. |
| `StopSound=`  | `0xBF`      | `0x2FC`     | VocClass index; played when anim is destroyed/cleaned up. |

### Key Parsing Logic (from ReadINI at 0x00428359)

```
// StartSound and Report share the same field at offset 0x2F8
// StartSound is tried first; if not found or -1, Report is tried
param_1[0xBE] = ReadStartSound();     // Try StartSound=
if (param_1[0xBE] == -1) {
    param_1[0xBE] = ReadReport();      // Fallback to Report=
}
param_1[0xBF] = ReadStopSound();       // StopSound= (separate field)
```

### Other AnimTypeClass Fields Referenced

| Field Index | Byte Offset | Purpose |
|-------------|-------------|---------|
| `0xAC`      | `0x2B0`     | Rate (converted: 900 / INI_Rate) |
| `0xAD`      | `0x2B4`     | Start frame |
| `0xAE`      | `0x2B8`     | LoopStart frame |
| `0xAF`      | `0x2BC`     | LoopEnd frame |
| `0xB0`      | `0x2C0`     | End frame (total frames) |
| `0xB1`      | `0x2C4`     | LoopCount |
| `0xB2`      | `0x2C8`     | Next (chained AnimTypeClass*) |
| `0xB3`      | `0x2CC`     | SpawnsParticle (ParticleSystemTypeClass*) |
| `0xB4`      | `0x2D0`     | NumParticles |
| `0xBC`      | `0x2F0`     | Spawns (child AnimTypeClass*) |
| `0xBD`      | `0x2F4`     | SpawnCount |
| `0xC0`      | `0x300`     | BounceAnim (AnimTypeClass*) |
| `0xC1`      | `0x304`     | ExpireAnim (AnimTypeClass*) |
| `0xC2`      | `0x308`     | TrailerAnim (AnimTypeClass*) |
| `0xC3`      | `0x30C`     | TrailerSeperation (int) |

---

## 2. AnimClass::AI — When Does an Anim Play Its Sound?

**Function:** `AnimClass::AI` at `0x00423AC0` (vtable offset 0x60)
**Function:** `AnimClass::Middle` at `0x00424CE0`
**Function:** `AnimClass::Start` at `0x00424F00`

### Sound Playback Timeline

#### A. Continuous Sound (every tick in AI)

At the **top of AnimClass::AI** (0x00423AC0), every tick:
```c
if (this->IsInvisible == false && this->Type->StartSound != -1) {
    // Get coords, then call AnimClass::SpawnDetached on sound handle at +0x1A0
    AnimClass__SpawnDetached(coords, &this->SoundHandle_0x1A0);
}
```
- **Address:** 0x00423AEA — checks `Type + 0x2F8 != -1`
- **Sound handle:** AnimClass offset `0x1A0`
- This is a **looping/continuous** sound — `AnimClass__UpdateLoopingSound` (0x00750D40) maintains the sound, adjusting volume/pan based on distance each tick, and stops it if too far away. (corrected 2026-05-29: was `SpawnDetached`; binary label confirmed via `get_function_by_address 0x00750D40` — RTTI_LABEL_DRIFT)

#### B. Sound on Middle (delay expires / anim chain transition)

`AnimClass::Middle` (0x00424CE0) is called when:
1. The initial delay countdown reaches zero (in AI, at 0x004243A1)
2. When transitioning to a `Next=` animation (at 0x00424925)
3. From the constructor if delay == 0

In Middle:
```c
// Virtual call at vtable+0x124 (ProcessCloakMode)
(*this->vtable[0x49])(2);

if (this->IsInvisible == false && this->Type->StartSound != -1) {
    // Play StartSound at anim's coordinates
    VocClass__PlayAt(this->Type->StartSound, &this->SoundHandle_0x1A0);  // was AnimClass__SpawnAtCoord; corrected 2026-05-29 — RTTI_LABEL_DRIFT
} else {
    // Stop any playing sounds
    SoundHandle__Release(&this->SoundHandle_0x1A0);
}
SoundHandle__Release(&this->SoundHandle_0x1B4);

if (this->Type->Start == 0) {
    AnimClass__Start();  // Calls Start if no start offset
}
```
- **Address:** 0x00424D01 — checks `Type + 0x2F8 != -1`
- `VocClass__PlayAt` (0x007509E0) initiates or restarts the sound (corrected 2026-05-29: was `AnimClass__SpawnAtCoord`; binary label confirmed via `get_function_by_address 0x007509E0` — RTTI_LABEL_DRIFT)

#### C. Sound on Cleanup/Destruction

In the AnimClass constructor cleanup path (when anim is being released):
```c
if (this->IsInvisible == false && this->Type != NULL && this->Type->StopSound != -1) {
    VocClass__PlayAt(this->Type->StopSound, &this->SoundHandle_0x1B4);  // was AnimClass__SpawnAtCoord; corrected 2026-05-29 — RTTI_LABEL_DRIFT
}
```
- This plays `StopSound=` when the anim finishes and is cleaned up.

### AnimClass Sound Handle Fields

| Offset | Size | Purpose |
|--------|------|---------|
| `0x1A0` | ~20 bytes | Sound handle for StartSound/Report (looping) |
| `0x1B4` | ~20 bytes | Sound handle for StopSound |

### Sound Playback Functions

| Address | Name | Purpose |
|---------|------|---------|
| `0x00750D40` | `AnimClass__UpdateLoopingSound` | Maintain continuous sound — adjusts volume/pan each tick, allocates from pool if needed (corrected 2026-05-29: was `AnimClass__SpawnDetached`; binary label is `AnimClass__UpdateLoopingSound` via `get_function_by_address 0x00750D40` — RTTI_LABEL_DRIFT) |
| `0x007509E0` | `VocClass__PlayAt` | Play/restart a one-shot sound by VocClass index at coordinates (corrected 2026-05-29: was `AnimClass__SpawnAtCoord`; binary label is `VocClass__PlayAt` via `get_function_by_address 0x007509E0` — RTTI_LABEL_DRIFT) |
| `0x00405D40` | `SoundHandle__Release` | Stop and release a sound handle |
| `0x00750E20` | `VocClass__PlayAtCoord` | General-purpose: play a VocClass index at given 3D coordinates (used by TechnoClass death) |

---

## 3. Building Damage Fire Animations

**Function:** `BuildingClass::CreateDamageFireAnims` at `0x0043C0D0`

### How It Works

When a building is damaged below the `ConditionYellow` threshold, this function creates fire/smoke animations at the building's damage point coordinates.

```c
int numFireAnims = Rules->NumBuildingDamageFireAnims;  // RulesClass + 0x2B0
int animIndex = Random(0, numFireAnims - 1);

for each damage point (offsets 0x15D8..0x1618, step 8) {
    AnimTypeClass* fireType = Rules->BuildingDamageFireAnims[animIndex];  // RulesClass + 0x2A4
    AnimClass* fire = new AnimClass(fireType, coords, 0, 1, 0x600, 0, 0);
    // Randomize start frame within the anim's total frames
    if (fire->Type->End > 0) {
        fire->CurrentFrame = Random(0, fire->Type->End - 1);
    }
    animIndex = (animIndex + 1) % numFireAnims;
}
```

### Sound Source

The fire/smoke anims get their sounds from their own `StartSound=`/`Report=` field in art.ini. There is no separate "fire crackle" sound in the building code — it comes entirely from the fire animation's art.ini entry.

The `BuildingDamageSound=` in `[AudioVisual]` (RulesClass + `0x714`) is a **separate** global sound for when a building first transitions to damaged state — it is NOT the continuous fire crackle.

### Relevant RulesClass Fields

| INI Key (in [AudioVisual]) | Offset (int*) | Byte Offset | Purpose |
|----------------------------|---------------|-------------|---------|
| `BuildingDamageFireAnims`  | `0xA9`        | `0x2A4`     | Pointer to array of fire AnimTypeClass* |
| (count)                    | `0xAC`        | `0x2B0`     | Number of fire anim types |
| `BuildingDamageSound=`     | `0x1C5`       | `0x714`     | Sound when building enters damaged state |

---

## 4. Death Animation Sounds (Infantry)

### How Infantry Death Works

When `TechnoClass::ReceiveDamage` (0x00701900) determines a unit is killed (Health reaches 0), the death handling occurs in a switch statement based on the damage result code.

#### Sound Playback on Death (case `default` at ~0x00702090)

Two separate sounds can play:

1. **VoiceDie** (TechnoTypeClass offset 0x4BC-0x4D8, sound list):
   ```c
   if (Type->VoiceDie_Count > 0) {  // offset 0x4CC
       int vocIndex = RandomFromSoundList(Type->VoiceDie);
       AnimClass__SpawnAtCoord(vocIndex, 0);  // One-shot at unit coords
   }
   ```

2. **DieSound** (TechnoTypeClass offset 0x510-0x528, sound list):
   ```c
   if (Type->DieSound_Count > 0) {  // offset 0x520
       int vocIndex = RandomFromSoundList(Type->DieSound);
       AnimClass__SpawnAtCoord(vocIndex, 0);  // One-shot at unit coords
   }
   ```

#### DamageSound on Non-Fatal Damage (case 1, at ~0x00702717)

```c
if (Type->DamageSound != -1) {  // TechnoTypeClass offset 0x538
    VocClass__PlayAtCoord(Type->DamageSound, &this->Coords, 0);
}
```
This is played twice (the decompiled code shows two identical blocks).

#### Death Animation from Warhead InfDeath

The warhead's `InfDeath=` value (WarheadTypeClass + 0x120) selects which death animation plays for infantry. The death animation is created separately (in the Killed virtual function, not in ReceiveDamage directly). The death anim itself can have its own `StartSound=`/`Report=` in art.ini, which plays independently of VoiceDie/DieSound.

**Summary:** When infantry die, up to THREE sounds can play:
1. `VoiceDie=` from the unit's rules.ini entry (voice line)
2. `DieSound=` from the unit's rules.ini entry (SFX)
3. The death animation's own `StartSound=`/`Report=` from art.ini

### TechnoTypeClass Sound Field Offsets (param_1 = int*)

| INI Key | Field Index | Byte Offset | Type |
|---------|-------------|-------------|------|
| `VoiceDie=` | `0x12F`-`0x135` | `0x4BC`-`0x4D4` | Sound list (DynamicVectorClass) |
| `DieSound=` | `0x144`-`0x14A` | `0x510`-`0x528` | Sound list (DynamicVectorClass) |
| `DamageSound=` | `0x14E` | `0x538` | Single VocClass index |
| `AuxSound1=` | `0x14B` | `0x52C` | Single VocClass index |
| `AuxSound2=` | `0x14C` | `0x530` | Single VocClass index |
| `CreateSound=` | `0x14D` | `0x534` | Single VocClass index |
| `MoveSound=` | `0x13D`-`0x143` | `0x4F4`-`0x50C` | Sound list |
| `VoiceMove=` | `0x10E`-`0x112` (partial) | - | Sound list |

---

## 5. Explosion Animation Sounds

**Function:** `WarheadTypeClass::Detonate` at `0x004690B0`
**Function:** `Warhead__SelectExplosionAnim` at `0x0048A4F0`

### How Explosion Anims Are Selected

The warhead's `AnimList=` (WarheadTypeClass offset 0x108/0x114) provides the explosion animation pool. When a warhead detonates:

```c
AnimTypeClass* explosionAnim = Warhead__SelectExplosionAnim(damage, warhead, resultCode, coords);
if (explosionAnim != NULL) {
    new AnimClass(explosionAnim, coords, 0, 1, 0x2600, zAdjust, 0);
}
```

`Warhead__SelectExplosionAnim` (0x0048A4F0) logic:
- If water impact and `WaterExplosion=yes`: select from `[General] SplashList=`
- If warhead == NukeWarhead: return `[General] BarracksAnim=`
- If `AnimList.Random=yes`: random selection from AnimList
- Otherwise: select based on damage level (damage / 25 maps to index)

### Sound Source for Explosions

The explosion animation's `StartSound=`/`Report=` from **art.ini** provides the explosion sound. The warhead itself has NO dedicated explosion sound field.

### WarheadTypeClass AnimList Fields

| Field | Byte Offset | Purpose |
|-------|-------------|---------|
| AnimList array ptr | `0x108` | Pointer to AnimTypeClass* array |
| AnimList count | `0x114` | Number of entries |
| AnimList.Random | `0x154` | Random selection flag |

---

## 6. Construction Animation Sounds

**INI Key:** `Construction=` in `[AudioVisual]` section of rules.ini
**RulesClass Offset:** `0x1B2` (int* index) = byte offset `0x6C8`

### How It Works

The `Construction=` sound is a global VocClass index stored in RulesClass. It is played when a building finishes its build-up animation and "slams down" into place. The related sound:

- **BuildingSlam=** (RulesClass + `0x6EC`): Played when building deployment animation completes

These are NOT per-building sounds from art.ini — they are **global** sounds defined in `[AudioVisual]`. Individual buildings do not override the construction sound.

### Related RulesClass AudioVisual Sound Fields

| INI Key | int* Index | Byte Offset | Purpose |
|---------|-----------|-------------|---------|
| `Construction=` | `0x1B2` | `0x6C8` | Building construction/build-up sound |
| `BuildingSlam=` | `0x1BB` | `0x6EC` | Building deployment slam sound |
| `BuildingDieSound=` | `0x1BA` | `0x6E8` | Building destruction sound |
| `BuildingDamageSound=` | `0x1C5` | `0x714` | Building entering damaged state sound |
| `CreateUnitSound=` | `0x5E` | `0x178` | Unit creation sound |
| `CreateInfantrySound=` | `0x5F` | `0x17C` | Infantry creation sound |
| `CreateAircraftSound=` | `0x60` | `0x180` | Aircraft creation sound |

---

## 7. Gate/Door Sounds

**INI Keys:** `GateUp=` and `GateDown=` in `[AudioVisual]` section of rules.ini

### RulesClass Fields

| INI Key | int* Index | Byte Offset | Purpose |
|---------|-----------|-------------|---------|
| `GateUp=` | `0x101` | `0x404` | Sound when gate opens (e.g., GAGATE, NAGATE) |
| `GateDown=` | `0x102` | `0x408` | Sound when gate closes |

### How It Works

Gate sounds are **global** sounds from `[AudioVisual]`, not per-building. When a building with `GateStages=` (BuildingTypeClass INI key, parsed at 0x004612D8) opens or closes its gate, the engine plays the corresponding global GateUp/GateDown sound from RulesClass.

The gate open/close logic is triggered when a unit approaches a gate building (in the building's radio response / mission logic).

---

## 8. Particle System Sounds

**Function:** `ParticleSystemTypeClass::ReadINI` at `0x006442D0`

### Finding: Particle Systems Have NO Sound Fields

After decompiling the complete ParticleSystemTypeClass::ReadINI function, the parsed INI keys are:

- HoldsWhat, Spawns, SpawnFrames, ParticleCap, SpawnRadius, Slowdown, SpawnCutoff,
  SpawnTranslucencyCutoff, Lifetime, BehavesLike, SpawnDirection, ParticlesPerCoord,
  SpiralDeltaPerCoord, SpiralRadius, PositionPerturbationCoefficient,
  MovementPerturbationCoefficient, VelocityPerturbationCoefficient,
  Laser, LaserColor, SparkSpawnFrames, LightSize, OneFrameLight, SpawnSparkPercentage

**None of these are sound-related.** Particle systems (fire, spark, smoke) are purely visual. Any sound associated with an effect that uses particles comes from:
1. The parent animation's StartSound/Report
2. A global RulesClass sound
3. The weapon/warhead that triggered the effect

---

## Summary: Sound Trigger Hierarchy

```
Animation Sound Sources:
  art.ini [AnimSection]
    StartSound= / Report=  -> Played continuously in AnimClass::AI + on Middle
    StopSound=              -> Played when anim is destroyed

Unit Death Sounds:
  rules.ini [UnitSection]
    VoiceDie=               -> Voice line on death
    DieSound=               -> SFX on death
  art.ini [DeathAnimSection]
    StartSound= / Report=  -> Death anim's own sound

Explosion Sounds:
  art.ini [ExplosionAnimSection]
    StartSound= / Report=  -> Sound comes from the AnimList anim, NOT the warhead

Building Fire Sounds:
  art.ini [FireAnimSection]
    StartSound= / Report=  -> Sound comes from the fire anim type

Building System Sounds (rules.ini [AudioVisual]):
    Construction=           -> Build-up completion
    BuildingSlam=           -> Deployment slam
    BuildingDieSound=       -> Building destruction
    BuildingDamageSound=    -> Entering damaged state
    GateUp= / GateDown=    -> Gate opening/closing

Unit Creation Sounds (rules.ini [AudioVisual]):
    CreateUnitSound=
    CreateInfantrySound=
    CreateAircraftSound=

Particle Systems:
    NO SOUND FIELDS         -> Purely visual
```

---

## Functions Labeled in This Session

| Address | Name | Purpose |
|---------|------|---------|
| `0x00750E20` | `VocClass__PlayAtCoord` | Play VocClass sound at 3D coordinates |
| `0x0048A4F0` | `Warhead__SelectExplosionAnim` | Select explosion anim from warhead AnimList |
| `0x006442D0` | `ParticleSystemTypeClass__ReadINI` | Parse particle system type from INI |
| `0x005226C0` | `InfantryClass__GetDisplayOwner` | Get display owner for infantry |
| `0x004238B0` | `AnimClass__ProcessCloakMode` | Cloak mode thunk for AnimClass |
| `0x00447780` | `BuildingClass__GrandOpening` | Building animation setup on placement |

### Previously Labeled (verified in this session)

| Address | Name |
|---------|------|
| `0x00427D00` | `AnimTypeClass__ReadINI` |
| `0x00423AC0` | `AnimClass__AI` |
| `0x00424CE0` | `AnimClass__Middle` |
| `0x00424F00` | `AnimClass__Start` |
| `0x00421EA0` | `AnimClass__Constructor` |
| `0x00424B50` | `AnimClass__SetOwnerObject` |
| `0x00750D40` | `AnimClass__UpdateLoopingSound` (corrected 2026-05-29: was `AnimClass__SpawnDetached` — RTTI_LABEL_DRIFT) |
| `0x007509E0` | `VocClass__PlayAt` (corrected 2026-05-29: was `AnimClass__SpawnAtCoord` — RTTI_LABEL_DRIFT) |
| `0x00405D40` | `AnimClass__Detach` (SoundHandle::Release) |
| `0x0043C0D0` | `BuildingClass__CreateDamageFireAnims` |
| `0x004690B0` | `WarheadTypeClass__Detonate` |
| `0x00489280` | `Apply_area_damage` |
| `0x00701900` | `TechnoClass__ReceiveDamage` |
| `0x0075D590` | `WarheadTypeClass__ReadINI` |
| `0x00712170` | `TechnoTypeClass__ReadINI` |
| `0x006691E0` | `RulesClass__ReadAudioVisual` |
