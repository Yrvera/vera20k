# WaveClass — AI Function + Trigger Corrections (2026-04-22 Addendum)

**Companion to** `WAVECLASS_GHIDRA_REPORT.md`. This addendum closes the Open
Questions from that report (per-tick AI, damage application, WaveType 1/2
triggers) and corrects two claims about which weapons instantiate WaveClass.

**Confidence:** HIGH for the AI function (decompiled end-to-end); HIGH for the
WeaponTypeClass flag offset map (read from `WeaponTypeClass::ReadINI`); HIGH
for the "IsSonic is dead in YR" finding (grep over `ini/rulesmd.ini` +
`ini/rules.ini`). The correction to the original report's "IsLaser/IsBigLaser
trigger WaveClass" claim is HIGH (those flags route to LaserDrawClass, a
different 0x5C class).

---

## 1. Two Beam-Effect Classes, Not One

The original `WAVECLASS_GHIDRA_REPORT.md` implied that every laser-style weapon
in YR instantiates WaveClass. This is wrong. The binary has **two separate
beam-effect classes**:

| Class | Size | Constructor | Used by (verified callsites) |
|-------|------|-------------|------------------------------|
| **WaveClass**      | `0x240` | `0x0075E950` | `TechnoClass::Fire_At` at `0x006FF470` (type 0), `0x006FF647` (type 3) |
| **LaserDrawClass** | `0x5C`  | `0x0054FE60` | `TechnoClass::SpawnLaser @ 0x006FD210` (main Prism firing beam), `BuildingClass::EmitPrismSupportBeam @ 0x0044ABD0` (support beam), `DiskLaserClass::AI @ 0x004A7340` (Vortex), `ParticleSystemClass::AI_Railgun @ 0x0062F230` (Railgun) |

`IsLaser=yes`, `IsBigLaser=yes`, `DiskLaser=yes`, Railgun, and Prism Forwarding
all go through **LaserDrawClass**. Only weapons with `IsSonic=yes` or
`IsMagBeam=yes` go through WaveClass.

**Cross-reference:** `PRISM_FORWARDING_GHIDRA_REPORT.md` §4 documents LaserDrawClass
(`FUN_0054FE60`) as the 92-byte "laser-draw instance" shared by the Prism main
fire and support-beam paths. That report and this report are consistent —
they describe two different classes.

## 2. Authoritative WeaponTypeClass Flag Offsets — read from ReadINI

Source: `WeaponTypeClass::ReadINI @ 0x00772080`, decompiled end-to-end.
Every flag below was read at the exact point where its INI string is loaded
and `CCINIClass::ReadBool` writes the byte.

| INI Key | Byte Offset | Triggers |
|---------|-------------|----------|
| `IsSonic`         | `+0x130` | **WaveClass type 0** (via Fire_At @ 0x006FF460) |
| `Spawner`         | `+0x131` | Aircraft carrier / Dreadnought spawn behavior |
| `LimboLaunch`     | `+0x132` | |
| `DecloakToFire`   | `+0x133` | |
| `CellRangefinding`| `+0x134` | |
| `FireOnce`        | `+0x135` | |
| `NeverUse`        | `+0x136` | |
| `RevealOnFire`    | `+0x137` | |
| `TerrainFire`     | `+0x138` | |
| `SabotageCursor`  | `+0x139` | |
| `MigAttackCursor` | `+0x13A` | |
| `DisguiseFireOnly`| `+0x13B` | |
| `InfiniteMindControl` | `+0x140` | |
| `FireWhileMoving` | `+0x141` | |
| `DrainWeapon`     | `+0x142` | |
| `FireInTransport` | `+0x143` | |
| `Suicide`         | `+0x144` | |
| `TurboBoost`      | `+0x145` | |
| `Supress`         | `+0x146` | |
| `Camera`          | `+0x147` | |
| `Charges`         | `+0x148` | |
| `IsLaser`         | `+0x149` | **LaserDrawClass** via SpawnLaser |
| `DiskLaser`       | `+0x14A` | **LaserDrawClass** |
| `IsLine`          | `+0x14B` | |
| `IsBigLaser`      | `+0x14C` | **LaserDrawClass** |
| `IsHouseColor`    | `+0x14D` | Color toggle for whichever beam class is used |
| `LaserDuration`   | `+0x14E` (int) | |
| `IonSensitive`    | `+0x14F` | |
| `AreaFire`        | `+0x150` | |
| `IsElectricBolt`  | `+0x151` | |
| `DrawBoltAsLaser` | `+0x152` | |
| `IsAlternateColor`| `+0x153` | |
| `IsRadBeam`       | `+0x154` | RadBeam class (separate, NOT WaveClass — needs its own research pass) |
| `IsRadEruption`   | `+0x155` | |
| `RadLevel`        | `+0x158` (int) | |
| `IsMagBeam`       | `+0x15C` | **WaveClass type 3** (via Fire_At @ 0x006FF5F5) |

