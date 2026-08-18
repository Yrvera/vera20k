# AnimClass Chaining, Damage, and Ownership — Ghidra Deep Dive

Source: Direct decompilation of gamemd.exe via Ghidra MCP.
Builds on `ANIM_CLASS_DEEP_DIVE.md` and `ANIM_CLASS_GHIDRA_REPORT.md`.
All offsets and behaviors verified from binary decompilation unless noted.

---

## 1. Anim-to-Anim Chaining: Next=

### How Next= Works (verified from AnimClass::AI at 0x423AC0)

**Next= does NOT create a new AnimClass.** It mutates the existing instance in-place
by replacing the Type pointer and resetting playback state. This is the single most
important fact about the chaining system.

**Confidence: HIGH** — verified directly from decompilation.

#### Trigger Condition

Next= is evaluated when ALL of these are true:
1. `CurrentFrame >= End` (for non-looping) or `CurrentFrame >= LoopEnd - Start` (for looping)
2. `LoopCountRemaining == 0` (all loops exhausted)
3. `type->Next != NULL` (AnimTypeClass offset 0x2C8)

#### What Gets Reset

```c
// From AnimClass::AI, the "Next" transition block:
this->Type = type->Next;                    // [0x32] = new AnimTypeClass*

// Auto-detect frame count if needed
if (next->End == -1) {
    next->End = next->GetShape()->NumFrames; // from SHP header offset +6
    if (next->Shadow) next->End /= 2;
}
if (next->LoopEnd == -1) {
    next->LoopEnd = next->End;
}

// Reset playback state
this->IsInactive = false;                    // +0x19B
this->LoopCountRemaining = next->LoopCount;  // +0x195 = next->+0x2C4
this->AccumulatedDamage = 0.0;               // [0x62..0x63] = 0
// TranslucencyStage reset was claimed here but NOT in the binary Next= block
// (corrected 2026-05-29: TranslucencyStage (+0x178 = param_1[0x5e]) is NOT reset
//  in the Next= transition; only AccumulatedDamage and LoopCountRemaining/IsInactive
//  are reset. Verified via decompile_function 0x423AC0 — OPERATOR_OR_ORDER_DRIFT)

// Recalculate rate (with RandomRate support)
rate = next->Rate;                           // next->+0x2B0
if (next->RandomRate_Min != 0 || next->RandomRate_Max != 0) {
    rate = RandomRanged(next->RandomRate_Min, next->RandomRate_Max);
}

// Handle Normalized rate
if (next->Normalized) {
    rate = FUN_005fb2e0(rate);               // normalize to game speed
}

this->LastFrameTime = g_CurrentFrameCounter; // [0x2D]
this->FrameDelay = rate;                     // [0x2F]
this->FrameDelayReload = rate;               // [0x30]
this->CurrentFrame = next->Start;            // [0x2B] = next->+0x2B4

AnimClass::Middle();                         // Begin playing the next anim
```

#### What Is Preserved

- **World coordinates** — same position, not recalculated
- **OwnerObject** (`+0xCC`) — still attached to same TechnoClass
- **OwnerHouse** (`+0x180`) — NOT reset, preserved from previous anim
- **ZAdjust** (`+0x100`) — NOT reset
- **DrawFlags** (`+0x190`) — NOT reset
- **IsBouncer** (`+0x194`) — NOT reset
- **Reverse flag** (`+0x120`) — NOT reset
- **The AnimClass instance itself** — no allocation, no destruction

#### Chain Length

Unlimited. Each Next= just replaces the Type pointer. A chain of A→B→C→D
works by: play A, when A ends morph to B, when B ends morph to C, etc.

#### Interaction with Looping

Next= is ONLY evaluated after all loops are exhausted. If the current type has
`LoopCount=3`, all 3 loops play to completion before Next= triggers.

If `LoopCount=0xFF` (255, treated as infinite), the anim loops forever and
Next= is never reached.

#### Edge Cases

- **Next with PingPong:** PingPong reversal happens independently of loop/next logic.
  If PingPong is true and LoopCountRemaining < 2, the anim reverses direction at
  frame boundaries instead of transitioning to Next.
- **Next with MakeInfantry:** If `type->Next != NULL`, the Next transition takes
  priority. MakeInfantry on the current type is skipped. MakeInfantry only runs
  when there is no Next AND the anim reaches its end.
- **Next with damage accumulation:** `AccumulatedDamage` is reset to 0.0 on
  transition. Any fractional damage from the old anim is lost.

---

## 2. TrailerAnim= and TrailerSeperation=

### How TrailerAnim Works (verified from AnimClass::AI)

**Confidence: HIGH** — verified directly from decompilation.

TrailerAnim creates new AnimClass instances periodically during playback.
Unlike Next=, trailers are independent anims that are not linked to the parent.

#### Code Flow

```c
// From AnimClass::AI, Phase 3 (trailer spawning):
if (this->IsActive && !this->IsInactive) {
    AnimTypeClass* trailerType = type->TrailerAnim;  // type+0x308
    int separation = type->TrailerSeperation;         // type+0x30C

    if (trailerType != NULL) {
        if (separation == 1 || g_CurrentFrameCounter % separation == 0) {
            // Spawn trailer at current coords
            new AnimClass(trailerType, this->GetCoords(), 1, 1, 0x600, 0, 0);
            // Note: delay param = 1 (one-tick delay before playing)
        }
    }
}
```

#### Key Details

- **Spawning frequency:** Every `TrailerSeperation` game frames. If `TrailerSeperation=1`,
  spawns every single frame. Otherwise uses signed
  `g_CurrentFrameCounter % TrailerSeperation == 0`.
- **Spawn position:** Uses `GetCoords()` (vtable+0x48), which accounts for OwnerObject
  offset if the anim is attached. So trailers follow attached anims.
- **Delay=1:** Trailer anims are created with `delay=1` (not 0). This means they wait
  one tick before `AnimClass::Middle()` is called. This prevents sound stacking from
  immediate StartSound playback.
- **LoopCount=1:** Trailers are created with loopCount=1 (single play).
- **DrawFlags=0x600:** Standard centered sprite flags.
- **No ownership transfer:** Trailers do NOT inherit OwnerObject or OwnerHouse from
  the parent. They are standalone anims at the same position.
- **No cleanup linkage:** When the parent anim ends, existing trailers keep playing
  independently. They are not destroyed.
- **Inactive check:** Trailers are NOT spawned if the anim is inactive (`+0x19B`).
  This prevents trailers from spawning on paused/hidden anims.

#### TrailerSeperation=0 With TrailerAnim Is Invalid, Not Disabled

Fresh read-only verification in
`ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md` corrected the earlier
claim here. If `TrailerAnim` is non-null, `AnimClass::AI` does not test
`TrailerSeperation` for nonzero before signed division. It special-cases
`TrailerSeperation == 1`; otherwise it evaluates
`g_CurrentFrameCounter % TrailerSeperation == 0`. Stock YR entries that set
`TrailerAnim` also set a positive `TrailerSeperation`, but a non-null trailer with
zero separation reaches divide-by-zero rather than silently disabling trailers.

Note: The INI key preserves the original misspelling "TrailerSeperation" (not
"TrailerSeparation"). Both the string in the binary and the ReadINI code use this
spelling.

---

## 3. SpawnsParticle= / NumParticles=

### Particle Spawning (from AnimClass::Start at 0x424F00)

**Confidence: HIGH** — verified from decompilation.

Particles are spawned once when `AnimClass::Start()` is called (triggered when
`CurrentFrame` reaches the midpoint frame, or when SHP data is NULL).

```c
// From AnimClass::Start:
if (type->SpawnsParticle != -1 && type->NumParticles > 0) {
    for (int i = 0; i < type->NumParticles; i++) {
        ParticleSystem_Create(
            g_ParticleSystemTypes[type->SpawnsParticle],  // type+0x2CC
            this->Location                                 // current coords
        );
    }
}
```

#### Key Details

- `SpawnsParticle` (type+0x2CC): index into ParticleSystemTypeClass array, -1 = none
- `NumParticles` (type+0x2D0): count of particle systems to spawn
- Spawned ONCE, not per-frame
- Position is the anim's Location coords (NOT GetCoords with owner offset)

---

## 4. BounceAnim= and ExpireAnim=

### BounceAnim / ExpireAnim Split (bouncer impact)

**Confidence: HIGH** — verified from decompilation.