### Corrections to the original WaveClass report

The original report §8 ("INI Bindings") mapped:

> `+0x130` → `IsLaser=yes` / `IsBigLaser=yes` / `IsSonic=yes` / `IsRadBeam=yes` (WaveType 0)

**This is wrong.** `+0x130` is **IsSonic only**. The correct mapping is:

- `+0x130` = `IsSonic` → WaveClass type 0
- `+0x149` = `IsLaser` → LaserDrawClass (different class entirely)
- `+0x14A` = `DiskLaser` → LaserDrawClass
- `+0x14C` = `IsBigLaser` → LaserDrawClass
- `+0x154` = `IsRadBeam` → (neither WaveClass nor LaserDrawClass — uses a
  dedicated RadBeam system; not decoded this pass)
- `+0x15C` = `IsMagBeam` → WaveClass type 3

## 3. ⚠ CRITICAL: IsSonic is TS-LEGACY DEAD CODE IN YR

Grep over `ini/rulesmd.ini` + `ini/rules.ini` for `IsSonic\s*=\s*(yes|true)`:
**zero matches** across both files.

This means the WaveClass WaveType=0 code path (triggered by
`WeaponTypeClass+0x130`) is **never instantiated in a stock YR game**. The
code exists in the binary, the LUTs and vertex tables at
`DAT_00B45DA8`/`DAT_00B45D80` exist, but no shipping weapon sets IsSonic=yes.

Historical context: `IsSonic=yes` was the Tiberian Sun Sonic Tank's weapon
flag. In YR the Dolphin has a "sonic" attack but uses a regular projectile
(DolphinSonic weapon → DolphinPulse warhead) with no IsSonic flag. The
Mirage Tank uses a regular shell, not a beam.