Fresh read-only verification in
`ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md` corrected the
ownership of this path. `AnimClass::ProcessBounceResult @ 0x00423930` owns
`BounceAnim=` (`AnimType+0x300`); `ExpireAnim=` (`AnimType+0x304`) is spawned later
by `AnimClass::AI` after return `1` or `2`, terrain/water gate acceptance, and a
non-null `ExpireAnim`.

```c
// type->BounceAnim at type+0x300, inside ProcessBounceResult
if (bounceResult == 1 && type->BounceAnim != NULL) {
    new AnimClass(type->BounceAnim, this->GetCoords(), 0, 1, 0x600, 0, 0);
    // Note: BounceAnim drawFlags=0x600
}
```

The field at type+0x304 is labeled `ExpireAnim` in INI. It is an accepted-impact
animation spawned by `AnimClass::AI`, not by `ProcessBounceResult` itself:

```c
CoordStruct pos = { ftol(bounce_x), ftol(bounce_y), ftol(bounce_z) };
new AnimClass(type->ExpireAnim, pos, 0, 1, 0x2600, -30, 0);
```

This means `BounceAnim` and `ExpireAnim` are distinct same-tick spawn families:
`BounceAnim` uses `drawFlags=0x600`, while `ExpireAnim` uses
`drawFlags=0x2600` and `ZAdjust=-30`.

### ExpireAnim on Normal Anim Destruction

**CRITICAL FINDING:** The `ExpireAnim` field (type+0x304) is NOT spawned as a new
AnimClass in `AnimClass::Destroy` (0x4255B0). The Destroy function only:

1. Detaches from OwnerObject
2. Calls SetOwnerObject(NULL)
3. Releases sound
4. Plays the **StopSound** (type+0x2FC) at the SparkleCoords (+0x1B4) — NOT ExpireAnim
5. Calls ObjectClass::UnInit

**Where is ExpireAnim actually created for non-bouncers?** After extensive analysis:

- For **bouncers**: ExpireAnim is spawned on ground impact (verified above)
- For **non-bouncers**: The ExpireAnim field appears to be **unused** in the standard
  AI/Destroy path. The constructor's fallthrough path (which shares code with Destroy)
  also does not create an ExpireAnim.

**Confidence: MEDIUM-HIGH** — The Destroy function was fully decompiled and does not
contain AnimClass::Constructor calls. The constructor's destruction fallthrough also
matches Destroy exactly. However, there may be an indirect path through
`ObjectClass::UnInit` or the pending-delete cleanup that was not traced.

---

## 5. Damage Application System

### AnimTypeClass Damage Fields

| Field | Offset | Type | Default | Purpose |
|-------|--------|------|---------|---------|
| Damage | +0x2A8 | double | 0.0 | Per-frame damage amount |
| Warhead | +0x330 | WarheadTypeClass* | NULL | For bouncer impact only |
| DamageRadius | +0x334 | int | 0 | For bouncer impact only |

### AnimClass Damage Fields

| Field | Offset | Type | Default | Purpose |
|-------|--------|------|---------|---------|
| AccumulatedDamage | +0x188 | double | 0.0 | Running damage accumulator |

### Per-Frame Damage Logic (AnimClass::AI, Phase 9)

**Confidence: HIGH** — verified from decompilation.

Damage is applied via a **fractional accumulation system**. The type's `Damage` value
(a double) is accumulated each frame tick, and when the integer portion reaches >= 1,
area damage is applied.

```c
// From AnimClass::AI, after frame advancement:
if (type->Damage > 0.0 && !this->IsBouncer) {
    double multiplier = type->Damage;  // type+0x2A8 (double)

    // Building damage reduction: if owner is BuildingClass (RTTI 0x24)
    if (this->OwnerObject != NULL) {
        int rtti = this->OwnerObject->GetRTTI();  // vtable+0x2C
        if (rtti == 0x24) {  // BuildingClass
            multiplier *= 0.5;  // DAT_007e3568 = 0.5
        }
    }

    this->AccumulatedDamage += multiplier;  // +0x188 += multiplier

    if (this->AccumulatedDamage >= 1.0 && !this->field_0x198) {
        int damage = ftol(AccumulatedDamage);
        this->AccumulatedDamage -= (double)damage;

        // Warhead selection based on anim name
        char* name = (char*)(this->Type + 0x24);  // AnimTypeClass name string
        if (strcmp(name, "RING1") == 0) {
            // RING1 uses C4Warhead
            warhead = Rules->C4Warhead;  // g_RulesClass_Instance+0xFA8
        } else {
            // Everything else uses FlameDamage2
            warhead = Rules->FlameDamage2;  // g_RulesClass_Instance+0xF88
        }

        Apply_area_damage(this->GetCoords(), 0, warhead, 1, 0);
        // Note: radius=0 means point damage at exact coords

        if (!this->IsActive) return;  // damage may have killed us
    }
}
```