**Implication for a Rust port:** do not implement WaveClass type 0 unless
you explicitly want TS-Sonic-Tank mod compatibility. Only WaveClass type 3
(Magnetron beam, Yuri's `[GMAGN]` unit) is live in YR.

**WaveType 1 and 2** (referenced in the constructor's `if (0 < type < 3)`
branch) have NO callsites in stock Fire_At. They are either (a) TS-era
intermediate types (Magbeam variants in the TS Sonic Tank era), (b)
reserved slots, or (c) used only by modded content. Confidence: HIGH that
they are not instantiated by standard YR game code.

## 4. Per-Tick AI Function — `FUN_00762AF0` Decoded

This was Open Question #2 in the original report. Verified this pass:

- **Address:** `0x00762AF0`
- **Role:** The WaveClass per-tick AI update. Called from the end of the
  constructor (initial tick) and every game tick from the wave manager's
  iteration over `DAT_00A8EC3C[0..DAT_00A8EC48]`.
- **Returns:** void. The wave removes itself via `this->vtable[0xF8]()` when
  both fade phases complete.

### Algorithm (decoded from the 200-line function)

```c
void WaveClass::AI(WaveClass *this) {
    // --- PHASE 1: deactivation tests ---

    // Test A: fade-in exceeded fade-out by more than 0.5 AND WaveType == 0
    //         → mark for fade-out (sets the "decay" flag)
    if (this->FadeInProgress - this->FadeOutProgress > DAT_007e3860  // const ~0.5
        && this->WaveType == 0) {
        this->IsDecaying = 1;                                       // +0x12D
    }

    // Test B: WaveType 3 animation-stage advance
    if (this->AnimIndex == Math::ftol(...)                           // some int-equality test
        && this->WaveType == 3) {
        this->AnimIndex   = 0x40;                                    // reset to 64
        this->AnimCounter += 1;
    }

    // Test C: validity check (target or owner gone?)
    if (this->Target == null || this->OwnerLink == null) {
        this->IsActive = 0;                                           // +0x12C
        this->IsDecaying = 1;
    } else if (this->AnimIndex == Math::ftol(...) ||
               this->OwnerLink->CurrentTarget != this->Target) {
        // Owner no longer aiming at same target → deactivate
        this->IsActive = 0;
        this->IsDecaying = 1;
    } else if (this->WaveType != 3) {
        // Test D: if the firer has moved too far from the saved beam origin
        // (saved during geometry compute), deactivate. Uses a LUT-based
        // distance threshold.
        int d = sqrt((owner.pos - target.pos).length_sq());
        if (d > DAT_00B45D80 * DAT_007EAA98) {
            this->IsActive = 0; this->IsDecaying = 1;
        }
    }

    // --- PHASE 2: geometry recompute (only if still active) ---
    if (this->IsActive) {
        if (this->WaveType == 3) {
            if (this->Target.vtable[0x2C]() == 1) {                    // Target.GetMission() == Attack
                // Target is attacking — recompute beam with target's weapon coord
                coord = this->OwnerLink.vtable[0xB0](...);               // Owner render coord
                target = this->Target.vtable[0x58](..., coord);          // Adjust by target offset
                FUN_00762070(this, target, 0);                           // type-3 geometry
            } else {
                // Target idle — recompute beam between owner and target
                coord = this->Target.vtable[0x58]();                     // Target coord
                endp  = this->OwnerLink.vtable[0xB0](..., coord);        // Owner's attack coord
                FUN_00762070(this, endp, buffer);                        // type-3 geometry
            }
        } else {
            // Types 0/1/2 — always recompute from current owner + target coords
            coord = this->Target.vtable[0x58]();
            endp  = this->OwnerLink.vtable[0xB0](..., coord);
            FUN_00761640(this, endp, buffer);                            // type-0/1/2 geometry
        }
    }

    // --- PHASE 3: fade-in and fade-out animation ---

    if (this->WaveType == 3) {
        // Type 3 has a more complex 3-phase anim with wrap
        if (this->FadeInProgress < 1.0 && this->IsActive) {
            this->FadeInProgress += _DAT_007F6BB0;                       // per-frame delta
            if (this->FadeInProgress > _DAT_007F6DE8) {                  // overshoot threshold
                this->FadeInProgress = 0;                                // reset
                // +0x13C also reset to 1.0 hi bits (0x3FF00000)
            }
            // write screenspace start & end of fade-in arc to point list
        }
        if (this->IsDecaying
           && this->FadeOutProgress <= _DAT_007F6BB0 * _DAT_007E1738
                                          + this->FadeInProgress) {
            this->FadeOutProgress += _DAT_007F6BB0;
            if (this->FadeInProgress <= this->FadeOutProgress) {
                this->vtable[0xF8]();                                    // = Detach/Remove
                return;
            }
            // write screenspace mid & end of fade-out arc
            FUN_007610F0();                                              // submit for draw
            return;
        }
    } else {
        // Types 0/1/2 — simpler two-phase fade
        if (this->FadeInProgress < 1.0 && this->IsActive) {
            this->FadeInProgress += _DAT_007F6BB0;
            if (this->FadeInProgress > _DAT_007F6DE8) {
                this->FadeInProgress = 0;
                // reset
            }
            // write screenspace points (0..15 bytes) for fade-in arc
        }
        if (this->IsDecaying
           && this->FadeOutProgress <= _DAT_007F6BB0 * _DAT_007E1738
                                          + this->FadeInProgress) {
            this->FadeOutProgress += _DAT_007F6BB0;
            if (this->FadeInProgress <= this->FadeOutProgress) {
                this->vtable[0xF8]();
                return;
            }
            // write screenspace points (0x18..0x2C) for fade-out arc
        }
    }

    FUN_007610F0();                                                      // submit for draw
}
```

### Key observations

1. **WaveClass does NOT apply damage.** There is no call to any damage, warhead,
   or `Apply_area_damage` function in the AI. Damage for IsSonic / IsMagBeam
   weapons flows through the normal BulletClass → WarheadTypeClass pipeline
   in `TechnoClass::Fire_At` — the WaveClass is purely visual.

2. **Per-tick geometry recompute means beams track.** Unlike LaserDrawClass
   (which samples endpoints at construction), WaveClass re-fetches both
   owner and target coords every tick. This is what makes the Magnetron
   beam visually stretch and follow as it levitates a target.

3. **Two constants drive the fade cadence:**
   - `_DAT_007F6BB0` = per-frame fade-progress delta
   - `_DAT_007E1738` = fade-out lag factor (fade-out starts when fade-in has
     advanced `_DAT_007E1738` frames beyond it)

   Neither is INI-driven. These are compiled-in constants.

4. **Removal is via `vtable[0xF8]`.** Same vtable slot as ObjectClass detach;
   the wave is simultaneously removed from `DAT_00A8EC3C` (wave manager
   array) via the Detach mechanism.

5. **Magnetron type-3 branch also does animation-stage advancement via
   `+0x130` (AnimIndex) reset to 64 and `+0x134` (AnimCounter) increment.**
   These were labeled "InitialStrength / Damage" in the original report —
   that label is incorrect. They are animation state, not damage state. The
   constructor's initial value of 100 is the "animation timeline length",
   not a damage value.

## 5. Fire_At Gate Conditions — Verified

From `TechnoClass::Fire_At @ 0x006FDD50`, the exact disassembly of the two
WaveClass callsites:

### Type 0 callsite @ `0x006FF470`

```
; EBX = WeaponType*
006ff43f: MOV AL, byte ptr [EBX + 0x130]      ; IsSonic
006ff445: TEST AL, AL
006ff447: JZ 0x006ff48a                       ; skip if !IsSonic

006ff449: PUSH 0x240                          ; sizeof(WaveClass)
006ff44e: CALL operator_new
...
006ff461: PUSH EDI                            ; target (arg 5)
006ff462: PUSH 0x0                            ; waveType = 0 (arg 4)
006ff464: PUSH ESI                            ; owner (arg 3)
006ff465: LEA EDX, [ESP + 0x94]               ; dst coord (arg 2)
006ff46c: PUSH ECX                            ; src coord (arg 1)
006ff46d: PUSH EDX
006ff46e: MOV ECX, EAX                        ; this = newed mem
006ff470: CALL 0x0075e950                     ; WaveClass::Constructor
006ff475: MOV dword ptr [ESI + 0x324], EAX   ; firer->CurrentWave = newwave
```

### Type 3 callsite @ `0x006FF647`

```
; EBX = WeaponType*, EDI = target, ESI = firer
006ff5f5: MOV AL, byte ptr [EBX + 0x15c]      ; IsMagBeam
006ff5fb: TEST AL, AL
006ff5fd: JZ 0x006ff656                       ; skip if !IsMagBeam

006ff5ff: MOV EAX, dword ptr [ESI + 0x324]    ; firer->CurrentWave
006ff605: TEST EAX, EAX
006ff607: JZ 0x006ff623                       ; if no current wave, skip the target-mission check
006ff609: TEST EDI, EDI
006ff60b: JZ 0x006ff619
006ff60d: MOV EDX, dword ptr [EDI]
006ff60f: MOV ECX, EDI
006ff611: CALL dword ptr [EDX + 0x2c]        ; target.GetMission()
006ff614: CMP EAX, 0x6                         ; == Sticky (building)?
006ff617: JZ 0x006ff656                       ; if targeting a building, skip (don't spawn another wave)

006ff619: MOV EAX, dword ptr [ESI + 0x324]
006ff61f: TEST EAX, EAX
006ff621: JNZ 0x006ff656                      ; skip if firer already has a wave
006ff623: PUSH 0x240                          ; sizeof(WaveClass)
006ff628: CALL operator_new
...
006ff634: PUSH EDI                            ; target (arg 5)
006ff635: PUSH 0x3                            ; waveType = 3 (arg 4)
006ff637: LEA ECX, [ESP + 0x4c]               ; src coord
006ff63b: PUSH ESI                            ; owner (arg 3)
006ff63c: LEA EDX, [ESP + 0x94]               ; dst coord
006ff643: PUSH ECX
006ff644: PUSH EDX
006ff645: MOV ECX, EAX
006ff647: CALL 0x0075e950                     ; WaveClass::Constructor
```

The Type 3 callsite has additional de-duplication logic: if the firer
already has a live wave, no new wave is spawned; also, if the target is a
building (mission 6), no wave is spawned (Magnetron beams don't target
buildings in vanilla YR).

`firer + 0x324` is the **CurrentWave** slot on TechnoClass — a single-wave
tracker per firer. This is how the game prevents stacking multiple
Magnetron beams on one Yuri unit. Store site: both callsites write
`firer->+0x324 = newwave` after successful construction.

## 6. Summary of Corrections to `WAVECLASS_GHIDRA_REPORT.md`

| Claim in original report | Correction |
|--------------------------|-----------|
| `+0x130 = IsLaser/IsBigLaser/IsSonic/IsRadBeam` (one offset, 4 flags) | **Wrong.** `+0x130 = IsSonic ONLY.` IsLaser is `+0x149`, IsBigLaser `+0x14C`, IsRadBeam `+0x154` — and those route to other classes, not WaveClass. |
| WaveClass is instantiated by "every beam-style weapon" | **Partially wrong.** Only IsSonic and IsMagBeam route here. IsLaser/IsBigLaser/DiskLaser route to LaserDrawClass; IsRadBeam routes to RadBeam. |
| WaveType 0 is "the most common (laser/sonic/radbeam)" | **Wrong.** WaveType 0 is **dead code in stock YR** — no IsSonic=yes weapon exists in `ini/rulesmd.ini`. Only WaveType 3 (MagBeam) is live. |
| `+0x130` field labeled "InitialStrength / Damage" (init to 100) | **Wrong semantics.** It's animation state (AnimIndex), reset to 64 during type-3 anim stage advancement. Not a damage multiplier. |
| WaveType 1, 2 usage "needs xref verification" | **Resolved.** No Fire_At callsites for 1 or 2 exist. Both are TS-legacy or reserved. Do not implement. |
| Per-tick AI "not decoded" (Open Q #2) | **Resolved.** `FUN_00762AF0` is the AI; it does no damage, only geometry recompute + fade animation + detach. |
| Damage application "unclear" | **Resolved.** WaveClass does NOT apply damage. Damage flows through the normal BulletClass path spawned in the same `Fire_At` call. |

## 7. What a Rust Port Must Match (Wave — Corrected)

1. **Only wire up WaveClass type 3** (IsMagBeam=yes weapons). Skip type 0
   unless you explicitly want TS Sonic Tank mod support.
2. **WaveClass is pure rendering.** Implement it as a visual effect with:
   - Per-frame owner/target coord re-fetch (so the beam tracks)
   - Fade-in progress counter (`0.0 → 1.0` over N frames)
   - Fade-out progress counter, triggered when target/owner is gone or
     owner's target changes
   - Removal when fade-in ≤ fade-out
3. **Track per-firer "CurrentWave" slot.** Exactly one wave per firing
   TechnoClass at a time. The game checks and writes `firer+0x324`.
4. **Don't apply damage in the wave.** Damage is already in the parallel
   bullet spawned by Fire_At.
5. **WaveType 3 skips when target is a building (Mission==6).** Do not
   spawn the wave in that case.

---

**Verified** from `gamemd.exe` (image base `0x00400000`) via Ghidra MCP, 2026-04-22.
Functions decompiled this pass: `WeaponTypeClass::ReadINI @ 0x00772080` (full),
`WaveClass::AI @ 0x00762AF0` (full), `TechnoClass::Fire_At` wave-construction
callsites (disassembled `0x006FF43F-0x006FF652`), `TechnoClass::SpawnLaser @
0x006FD210` (full — confirmed LaserDrawClass uses `operator_new(0x5C)`).
INI checks run against `ini/rulesmd.ini` and `ini/rules.ini` for `IsSonic`
pattern.