### Key Damage Details

1. **Timing:** Damage accumulates every frame that `CurrentFrame` advances (when
   FrameDelay timer expires). NOT every game tick — only on frame advancement ticks.

2. **Building reduction:** If the anim is attached to a BuildingClass (checked via
   OwnerObject->GetRTTI() == 0x24), damage is halved. This reduces fire damage on
   buildings.

3. **Warhead selection is hardcoded by name:** The type's own `Warhead` field is NOT
   used for per-frame damage. Instead, the anim's NAME is compared against "RING1":
   - "RING1" → `Rules->C4Warhead` (offset 0xFA8)
   - Everything else → `Rules->FlameDamage2` (offset 0xF88)

4. **The type's Warhead field** (type+0x330) is ONLY used for **bouncer impact damage**
   when a bouncing anim hits the ground. It is completely separate from per-frame damage.

5. **Radius is always 0** for per-frame damage. This means it's point damage at the
   anim's exact coordinates, not area damage.

6. **DamageApplyDelay:** There is no explicit "DamageApplyDelay" field in gamemd.exe.
   The delay between damage applications is controlled entirely by the frame Rate
   (how fast the anim plays) and the fractional accumulation system. If `Damage=0.3`,
   it takes 4 frame ticks to accumulate >= 1.0 and apply 1 point of damage.

7. **Bouncer damage** (separate system): When a Bouncer=yes anim hits the ground,
   `Apply_area_damage(coords, type->Warhead, 1, 0)` is called using the type's own
   Warhead and with radius from type->DamageRadius. This is a one-shot on impact.

### Damage Through Anim Chains (Next=)

When Next= transitions, `AccumulatedDamage` is reset to 0.0. Any fractional damage
from the previous anim in the chain is lost. The new anim starts accumulating fresh
with its own `Damage` value.

---

## 6. Ownership / House Tracking

### AnimClass Ownership Fields

| Field | Byte Offset | Index | Type | Default | Purpose |
|-------|-------------|-------|------|---------|---------|
| OwnerObject | +0x0CC | [0x33] | ObjectClass* | NULL | What this anim is attached to |
| field_0x5F | +0x17C | [0x5F] | void* | NULL | Secondary owner/link (unclear purpose) |
| OwnerHouse | +0x180 | [0x60] | HouseClass* | NULL | Which house owns this anim |

### How OwnerHouse Gets Set

**Confidence: HIGH** — verified from constructor and all major callers.

**The constructor initializes OwnerHouse to NULL (0).** It is NOT set automatically
based on the creating unit/weapon/warhead.

The key finding is: **Most anims in YR have OwnerHouse = NULL.** The field is only
populated in specific scenarios:

#### Scenario 1: MakeInfantry Path (AnimClass::AI)

When an anim finishes and has `MakeInfantry != -1`, the AI code tries to find an
owner house for the spawned infantry:

```c
// From AnimClass::AI, MakeInfantry section:
if (this->OwnerHouse == NULL || owner_is_observer) {
    // Look up country from cell
    int country = FUN_006a46d0();  // GetCountryFromCell

    // Search house array for matching country
    for (int i = 0; i < g_HouseClass_Array_Count; i++) {
        HouseClass* house = g_HouseClass_Array[i];
        if (house->Country->ArrayIndex == country) {
            this->OwnerHouse = house;  // param_1[0x60] = house
            break;
        }
    }
}
```

This only matters for parachuting infantry (CIVA, etc.) — the anim determines which
house gets credit for the spawned unit.

#### Scenario 2: External Callers Set It Directly

Some callers set `OwnerHouse` on the AnimClass instance after construction. This is
done by writing directly to the field at offset 0x180. Examples include:
- Superweapon launch code (Lightning Storm stores owner house globally, not on anims)
- Psychic Dominator mind control anim code
- Specific scripted/trigger contexts

However, **the vast majority of combat anims (explosions, fire, weapon effects) do NOT
have OwnerHouse set.** This means:
- Explosion anims from weapons: OwnerHouse = NULL
- Fire anims (Damage=X type): OwnerHouse = NULL
- Trail anims: OwnerHouse = NULL
- Building damage fires: OwnerHouse = NULL

#### Scenario 3: Detach Handler

If OwnerHouse is being destroyed (the house object is deleted), AnimClass::Detach
handles cleanup. The function checks four fields:

```c
// From AnimClass::Detach (0x425150) — full logic (corrected 2026-05-29:
// prior version showed only the OwnerHouse branch, omitting OwnerObject and
// field_0x5F branches. Verified via decompile_function 0x425150 — MISLEADING)
if (this->OwnerObject == objectPtr && objectPtr != NULL) {    // param_1[0x33]
    DisplayClass__RemoveFromLayer(this);
    vtable[0x60](this);        // RemoveAnim from OwnerObject
    this->OwnerObject = NULL;  // param_1[0x33] = 0
    this->IsInactive = true;   // NOT destroyed — marked inactive
    vtable->SetVisibility(0);
}
if (this->Type == objectPtr) {                                // param_1[0x32]
    this->Type = NULL;
}
if (this->field_0x5F == objectPtr) {                         // param_1[0x5f]
    this->field_0x5F = NULL;
    vtable->Destroy();  // Self-destruct
}
if (this->OwnerHouse == objectPtr) {                         // param_1[0x60]
    this->field_0x5F = NULL;  // clears field_0x5F (not OwnerHouse)
    vtable->Destroy();  // Self-destruct
}
```

### Ownership Through Chains (Next=)

**OwnerHouse is preserved through Next= transitions.** Since Next= mutates the
existing AnimClass in-place without resetting OwnerHouse, whatever house was set
(including NULL) carries through the entire chain.

**OwnerObject is also preserved through Next= transitions.** The anim remains
attached to the same TechnoClass.

### Kill Credit and Score

Since most damage-dealing anims have `OwnerHouse = NULL`, the per-frame damage system
(`Damage=` on AnimTypeClass) typically applies "ownerless" damage. The
`Apply_area_damage` call in the damage path passes `param_6=0` (no source house),
meaning no kill credit is awarded for flame/fire damage from anims.

For **bouncer damage**, the bullet that created the bouncing anim had an owner, but
that ownership is not transferred to the AnimClass. The bouncer's impact damage also
has no house attribution.

The **exception** is the MakeInfantry path, where OwnerHouse determines which player
gets the spawned infantry unit.

---

## 7. AnimClass::AI Full Pseudocode (0x423AC0)

This consolidates and clarifies the full execution order.

```
AnimClass::AI():

  // === PHASE 1: Pre-tick special behaviors ===

  // 1a. Update looping sound (positional audio)
  if (!field_0x198 && type->StartSound != -1):
      AnimClass__UpdateLoopingSound()           // 0x750D40

  // 1b. IsFlamingGuy bounce AI
  if (type->IsFlamingGuy):
      AnimClass__BounceAI()
      ObjectClass::AI()

  // 1c. PsiWarning visibility
  if (type->PsiWarning):
      cell = GetCell()
      visible = FUN_0043b4c0(cell)              // check if psi warning should show
      this->IsInvisible = !visible

  // 1d. Rules PsiWarning anim global toggle
  if (type == Rules->PsiWarningAnim):
      this->IsInvisible = (DAT_00a8eb7f == 0)

  // 1e. HideIfNoOre
  if (type->HideIfNoOre):
      cell = GetCell()
      this->IsInvisible = (CellClass__Get_Tiberium_Value(cell) == 0)

  // 1f. MakeInfantry cell registration
  if (type->MakeInfantry != -1):
      MarkCellOccupancy(this->Location)

  // 1g. Shadow tracking
  if (field_0x11B && field_0x47 == CurrentFrame):
      field_0x11B = false

  // === PHASE 2: Bouncer physics ===

  if (this->IsBouncer):
      layer = ProcessBounceResult()             // vtable+0x1E8
      if (layer == 2 || layer == 1):            // ground hit
          // ... bouncer impact handling (see section 4) ...
          // Spawns ExpireAnim, applies Warhead damage, spawns children
          // Self-destructs via vtable->Destroy()
          return

  // === PHASE 3: Trailer anim spawning ===

  if (IsActive && !IsInactive):
      if (type->TrailerAnim != NULL):
          if (TrailerSeperation == 1 || g_CurrentFrameCounter % TrailerSeperation == 0):
              new AnimClass(type->TrailerAnim, GetCoords(), 1, 1, 0x600, 0, 0)

  // === PHASE 4: VEINHOLE/overlay check ===

  if (type == Rules->VeinholeAnim):
      if (building exists in cell): IsInactive = true

  if (IsInactive): goto DESTROY_SELF

  // === PHASE 5: One-shot flag ===

  if (field_0x19C):
      field_0x19C = false
      return

  // === PHASE 6: Delay countdown ===

  if (Delay > 0):
      Delay--
      if (Delay == 0): AnimClass::Middle()      // begin playback
      return

  // === PHASE 7: Overlay/tiberium check ===

  if (type->IsAnimatedTiberium):
      // Check if overlay still matches this anim type
      if (overlay mismatch): IsInactive = true

  // === PHASE 8: Auto-detect frame count ===

  if (type->End == -1):
      type->End = GetShape()->NumFrames
      if (type->Shadow): type->End /= 2
  if (type->LoopEnd == -1):
      type->LoopEnd = type->End

  // === PHASE 9: SetVisibility ===

  vtable->SetVisibility()

  // === PHASE 10: Pause checks ===

  if (field_0x19E): return                      // externally paused
  if (Paused): return                           // +0x11A

  // === PHASE 11: Frame advancement ===

  remaining = CDTimerClass__GetTimeRemaining()
  if (remaining != 0 || FrameDelayReload == 0):
      FrameAdvanced = false
      return

  FrameAdvanced = true
  CurrentFrame += FrameStep                     // +1 or -1
  LastFrameTime = g_CurrentFrameCounter
  FrameDelay = FrameDelayReload                 // reload timer

  // === PHASE 12: Per-frame damage ===

  if (type->Damage > 0.0 && !IsBouncer):
      // ... damage accumulation (see section 5) ...

  // === PHASE 13: Start() trigger ===

  if (type->SHP != NULL):
      if (type->Start + CurrentFrame == SHP_total_frames):
          if (!IsBouncer): AnimClass::Start()

  // === PHASE 14: PingPong direction reversal ===

  if (type->PingPong):
      // ... direction reversal at boundaries (see deep dive) ...

  // === PHASE 15: End detection / Loop / Next ===

  if (CurrentFrame >= End):                     // (or LoopEnd-Start for looping)

      // Decrement loop count
      if (LoopCountRemaining != 0 && LoopCountRemaining != 0xFF):
          LoopCountRemaining--

      // Still looping?
      if (LoopCountRemaining > 0):
          if (Reverse || this->Reverse):
              CurrentFrame = type->LoopEnd
          else:
              CurrentFrame = type->LoopStart - type->Start

          // Apply RandomLoopDelay
          if (RandomLoopDelay configured):
              this->Delay = RandomRanged(min, max)
          return

      // No more loops — check Next=
      if (type->Next != NULL):
          // === IN-PLACE MORPH ===
          this->Type = type->Next
          // ... reset playback state (see section 1) ...
          AnimClass::Middle()
          return

      // No Next — check MakeInfantry
      if (type->MakeInfantry != -1):
          // ... spawn infantry (see deep dive) ...
          IsMarkedForDeletion = true
          vtable->Destroy()
          return

      // Simple end
      IsMarkedForDeletion = true

  DESTROY_SELF:
      vtable->Destroy()
```

---

## 8. AnimClass::Destroy (0x4255B0) — Cleanup

```c
void AnimClass::Destroy() {
    // 1. Detach from owner's anim list
    if (this->OwnerObject != NULL) {
        OwnerObject->vtable->RemoveAnim(this);  // vtable+0x60
    }

    // 2. Clear owner reference
    AnimClass::SetOwnerObject(NULL);            // 0x424B50

    // 3. Release sound handle
    SoundEvent__Release();

    // 4. Play StopSound at SparkleCoords
    if (!field_0x198 && this->Type != NULL && type->StopSound != -1) {
        CoordStruct* sparkle = &this->SparkleCoords;  // +0x1B4
        GetCoords(sparkle);
        VocClass__PlayAt(type->StopSound, sparkle);
    }

    // 5. Add to pending-delete list
    ObjectClass::UnInit();
}
```

**Key clarification:** Destroy does NOT spawn ExpireAnim. It only plays StopSound.
The constructor's fallthrough destruction path (reachable when type == NULL at end of
constructor) is identical to Destroy.

---

## 9. AnimClass Constructor — Ownership Initialization (0x421EA0)

### Parameters

```c
AnimClass::Constructor(
    AnimTypeClass* type,      // param_2
    CoordStruct*   coords,    // param_3
    int            delay,     // param_4: ticks before playing
    int            loopCount, // param_5: multiplied with type->LoopCount
    uint           drawFlags, // param_6: CC_Draw_Shape flags
    int            zAdjust,   // param_7: Z-order offset (0 = use type default)
    char           reverse    // param_8: play in reverse
)
```

### Ownership Fields Initialization

```c
param_1[0x33] = 0;      // OwnerObject = NULL
param_1[0x5F] = 0;      // field_0x5F = NULL (secondary link)
param_1[0x60] = 0;      // OwnerHouse = NULL
```

### LoopCount Calculation

```c
// loopCount param (param_5) is clamped: if < 2, set to 1
if (param_5 < 2) param_5 = 1;

// LoopCountRemaining = type->LoopCount * param_5
byte result = (byte)(type->LoopCount) * (byte)(param_5);
this->LoopCountRemaining = result;

// Ensure at least 1 loop
if (result < 2) result = 1;
this->LoopCountRemaining = result;
```

### Rate Setup

```c
int rate = type->Rate;                          // type+0x2B0

// RandomRate override
if (type->RandomRate_Min != 0 || type->RandomRate_Max != 0) {
    if (type->RandomRate_Min <= type->RandomRate_Max) {
        rate = RandomRanged(type->RandomRate_Min, type->RandomRate_Max);
    }
}

// Normalized rate
if (type->Normalized) {
    rate = FUN_005fb2e0(rate);                  // adjust for game speed
}

this->FrameDelay = rate;
this->FrameDelayReload = rate;
```

### Bouncer Initialization

If the type is a Bouncer (type+0x35A) or Meteor (type+0x356):
- Sets IsBouncer flag
- Calculates random initial velocity from Elasticity, MinZVel, MaxXYVel
- Sets up bounce physics state via FUN_004397e0

### Immediate Playback

```c
if (delay == 0) {
    AnimClass::Middle();  // begin playing immediately
}
```

---

## 10. New/Clarified Struct Fields

### AnimClass (discovered/clarified in this investigation)

| Byte Offset | Index | Field | Type | Notes |
|-------------|-------|-------|------|-------|
| +0x0CC | [0x33] | OwnerObject | ObjectClass* | Attachment target |
| +0x120 | [0x48] | Reverse | bool | Play in reverse (from constructor param_8) |
| +0x17C | [0x5F] | SecondaryLink | void* | Cleared on detach, triggers Destroy if detached |
| +0x180 | [0x60] | OwnerHouse | HouseClass* | Usually NULL; set for MakeInfantry |
| +0x188 | [0x62-0x63] | AccumulatedDamage | double | Fractional damage accumulator |
| +0x194 | [0x65] byte | IsBouncer | bool | Set for Bouncer=yes and IsMeteor=yes |
| +0x195 | +0x195 | LoopCountRemaining | byte | 0xFF = infinite, 0 = done |
| +0x198 | [0x66] byte | field_0x198 | bool | Suppresses sound and damage |
| +0x19B | +0x19B | IsInactive | bool | Suppresses drawing and AI |
| +0x19C | [0x67] byte | OneShot | bool | Set to 1 in constructor, cleared after first AI tick |

### AnimTypeClass (all previously documented, confirmed here)

| Byte Offset | Index | INI Key | Type | Default |
|-------------|-------|---------|------|---------|
| +0x2A8 | [0xAA-AB] | Damage | double | 0.0 |
| +0x2C8 | [0xB2] | Next | AnimTypeClass* | NULL |
| +0x2CC | [0xB3] | SpawnsParticle | int | -1 |
| +0x2D0 | [0xB4] | NumParticles | int | 0 |
| +0x300 | [0xC0] | BounceAnim | AnimTypeClass* | NULL |
| +0x304 | [0xC1] | ExpireAnim | AnimTypeClass* | NULL |
| +0x308 | [0xC2] | TrailerAnim | AnimTypeClass* | NULL |
| +0x30C | [0xC3] | TrailerSeperation | int | 0 |
| +0x330 | [0xCC] | Warhead | WarheadTypeClass* | NULL |
| +0x334 | [0xCD] | DamageRadius | int | 0 |

---

## 11. Function Address Reference

| Address | Name | Notes |
|---------|------|-------|
| 0x421EA0 | AnimClass::Constructor | 7 params + this, full initialization |
| 0x422720 | AnimClass::Constructor (load) | Deserialization, no params |
| 0x423AC0 | AnimClass::AI | Per-tick update, 587 lines |
| 0x424B50 | AnimClass::SetOwnerObject | Attach/detach from TechnoClass |
| 0x424CE0 | AnimClass::Middle | Called when delay expires, begins play |
| 0x424F00 | AnimClass::Start | Sound/particle/scorch on midpoint |
| 0x425150 | AnimClass::Detach | Detach handler for dying objects |
| 0x4255B0 | AnimClass::Destroy | Self-removal + cleanup |
| 0x425630 | AnimClass::GetZAdjust | ZAdjust + owner's ZAdjust |
| 0x425670 | AnimClass::BounceAI | Bouncer movement physics |
| 0x425D10 | AnimClass::FindAttachTarget | Bouncer landing position search |
| 0x426270 | AnimClass::MarkCellOccupancy | Register in cell |
| 0x426300 | AnimClass::ClearCellOccupancy | Unregister from cell |
| 0x427D00 | AnimTypeClass::ReadINI | Parses art.ini |
| 0x428B80 | AnimTypeClass::FindByName | |
| 0x428F70 | AnimTypeClass::FindOrCreate | Used for Next, TrailerAnim, etc. |
| 0x489280 | Apply_area_damage | Damage distribution |
| 0x750D40 | AnimClass::UpdateLoopingSound | Positional looping sound update |

---

## 12. Summary of Gaps Filled

### Previously Documented (in ANIM_CLASS_DEEP_DIVE.md)
- AnimClass::AI full flow ✓
- Next= in-place morph mechanism ✓
- TrailerAnim spawning basics ✓
- Per-frame damage accumulation ✓
- Bouncer physics ✓

### New Findings in This Report

1. **TrailerAnim delay=1:** Trailers are created with delay=1, not delay=0.
   This prevents immediate sound stacking.

2. **Trailers have NO ownership transfer:** No OwnerObject or OwnerHouse from parent.

3. **TrailerSeperation is global-frame modulo:** `TrailerSeperation==1` spawns every
   tick; otherwise non-null `TrailerAnim` uses `g_CurrentFrameCounter % TrailerSeperation`.
   There is no zero-disable guard.

4. **ExpireAnim is NOT spawned on normal anim destruction:** Only used for bouncer
   impact. The Destroy function plays StopSound, not ExpireAnim.

5. **OwnerHouse is usually NULL:** The constructor sets it to 0. Most callers
   (TechnoClass::Fire_At, WarheadTypeClass::Detonate, Apply_area_damage,
   LightningStorm, BuildingClass) do NOT set it after construction.

6. **OwnerHouse only matters for MakeInfantry:** The only code that reads OwnerHouse
   is the infantry spawning path, where it determines which player gets the unit.

7. **Per-frame damage has no house attribution:** Apply_area_damage is called with
   no source house for flame/fire damage. No kill credit for fire deaths.

7. **Warhead selection is hardcoded by name comparison:** "RING1" gets C4Warhead,
   everything else gets FlameDamage2. The type's own Warhead field is for bouncer
   impact only.

8. **AccumulatedDamage reset on Next= transition:** Fractional damage is lost when
   chaining to the next animation.

9. **OwnerHouse preserved through Next= chains:** Since no reset occurs, whatever
   was set (usually NULL) carries through.

10. **AnimClass::UpdateLoopingSound (0x750D40):** Previously mislabeled as
    "SpawnDetached". Actually handles positional looping sound volume/pan updates.
    Called at start of AI when the type has a StartSound.
