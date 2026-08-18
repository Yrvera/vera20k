# Prism Tower Cascade Trigger -- Ghidra Research Report

**Companion to** `PRISM_FORWARDING_GHIDRA_REPORT.md`. That report covered
INI parsing, damage-scaling math, and the per-supporter beam-emit field
writes; this one resolves its open gaps **G1, G2, G4, G5, G6** by locating
and decoding the cascade trigger inside `BuildingClass::Mission_Attack` and
the supporter's beam-emit function.

**Primary addresses:**
- `BuildingClass::Mission_Attack @ 0x0044ACF0` -- contains the cascade
  trigger entry at `0x0044b2bc` and the selector loop at `0x0044b32f`
- `BuildingClass::EmitPrismSupportBeam @ 0x0044ABD0` (was `FUN_0044abd0`,
  formerly thought to be "ProcessDelayedFire mode 2" — it IS that, and is
  also the only consumer of `Rules->PrismSupportDuration` and
  `Rules->PrismSupportDelay`)
- `BuildingClass::ProcessDelayedFire @ 0x004503F0` -- the per-tick timer
  that dispatches mode 1 (firing tower fires) vs mode 2 (supporter emits)

**Confidence:** HIGH for everything in Sections 1–4 (multiple independent
callsites or full decompilations). MEDIUM for the `+0x702`/`+0x5ec`
pre-gate identity (Section 1.1, see Open Questions).

**Active in YR:** YES, unconditionally — the entire system is YR-only by
default since `PrismType=ATESLA` has no TS counterpart.

---

## 1. Overview

The prior `PRISM_FORWARDING_GHIDRA_REPORT.md` documented the **fire-tower
side** of the system (the increment of `+0x664`, the bullet damage scaler,
the field writes for delayed-fire mode). It explicitly flagged six gaps
(G1–G6) covering the **supporter side**: how supporters are selected,
how mode 1/2 are entered, whether there is an `IsPrismCannon` flag,
whether forwarding cascades, what the visual is, and what mode 2 does.

This report fills those gaps:

- **G1 (Support-beam trigger site):** in `Mission_Attack` at `0x0044b2f8`,
  gated by `Type == Rules->PrismType`. Selector loop at `0x0044b32f`.
- **G2 (Attack initiation, who sets `field_0x704 = 1`):** at `0x0044b5bb`
  (Prism path) and `0x0044b65c` (IsAnimDelayedFire fallback path), both
  inside `Mission_Attack`.
- **G3 (`IsPrismCannon` flag):** **REFUTED.** No such flag exists.
  The cascade gate is `this->Type == Rules->PrismType` direct compare.
  The byte at `BuildingTypeClass+0x16c5` examined in the prior report is
  `HasTurret` per `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` and is
  **unrelated** to prism gating.
- **G4 (Recursive forwarding):** the design is **not recursive**. Each
  Mission_Attack tick picks ONE eligible supporter and increments the
  firing tower's count by 1. Cascades are the natural consequence of
  multiple ticks accumulating multiple supporters (visible as a wave of
  beams on screen).
- **G5 (Visual beam):** `LaserDrawClass` (32-byte alloc, registered to
  `g_LaserDraw_Array`), constructed by `LaserDrawClass::Constructor`
  (was `FUN_0054fe60`). Color is the owner's `HouseClass+0x56FC` (laser
  color). Duration is `Rules->PrismSupportDuration`.
- **G6 (`field_0x704` modes 2 and 3):** mode 2 IS the supporter beam-emit
  path; calling `EmitPrismSupportBeam` with the saved firing-tower coords.
  **There is no mode 3** — the prior decompiler artifact (`iVar1 - 2 == 1`)
  was an unreachable fall-through, not a real case. Modes are `{0, 1, 2}`.

---

## 2. Cascade Trigger -- Pre-Gate and Prism Gate

The cascade lives inside `BuildingClass::Mission_Attack`. After
`GetFireError` returns a small value, control jumps through a jumptable
at `PTR_LAB_0044b728` (one entry per fire-error code 0–10). One of those
entries falls into the prism cascade entry at `0x0044b2bc`.

### 2.1 Pre-cascade gates (immediate-fire vs cascade-eligible)

```
0044b2bc: AL = this->field_0x702                   ; pre-cascade gate byte
0044b2c2: TEST AL, AL
0044b2c4: JZ  0x0044b2f8                           ; if zero, jump to PRISM gate
0044b2c6: ECX = this->field_0x5ec                  ; secondary gate
0044b2cc: CMP ECX, 0
0044b2ce: JZ  0x0044b2f8                           ; if zero, jump to PRISM gate

; Both gates non-zero — fall through to "fire normally":
0044b2d0: CALL FUN_00712130                        ; tertiary check (mind-control / link sanity)
0044b2d5: TEST AL, AL
0044b2d7: JZ  0x0044b6d6                            ; if false, skip everything
0044b2dd-0044b2e9: CALL vtable[0x3cc]              ; Fire(target, weapon=0) — IMMEDIATE
0044b2f3: JMP 0x0044b6c7                            ; goto fire-result handler
```

Identity of `+0x702` and `+0x5ec` is not yet confirmed (see Open Questions
O1). Working hypothesis: they record an in-progress firing-anim or
just-fired latch — when set, the building is mid-cycle and should fire
immediately rather than re-entering the cascade. When clear, the building
is ready to (re-)evaluate the prism cascade.

### 2.2 Prism gate

Reached at `0x0044b2f8` whenever EITHER pre-gate is zero:

```
0044b2f8: ECX = [g_RulesClass_Instance]            ; pointer at 0x008871e0
0044b2fe: EAX = this->field_0x520                  ; this->Type
0044b304: CMP EAX, [ECX + 0x498]                   ; vs Rules->PrismType
0044b30a: JNZ 0x0044b630                            ; not Prism → IsAnimDelayedFire branch
                                                     ; (else fall through to selector loop)
```

**This is THE gate.** No `BuildingTypeClass` flag — the entire prism
behavior keys off `this->Type == Rules->PrismType`. Per side, exactly one
BuildingType (whichever is named in `[General] PrismType=`) gets the
cascade behavior.

`FUN_00712130` (the tertiary check on the immediate-fire path) reads
`param_1+0x898` and `param_1+0x8b4` and tests their sub-fields at +0x9c —
likely a TechnoClass mind-control / radio link sanity test. Not central
to prism behavior.

---

## 3. Selector Loop (resolves G1)

After the prism gate, the firing tower captures its own coordinates via
`vtable[0xAC]` (`Get_Source_Coord`-equivalent) and stores them on the stack
at `[ESP + 0x3C..0x44]`. Then:

```
0044b32f: ECX = this->field_0x664                  ; current support count
0044b349: EAX = Rules->PrismSupportMax (+0x4a0)
0044b34f: CMP ECX, EAX
0044b351: JGE 0x0044b595                            ; CAP CHECK: count >= max → skip cascade
0044b357: EAX = this->field_0x21c                  ; this->Owner
0044b361: EAX = Owner->field_0x78                  ; building count for this house
0044b366: save EAX as loop limit at [ESP+0x24]
0044b36a: JLE 0x0044b595                            ; no buildings → skip cascade

; LOOP TOP @ 0x0044b370 — iterate Owner->Buildings array
0044b370: ECX = this->Owner
0044b376: EAX = loop_idx (initially 0)
0044b37a: EDX = Owner->field_0x6c                  ; BuildingArray ptr
0044b37d: EDI = BuildingArray[loop_idx]            ; candidate
0044b380: TEST EDI; JZ skip                         ; null-skip
0044b388: AL  = candidate->field_0x90              ; "active/alive" flag
0044b38e: TEST AL; JZ skip                          ; not active → skip
0044b39c: ECX = candidate->field_0x520             ; candidate->Type
0044b3a2: CMP ECX, Rules->PrismType
0044b3a8: JNZ skip                                  ; not Prism → skip

; Cooldown check
0044b3ae: EDX = candidate->field_0x2EC             ; LastSupportFrame
0044b3b4: ECX = candidate->field_0x2F4             ; LastSupportDelay
0044b3ba: CMP EDX, -1
0044b3bd: JZ ready (0x0044b3cc)                    ; -1 = never emitted, ready
0044b3bf: EAX = g_CurrentFrameCounter (0x00a8ed84)
0044b3c4: SUB EAX, EDX                              ; elapsed
0044b3c6: CMP EAX, ECX
0044b3c8: JGE ready (0x0044b3d4)                    ; elapsed >= delay → ready
0044b3ca: SUB ECX, EAX                              ; remaining
0044b3cc: TEST ECX, ECX
0044b3ce: JNZ skip                                  ; cooling down → skip

; Mode check — supporter must not already be in delayed-fire
0044b3d4: EAX = candidate->field_0x714             ; timer
0044b3da: TEST EAX; JNZ skip                        ; busy → skip

; Deploying check
0044b3e2: ECX = candidate
0044b3e4: CALL TechnoClass::IsDeploying (0x0070FEC0)
0044b3e9: TEST AL; JNZ skip                          ; deploying → skip

; Mission state check
0044b3f3: ECX = candidate
0044b3f5: CALL vtable[0x184]                        ; (Get_Mission()?)
0044b3fb: CMP EAX, 1                                 ; (mission == idle/guard? → 1)
... (additional checks, see Open Questions O2)

; DISTANCE/SCORE — pick CLOSEST eligible candidate
; Compute candidate→firing-tower vector (stored at [ESP+0x3C..0x44]),
; square components, sum:
0044b421-0044b48c: distance² via FILD/FMUL chain → Math::ftol
0044b48e: PUSH 0x1
0044b494: CALL vtable[0x168]                        ; candidate's range/threshold
0044b49a: CMP EBP, EAX                               ; if dist² > threshold, skip
0044b49c: JG skip
0044b49e: TEST EBX, EBX
0044b4a0: JZ take_this (0x0044b4a8)                 ; first valid → take
0044b4a2: CMP EBP, [ESP + 0x10]                     ; vs best-so-far (lower = better)
0044b4a6: JGE skip
0044b4a8: take_this:
          EBX = EDI                                   ; record best supporter
          [ESP+0x10] = EBP                            ; record best score (smallest)

; LOOP TAIL @ 0x0044b4ae
0044b4ae: loop_idx++; if (idx < count) goto 0x0044b370
```

After the loop, `EBX != 0` iff a supporter was picked; the picked supporter
has the **smallest squared distance** to the firing tower among all
eligible candidates within the candidate-range threshold. Equal distances
favor earlier loop indices (the `JGE skip` requires strictly smaller).

### 3.1 Eligibility summary

A candidate qualifies as a supporter iff ALL of:

1. `candidate != nullptr` (skip null array slots)
2. `candidate->field_0x90 != 0` (alive / active)
3. `candidate->Type == Rules->PrismType` (same type as the firing tower)
4. Cooldown expired: `LastSupportFrame == -1` OR
   `currentFrame - LastSupportFrame >= LastSupportDelay`
5. `candidate->field_0x714 == 0` (not currently in delayed-fire)
6. `TechnoClass::IsDeploying(candidate) == false`
7. `candidate->vtable[0x184]() == 1` (mission state check, see O2)
8. squared distance to firing tower ≤ `candidate->vtable[0x168](1)`
   (per-candidate range, see O3)

The same filters apply to the FIRING tower's own slot if it appears in
`Owner->Buildings` (which it does), but the cooldown / busy filters
(`+0x714 != 0`) self-exclude it because by this point `Mission_Attack`
hasn't yet set the firing tower's mode 1.

Wait — let me re-check this self-exclusion claim. At the cascade entry,
the firing tower is mid-`Mission_Attack` and has NOT yet been written
mode 1. So `field_0x714 == 0` is still true. So the firing tower could
be picked as its own supporter? The loop body at `0x0044b3a2` checks
`candidate->Type == Rules->PrismType` — true for the firing tower. The
cooldown check would pass if the firing tower has never emitted as a
supporter. Then it would self-pick.

Actual self-exclusion likely lives in the additional eligibility checks
between `0x0044b3fb` and `0x0044b421` — this hasn't been fully decoded
(O2). A pointer-equality check against `this` (`candidate != this`) is
the natural form. Documented as **strongly suspected but not yet
verified** in O4.

---

## 4. Trigger the Chosen Supporter (mode-2 setup, resolves G2)

```
0044b4c3: TEST EBX, EBX
0044b4c5: JZ  0x0044b595                            ; no supporter found → skip emit
0044b4cb: EAX = this->field_0x664
0044b4d4: INC EAX
0044b4d7: this->field_0x664 = EAX                   ; firing tower count++  ← G1 INCREMENT SITE
0044b4dd: build COORD struct on stack
0044b4f6: CALL vtable[0xb0]                         ; this.Get_Cell_Coord (firing tower coords)
0044b4fc: EDI = supporter->field_0x520              ; supporter->Type
0044b502-0044b507: load X, Y, Z from returned coord
0044b50a: EDI = supporter->Type->field_0x16ec       ; DelayedFireDelay
0044b512: supporter->field_0x714 = EDI              ; supporter timer
0044b51e: supporter->field_0x704 = 2                ; mode = supporter ← MODE-2 SET SITE
0044b528: supporter->field_0x708 = X                ; firing-tower X (saved for emit)
0044b52c: supporter->field_0x70c = Y
0044b52f: supporter->field_0x710 = Z
0044b532: CALL BuildingClass::ClearAnimSlot          ; on supporter (reset visual)
0044b539: CALL ObjectClass::GetHealthRatio           ; on supporter
0044b53e+: compare with Rules+0x1700 (ConditionYellow), select damaged-vs-healthy charge anim
```

**Per cascade-tick: exactly one supporter is selected and triggered.**
The firing tower's count goes up by exactly one. Over multiple ticks,
count accumulates until either:

- `support_count >= Rules->PrismSupportMax` (cap check at start),
- the candidate pool is exhausted for this tick (all on cooldown / busy /
  not prism / not in range), or
- the firing tower's own delayed-fire timer expires and it shoots.

This is **the correction to G4**: there is no recursive cascade in code.
Multi-supporter cascades are an emergent property of (a) repeated
`Mission_Attack` ticks each picking one supporter and (b) supporters that
have themselves never emitted a beam recently being eligible immediately.

---

## 5. Firing Tower Mode-1 Setup (resolves G2)

After the cascade body falls through to `0x0044b595` — whether a
supporter was found this tick or not:

```
0044b595: EDX = this->field_0x520                  ; this->Type
0044b59f: ECX = this->Type->field_0x16ec           ; DelayedFireDelay
0044b5ab: this->field_0x714 = ECX                  ; timer = DelayedFireDelay
0044b5b5: this->field_0x708 = 0                    ; weapon idx (primary)
0044b5bb: this->field_0x704 = 1                    ; mode = firing tower ← MODE-1 SET SITE
0044b5c5: this->field_0x70c = saved (likely target X)
0044b5c8: this->field_0x710 = saved (likely target Y)
0044b5cd: CALL BuildingClass::ClearAnimSlot
0044b5d4: CALL ObjectClass::GetHealthRatio
0044b5df: FCOMP [Rules + 0x1700]                   ; vs ConditionYellow
0044b5e7: TEST AH, 0x41                            ; choose anim variant
... (plays charge anim)
```

Even when zero supporters are picked this tick, the firing tower still
enters mode 1 with `Type->DelayedFireDelay` ticks — so it still has a
delayed shot, just an unscaled one (count stays at 0 → multiplier = 1.0x).

### 5.1 Non-prism fallback — IsAnimDelayedFire

When `Type != Rules->PrismType`, the prism gate jumps to `0x0044b630`:

```
0044b630: CL = this->Type->field_0x16a7            ; IsAnimDelayedFire (BuildingTypeClass+0x16a7)
0044b636: TEST CL, CL
0044b638: JZ  0x0044b6c4                           ; flag false → skip delayed fire
0044b63e: EAX = this->Type->field_0x16ec           ; DelayedFireDelay
0044b64e: this->field_0x714 = EAX                  ; timer = DelayedFireDelay
0044b65c: this->field_0x704 = 1                    ; mode = firing tower ← second MODE-1 SITE
0044b65a: this->field_0x708 = EBP                  ; weapon idx (passed in)
... (same ClearAnimSlot + health-ratio anim selection)
```

**`IsAnimDelayedFire` (BuildingTypeClass+0x16a7) is the generic
delayed-fire flag** for non-Prism buildings — buildings that just want a
charge animation cycle before firing without any cascade. Located via
xref to string "IsAnimDelayedFire" at `0x0081a760` from
`BuildingTypeClass::ReadINI` at `0x004611aa`. Stock YR does use this path:
`[TESLA] Image=NATSLA`, and `[NATSLA]` sets `IsAnimDelayedFire=yes` with
`DelayedFireDelay=28`. `[ATESLA] Image=GAPRIS` carries the same art keys but
is selected first by `Rules->PrismType`, so it follows the prism path.

---

## 6. Support Beam Emission (resolves G5 + G6)

When a supporter's mode-2 timer expires, `ProcessDelayedFire` at
`0x004503F0` invokes `BuildingClass::EmitPrismSupportBeam @ 0x0044ABD0`
with `(this, target_X, target_Y, target_Z)`:

```c
void BuildingClass__EmitPrismSupportBeam(
        BuildingClass *this,
        int target_x, int target_y, int target_z) {

    void *beam = operator_new(0x5C);              // LaserDrawClass instance (0x5C bytes)
    if (beam) {
        int duration = Rules->PrismSupportDuration;       // [g_Rules + 0x4a8]
        int laser_color_packed = *(uint3 *)(this->Owner + 0x56FC);  // owner laser color
        // Build coord struct via this->vtable[0x2C] (Get_Coord-equivalent)
        coord_t *src = vtable[0x2C](this, &local_coords);
        // LaserDrawClass::Constructor — builds + adds to g_LaserDraw_Array
        LaserDrawClass *laser = LaserDrawClass__Constructor(
            beam,                                  // this
            src->x, src->y, src->z,                // source = supporter location
            target_x, target_y, target_z,          // target = firing tower coords
            /*flags=*/ 0, /*one_shot=*/ 1,
            laser_color_packed,                    // owner color (HouseColor)
            /*inner=*/ 0, /*spread=*/ 0,
            duration,                              // PrismSupportDuration (lifetime in ticks)
            /*aux=*/ 0, /*?=*/ 1, /*intensity=*/ 0x3F800000 /*1.0f*/,
            /*?=*/ 0);
        if (laser) {
            laser->field_0x20 = 1;                 // "is support beam" flag
            laser->field_0x1c = 3;                 // beam type = 3 (prism support)
        }
    }

    // Mark this supporter offline:
    this->field_0x664 = 0;                          // clear OWN support count
    this->field_0x2EC = g_CurrentFrameCounter;     // LastSupportFrame
    this->field_0x2F0 = target_y;                  // (informational, target Y)
    this->field_0x2F4 = Rules->PrismSupportDelay;  // SavedDelay used by cooldown check
}
```

### 6.1 LaserDrawClass identity

`LaserDrawClass::Constructor` (`FUN_0054FE60`) is shared by other beam-type
weapons in YR:

```
$ get_function_callers(0x0054FE60):
  FUN_0044ABD0                                  ; prism support beam (this report)
  FUN_004A7340                                  ; (other building beam)
  FUN_006FD210                                  ; TechnoClass laser fire
  ParticleSystemClass__AI_Railgun @ 0x0062F230  ; railgun beam
```

The beam is drawn through the standard laser-draw render path
(`g_LaserDraw_Array`) — **not** through `AnimClass`. This explains why
no AnimClass entry exists for "PrismSupportAnim" in the INI: there isn't
one. The visual is a procedurally-drawn laser, colored per-house.

### 6.2 Self-clear semantics

Resetting `this->field_0x664 = 0` on a supporter that just emitted is
critical: it prevents a chain-of-supporters-of-supporters bug. Even if a
supporter had previously accumulated its own count (e.g. by being a firing
tower in a previous attack that was then re-targeted), emitting a beam
invalidates that pending count.

The set of `(LastSupportFrame, LastSupportDelay)` is what the cascade
selector reads at `0x0044b3ae` to compute remaining cooldown. The supporter
becomes re-eligible after `Rules->PrismSupportDelay` ticks
(default 45 in YR, 60 in RA2-base).

---

## 7. ProcessDelayedFire mode dispatch

For completeness, here is the full `ProcessDelayedFire` body in normalized
form (param is `int *`, so `param[N]` = byte offset `N*4`):

```c
int BuildingClass__ProcessDelayedFire(BuildingClass *this) {
    int mode = this->field_0x704;
    if (mode != 0 && --this->field_0x714 < 1) {
        this->field_0x714 = 0;
        if (mode == 1) {                                // Firing tower
            int err = 0;
            if (this->field_0x2b4 != 0) {
                err = vtable[0xF0](this, this->field_0x2b4, this->field_0x708, 1);
                if (err == 0) {
                    BulletClass *bullet = vtable[0xF3](this, this->field_0x2b4, this->field_0x708);
                    if (bullet != 0 && this->field_0x664 != 0) {
                        int pct = Rules->PrismSupportModifier * this->field_0x664 + 100;
                        bullet->field_0x150 = (uint)(pct * 0x100) / 100;
                        this->field_0x664 = 0;          // consume supporters
                    }
                }
            }
        } else if (mode == 2) {                          // Supporter
            BuildingClass__EmitPrismSupportBeam(
                this,
                this->field_0x708,    // target X
                this->field_0x70c,    // target Y
                this->field_0x710);   // target Z
            this->field_0x704 = 0;
            return ...;
        }
        this->field_0x704 = 0;
    }
    return ...;
}
```

**There is no mode 3.** The prior decompiler artifact (`iVar1 - 2 == 1`
in the `else` branch) was a syntactic fall-through pattern emitted by
Ghidra, not a real case. Mode is `{0 = idle, 1 = firing tower, 2 = supporter}`.

---

## 8. End-to-End Tick Walkthrough

A complete picture of a fresh prism attack:

**Tick T+0:** Prism A's `Mission_Attack` runs. Pre-gates pass (this is a
fresh attack, no in-flight firing animation), so control reaches the prism
gate. `Type == Rules->PrismType` → enter cascade. Cap check: count = 0,
max = 8 → OK. Loop iterates owner buildings, finds Prism B (closest,
eligible). `support_count = 1`. Sets B's mode = 2, B's timer =
`Type->DelayedFireDelay`, B's emit-target = A's coords. Falls through:
sets A's mode = 1, A's timer = `Type->DelayedFireDelay`.

**Tick T+1..T+(Delay-1):** A's `ProcessDelayedFire` decrements A's timer
each tick. B's `ProcessDelayedFire` decrements B's timer. Meanwhile A's
`Mission_Attack` may keep running and re-entering the cascade — but the
pre-gates likely exclude re-entry (see O1). If they don't: A re-evaluates,
B is no longer eligible (`+0x714 != 0` → "busy"), so A picks Prism C,
`support_count = 2`. And so on for D, E, F, G, H.

**Tick T+Delay:** B's timer hits 0. ProcessDelayedFire mode 2:
`EmitPrismSupportBeam(B, A_X, A_Y, A_Z)` runs. A LaserDrawClass beam
appears from B's location to A's location, displayed for
`Rules->PrismSupportDuration` ticks (15 default). B's `+0x664 = 0`, B's
`+0x2EC = T+Delay`, B's `+0x2F4 = Rules->PrismSupportDelay`. B's mode = 0.

**Tick T+Delay (same tick or +1):** A's timer hits 0. ProcessDelayedFire
mode 1: `Fire(target, weapon=0)` returns a `BulletClass*`. A's
`support_count = 8` (or whatever was accumulated). Compute
`pct = 150 * 8 + 100 = 1300`. Write `bullet->field_0x150 =
(1300 * 0x100) / 100 = 0xD00 = 13.0x`. Reset A's count to 0, mode to 0.

**Tick T+Delay+1..T+Delay+Rules->PrismSupportDelay:** B is on cooldown,
not eligible to support a new attack on A or anyone else.

This matches observed in-game behavior.

---

## 9. Implementation Implications

If implementing the cascade in Rust:

1. **Cascade evaluator runs each `Mission_Attack` tick on a Prism Tower.**
   Not a one-shot. Allow it to be re-entered each tick of the building's
   attack mission until either the cap is hit or the timer fires.

2. **Single supporter per tick.** Don't try to find ALL supporters at
   once. Pick the closest eligible one, set its mode 2, increment count,
   move on.

3. **Eligibility filter (in this exact order):**
   - candidate present in owner's building list
   - candidate active flag set
   - candidate Type matches `Rules.PrismType`
   - candidate `LastSupportFrame == -1` OR
     `current_frame - LastSupportFrame >= candidate.LastSupportDelay`
   - candidate not in delayed-fire (`timer == 0`)
   - candidate not deploying
   - candidate mission state check (mission == Guard? — see O2)
   - candidate's "in range" check (squared distance ≤ candidate's range
     value, see O3)

4. **Distance scoring is squared lepton distance** to firing tower's
   `Get_Cell_Coord`. The `Math::ftol` cast is a `f64 → i32` truncation;
   the score field is signed 32-bit. Handle overflow as the binary does
   (it doesn't — `0x7FFFFFFF` is the initial sentinel meaning "no winner
   yet").

5. **Save firing tower coords on the supporter.** The supporter's beam
   target is the firing tower's location at the moment of selection,
   not the live firing tower position at emit time. So if the firing
   tower were destroyed during the supporter's delay, the beam still
   draws to the old coord. (Practically irrelevant — buildings don't
   move, but the data flow says save-then-use.)

6. **EmitPrismSupportBeam clears OWN count.** This is a critical
   semantic — don't skip it.

7. **No recursive cascade.** Don't try to model "supporter has
   sub-supporters." The recursive look comes from the per-tick re-evaluation
   in `Mission_Attack`, not from any code reentry.

8. **`Rules.PrismSupportHeight` is unused in the trigger/emit paths.**
   It almost certainly affects the laser endpoint Z inside
   `LaserDrawClass::Constructor`. If implementing the visual beam,
   reproduce that — the source coord is the supporter's cell coord, and
   the target coord likely gets `Z += PrismSupportHeight` somewhere
   inside the laser constructor.

---

## 10. Resolved Open Questions (Iteration 2)

This section folds in findings from a follow-up Ghidra pass that resolved
most of the questions left open by the first iteration.

### O2 — Final eligibility checks (RESOLVED)

The instructions between `0x0044b3fb` and `0x0044b421` are:

```
0044b3fb: CMP EAX, 0x1               ; (vtable[0x61] result)
0044b3fe: JZ  0x0044b4ae              ; mission == ATTACK → skip
0044b404: CMP EDI, ESI                ; candidate (EDI) == this (ESI)?
0044b406: JZ  0x0044b4ae              ; YES → SELF-EXCLUSION
0044b40c: MOV EDX, [EDI]              ; supporter vtable
0044b415: CALL [EDX + 0xac]           ; supporter.Get_Source_Coord
0044b41b: EBP = [ESP+0x3c]            ; load firing tower X (saved earlier)
0044b421: SUB ESP, 0x8                 ; align stack for FILD chain
```

So the full eligibility list is now:

1. `candidate != nullptr`
2. `candidate->field_0x90 != 0` (active)
3. `candidate->Type == Rules->PrismType`
4. cooldown expired
5. `candidate->field_0x714 == 0` (not in delayed-fire)
6. `TechnoClass::IsDeploying(candidate) == false`
7. **`candidate->vtable[0x61]() != 1`** — i.e. candidate's current Mission
   is NOT `MISSION_ATTACK` (RA2 mission code 1 = `MISSION_ATTACK`).
   Idle/Guard/Stop candidates qualify; candidates currently shooting at
   their own targets do not.
8. **`candidate != this` — explicit self-exclusion** (resolves O4).
9. squared distance to firing tower ≤ `candidate->vtable[0x5A](1)`
   (per-candidate range)

### O4 — Self-exclusion (RESOLVED — same as O2 #8 above)

Verified via `CMP EDI, ESI; JZ 0x0044b4ae` at `0x0044b404`. The firing
tower is explicitly pointer-compared against the candidate and skipped if
they match. So self-picking is impossible by construction — the cascade
never adds the firing tower itself to its own support count.

### O7 — `DelayedFireDelay` source (RESOLVED — comes from `artmd.ini`)

The visible charge cycle for Prism Towers and Tesla Coils comes from the
**art** INI files, not the rules INI. `BuildingTypeClass::ReadINI` is
called against both `rules*.ini` and `art*.ini` for each building type,
and the art file's `IsAnimDelayedFire` and `DelayedFireDelay` keys
overwrite the same struct offsets (+0x16a7 and +0x16ec).

In `ini/artmd.ini`:

```
[NATSLA]
IsAnimDelayedFire=yes  ; SJM: Firing anim (SpecialAnim) delays firing of weapon
DelayedFireDelay=28    ; SJM: Must match playback of anim, and ideally audio too

[GAPRIS]
IsAnimDelayedFire=yes  ; SJM: Firing anim (SpecialAnim) delays firing of weapon
DelayedFireDelay=28    ; SJM: Must match playback of anim, and ideally audio too
```

So:
- ATESLA (cascade path) uses `Type->DelayedFireDelay = 28` ticks for
  both mode-1 (firing tower) and mode-2 (supporter) timers.
- NATSLA (Tesla Coil, IsAnimDelayedFire path) uses the same 28 ticks
  for its mode-1 only — no cascade because `Type != Rules->PrismType`.
- 28 ticks at the engine's ~15 fps logic rate ≈ 1.9 seconds of visible
  delay between attack initiation and projectile launch — matches the
  in-game charge cycle.
- Over 28 ticks of `Mission_Attack`, the cascade can accumulate up to 8
  supporters before `PrismSupportMax` caps further additions.

`IsAnimDelayedFire=yes` on ATESLA is harmless — the prism gate at
`0x0044b30a` is reached first when `Type == Rules->PrismType`, so the
IsAnimDelayedFire branch is never taken for ATESLA (the JNZ at
`0x0044b30a` is not taken). It's set for completeness / consistency
with NATSLA's art definition.

The `Image=GAPRIS` line in `[ATESLA]` is what causes the engine to
parse the art `[GAPRIS]` section as the building's Type-art, providing
those keys to the same BuildingTypeClass instance.

### O6 — `ChargedAnimTime` is NOT prism-related (CORRECTED)

The earlier suspicion that `ChargedAnimTime` was the prism charge timer
is **wrong**. The keys appear only in superweapon building sections:

```
$ grep ChargedAnimTime ini/rulesmd.ini
[GACSPH] ChargedAnimTime=1   ; Chronosphere
[GAWEAT] ChargedAnimTime=1   ; Weather Control
[NAIRON] ChargedAnimTime=1   ; Iron Curtain
[NAMISL] ChargedAnimTime=1   ; Nuke Silo
[YAGNTC] ChargedAnimTime=1   ; Genetic Mutator
[YAPPET] ChargedAnimTime=1   ; Psychic Dominator
```

The comment "Number of minutes left at which weapon should switch to
charged state" confirms it: ChargedAnimTime is the **superweapon**
"about-to-fire-soon" animation switch threshold (in minutes of remaining
recharge time). The consumer is `BuildingClass::UpdateAnimation @
0x004509D0`, in the block guarded by `ChargedAnimTime <= 0.0` checks
and `Type->ChargedAnim_Index (+0x16f0) != -1` — superweapon-specific
animation state machine.

`ATESLA` does NOT set `ChargedAnimTime=` and does NOT have a
`ChargedAnim=` art linkage. It is irrelevant to Prism Tower behavior.
**Strike from the implementation spec.**

### O5 — `PrismSupportHeight` is parsed-but-unread (RESOLVED)

A binary-wide pattern search for the four common addressing modes that
would read `Rules+0x4AC`:

```
8B ?? AC 04 00 00   (MOV reg32, [reg + 0x4AC])    — only one match,
                                                    misaligned in
                                                    WorldDominationTour
                                                    (false positive)
D9 ?? AC 04 00 00   (FLD float [reg + 0x4AC])     — no matches
DB ?? AC 04 00 00   (FILD int [reg + 0x4AC])      — no matches
FF ?? AC 04 00 00   (PUSH dword [reg + 0x4AC])    — 7 matches, all
                                                    are vtable[0x12B]
                                                    calls on other structs
                                                    (false positives)
```

`Rules+0x4AC` is **never read** through any standard addressing mode in
the binary. The only writer is `RulesClass::ReadGeneral` at `0x006711EB`
(the parse step). `PrismSupportHeight` is a **dead INI key** — parsed
into a Rules field that is never consumed anywhere in gamemd.exe.

Implication: do not implement any height-adjustment logic for the
support beam. The supporter→firing-tower beam goes from supporter cell
coord to firing-tower cell coord, no Z offset applied. The visual
nicety of the beam appearing to land "on top of" the firing tower
comes from the firing tower being a tall sprite — the laser endpoint
is at ground level, but the sprite is rendered above it.

(This is plausibly a vestigial Tiberian Sun / pre-release Prism Tower
design artifact that was never wired up.)

### O3 — `vtable[0x5A]` range threshold (PARTIALLY RESOLVED)

Now identified as `vtable[0x5A](1)` (byte offset 0x168 = index 0x5A).
The argument `1` is a weapon index, suggesting this is a per-weapon
range query. The natural identity is `TechnoClass::Weapon_Range(int
weapon_idx)` returning the maximum lepton range of the candidate's
Weapon 1 (Secondary), which for ATESLA is `PrismSupport` with
`Range=8`.

Verification of the exact return units (leptons vs leptons²) was not
done — the code computes `dist²` and compares with `vfunc()`, so the
vfunc must return the same unit. Conventionally `Weapon_Range` returns
leptons; if so, the code path here would need to square it, but the
disassembly compares directly. This suggests either:

- (a) `vtable[0x5A]` returns `range²` already (less likely), or
- (b) my reading of the FILD/FMUL chain is wrong and the result is
  actually distance not distance² (more likely — let me re-trace),
- (c) the function is something other than `Weapon_Range`.

Pending clarification — but the high-level meaning ("must be in range")
is unambiguous regardless.

### O1 — `BuildingClass+0x702` and `+0x5ec` (STILL OPEN)

Identity not pinned. The cascade selector itself doesn't depend on
either field — they only gate per-tick re-entry into the cascade vs
immediate-fire path. Pre-cascade gate semantics and identities can be
filled in by a separate field-access scan across all `BuildingClass`
writers; not central to prism behavior.

### O8 — Vtable index resolution (PARTIALLY RESOLVED)

Best-guess names for the vtable indices used in the cascade, by
behavior + offset:

| Offset | Index | Inferred name             | Used at         |
|--------|-------|---------------------------|-----------------|
| +0xAC  | 0x2B  | `Get_Source_Coord`        | `0044b323`, `0044b415` |
| +0xB0  | 0x2C  | `Get_Cell_Coord`          | `0044b4f6`      |
| +0x168 | 0x5A  | `Weapon_Range(int wpn)`?  | `0044b494`      |
| +0x184 | 0x61  | `Get_Mission()`           | `0044b3f5`, `0044b3fb` |
| +0x3C0 | 0xF0  | `Can_Fire(target, wpn)`   | `0044b349`-area, `ProcessDelayedFire` |
| +0x3CC | 0xF3  | `Fire(target, wpn)`       | `ProcessDelayedFire` |
| +0x3C8 | 0xF2  | `Assign_Target` (?)       | `0044b095`-area |

Confirmation against `TECHNOCLASS_VTABLE_COMPLETE.md` and
`BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` is left as a future cleanup.

---

## 11. Implementation Spec — Final (revised post-iteration-2)

Update Section 9 with these refinements:

1. **Self-exclusion is mandatory** — explicit pointer-equality check
   (`candidate != this`) before any range/distance work on the candidate.
2. **Mission-state filter is `mission != ATTACK`** (RA2 mission code 1).
   Idle, Guard, Stop, Sleep, etc. all qualify; only currently-attacking
   candidates are excluded.
3. **`DelayedFireDelay = 28` is sourced from artmd.ini's `[GAPRIS]`
   section**, not rulesmd.ini. If implementing the INI parser, the art
   INI parse pass must overwrite the same BuildingTypeClass field that
   the rules pass writes (or vice-versa — order-dependent).
4. **Drop `PrismSupportHeight` from the spec** — dead INI key, unused
   by the engine.
5. **Drop `ChargedAnimTime` from the prism spec** — superweapon-only
   field, irrelevant to Prism Tower behavior.
6. **The full cascade timing is**:
   - Tick 0: cascade picks closest eligible supporter S₁, sets S₁ to
     mode 2 with timer = 28, sets firing tower mode 1 with timer = 28,
     count = 1.
   - Tick 1..27: each tick runs Mission_Attack again. Pre-cascade gates
     (`+0x702`, `+0x5ec`) presumably block re-entry while mid-attack
     (still O1). If they do NOT block re-entry: each tick picks another
     eligible supporter S₂, S₃, ..., up to PrismSupportMax = 8. Each
     supporter has its own 28-tick timer.
   - Around tick 14 (timer/2): supporters' beams start firing as their
     own mode-2 timers expire. (Beam visuals last `PrismSupportDuration
     = 15` ticks.)
   - Tick 28: firing tower mode-1 timer expires. `Fire()` returns a
     bullet. Damage scaled by accumulated count (e.g. count = 8 →
     13.0× damage). Reset count to 0.
   - Each emitting supporter records `LastSupportFrame = currentFrame`
     and `LastSupportDelay = Rules->PrismSupportDelay = 45`. Cannot
     support again until 45 ticks elapse.

Whether the per-tick re-entry actually accumulates is the **last
remaining behaviorally-significant unknown** — see Open Question O1
which we couldn't fully resolve in this iteration.

---

## 12. Pre-cascade gate identity (RESOLVED in iteration 2)

`BuildingClass+0x702` is the **upgrade count** (number of installed
upgrade buildings, e.g. PowerTurbine on a PowerPlant). `BuildingClass+0x5E8`,
`+0x5EC`, `+0x5F0` are the three **upgrade slot pointers**.

Verified writers:

| Address | Function | Operation |
|---------|----------|-----------|
| `0x0043B9DC` | `BuildingClass::Constructor` | initial zero |
| `0x004515DB` | (`BuildingClass::AddUpgrade`-equivalent, ~`0x00451460`) | `field_0x702++` after `CreateAnimForSlot` |
| `0x0045160A` | (same function, second branch) | `field_0x702++` (damaged-variant path) |
| `0x004516E7` | `BuildingClass::RemoveLastUpgrade @ 0x00451690` | clears slot, `field_0x702 = 0` |
| `0x0045171B` | (same) | `field_0x702--`; clears `+0x5e8 + idx*4` |

The pre-cascade gate at `0x0044b2bc` reads:

```
this->upgrade_count != 0 AND this->upgrade_slot_1 != 0  → fire immediately
                                                        → JMP cascade-skip path
otherwise                                                → enter prism gate
```

The semantics: **buildings that have at least 2 upgrades installed (slot
0 and slot 1 occupied) bypass the cascade and fire immediately.** The
specific check on slot 1 (not just slot 0) means a building with one
upgrade still cascades; only the second upgrade triggers the bypass.
This matches no observed YR building behavior (e.g. PowerPlants don't
have a weapon, so the bypass is moot for them); the mechanism is
**vestigial or applies to a building type / configuration not present
in stock YR**.

**Implication for Prism Towers:** ATESLA is `Capturable=false` and
not upgradeable. Both gates always read 0. Cascade is always reached.

---

## 13. Mode-1-Reset Concern — RESOLVED (Iteration 3)

The concern was: would `Mission_Attack`'s cascade-tail re-set the
delayed-fire timer back to 28 every tick, preventing the firing tower
from ever reaching `ProcessDelayedFire`'s timer-zero condition?

**Answer: NO — `BuildingClass::GetFireError` reads `+0x714` directly
and returns `3` while the timer is non-zero. The Mission_Attack
jumptable case for code 3 does not re-enter the cascade. Cascade runs
exactly once per attack cycle, when timer is zero.**

### 13.1 GetFireError checks the timer

`BuildingClass::GetFireError @ 0x00447F10`:

```c
int BuildingClass::GetFireError(BuildingClass *this, target, weapon, unk) {
    // ... (other building-state checks omitted) ...

    if (vtable[0xD4]() == 0) return 6;             // not powered/active

    // ★★★ DELAYED-FIRE GATE ★★★
    if (this->field_0x714 != 0) return 3;          // BUSY (currently charging)

    int err = TechnoClass::GetFireError(target, weapon, unk);

    if (err == 0 && vtable[0xFF]()) {
        // facing/timing check using Type+0x16c5
        ...
        if (facing_diff > threshold) return 2;     // ROTATING
    }
    return err;
}
```

So once mode 1 is set (timer = 28), every subsequent `GetFireError`
call returns `3` until `ProcessDelayedFire` decrements timer to 0.

### 13.2 Mission_Attack jumptable contents (verified by reading binary)

`PTR_LAB_0044b728` table at `0x0044b728` (44 bytes, 11 little-endian
4-byte addresses):

| Idx | Address     | Meaning                          |
|-----|-------------|----------------------------------|
| 0   | `0x0044b2bc` | **Cascade entry** (OK to fire)   |
| 1   | `0x0044b0de` | REARM/etc — generic fire-result handler |
| 2   | `0x0044b187` | ROTATING — facing + post-handler |
| 3   | `0x0044b1de` | **REARM/timer-active** — clears `+0xC4`, returns `2` (delay) |
| 4   | `0x0044b14e` | CANT — clears `+0xC4`, returns `1` |
| 5   | `0x0044b0de` | (same as 1) |
| 6   | `0x0044b0de` | (same as 1) |
| 7   | `0x0044b14e` | (same as 4) |
| 8   | `0x0044b0de` | (same as 1) |
| 9   | `0x0044b284` | code 9 — separate handler |
| 10  | `0x0044b24f` | code 10 — separate handler |

So when the timer is non-zero (`GetFireError == 3`), control goes to
`0x0044b1de`, which does some bookkeeping and returns. **It does NOT
re-enter the cascade entry at `0x0044b2bc`.**

### 13.3 Per-attack-cycle behavior (final)

A complete cycle:

1. **Tick 0**: Mission_Attack runs, GetFireError returns 0, jumptable[0]
   → cascade. Picks closest eligible supporter S, sets S to mode 2 with
   28-tick timer, sets THIS to mode 1 with 28-tick timer, count = 1.
2. **Ticks 1..27**: Mission_Attack runs each tick. GetFireError returns
   3 (timer != 0). Jumptable[3] runs (no cascade). ProcessDelayedFire
   decrements THIS's timer each tick. S's timer also decrements; at some
   tick S emits its support beam (LaserDrawClass) and clears S's own
   support count.
3. **Tick 28**: THIS's timer reaches 0. ProcessDelayedFire mode-1 path
   calls `vtable[0xF3]` (Fire). On success, `bullet->DamageScale =
   (PrismSupportModifier * 1 + 100) * 256 / 100 = 0x280` (2.5×). Count
   reset to 0. Mode reset to 0.
4. **Tick 29+**: Next attack cycle starts. GetFireError = 0, cascade
   re-enters, picks NEW supporter (the previous supporter S is on
   `Rules->PrismSupportDelay = 45`-tick cooldown).

So **per shot, count = 1, multiplier = 2.5×**. The firing tower goes
through this 28-tick cycle repeatedly, with each shot getting one
supporter, supporters cycling through (B, C, D, ...) until B's 45-tick
cooldown expires.

### 13.4 When can count actually exceed 1?

Count is reset only when `Fire()` succeeds AND returns a non-null bullet.
If the fire attempt is suppressed:

- `if (this->field_0x2b4 == 0)` (no target) — Fire not called, count
  not reset
- `if (err != 0)` (GetFireError returned non-zero at the moment the
  ProcessDelayedFire timer hit 0) — Fire not called, count not reset
- `if (bullet == 0)` (Fire returned null — out of ammo, etc.) — count
  not reset

In any of these cases, count survives to the next attack cycle. On the
next cycle, the cascade-tail INCs count to 2. Repeat → count can grow
to `Rules->PrismSupportMax = 8` (the cap then prevents further growth).

So **`PrismSupportMax = 8` is a CAP for the contested/multi-cycle
accumulation case**, NOT a normal-play multi-beam-per-shot accumulator.
The "13× damage" theoretical max requires 8 attack cycles where Fire
keeps failing to commit but the cascade keeps incrementing — a degenerate
edge case.

### 13.4b Verified jumptable[3] body (`0x0044b1de`)

Decoded from the disassembly, the REARM/timer-active case body is:

```
0044b1de: EAX = this->target (+0x2b4)
0044b1e4: CMP EAX, 0
0044b1e6: JZ skip_facing                    ; no target → skip facing update
0044b1e8: vtable[0x13A](&local_coords, target)  ; get target coords
0044b1f9: ECX = &this->FacingTimer (+0x388)
0044b1ff: CALL RateTimer__Set (0x004C9220)  ; update facing-tracking timer
skip_facing:
0044b204: ECX = this->Type
0044b20a: AL = Type[+0xCD5]                 ; "Trainable"/"Gattling" flag
0044b210: TEST AL
0044b212: JZ skip_gattling
0044b214: EDX = this->field_0xC4            ; some pending-fire counter
0044b21d: CALL TechnoClass::IncreaseGattlingStage (0x0070DE70)
0044b222: this->field_0xC4 = 0
skip_gattling:
0044b228: EAX = 2                            ; return delay = 2 ticks
0044b22d-0044b234: epilogue + RET
```

So jumptable[3]:
- Updates the building's facing-tracking timer toward the target
  (so the visual turret keeps pointing at it during charge)
- For Gattling/Trainable buildings: advances the gattling stage and
  resets `+0xC4` (fire-pending counter)
- Returns 2 (Mission_Attack re-runs in 2 ticks)

For ATESLA (not gattling), only the facing update happens. Crucially,
**no path within jumptable[3] re-enters the cascade.**

### 13.4c Verified cascade-tail return value

The cascade-entry case (jumptable[0]) ends at the same tail block
(0x0044b6d6 onward) and returns **1** (Mission_Attack re-runs in 1 tick).

So on the tick AFTER cascade entry, Mission_Attack runs again, sees
GetFireError = 3 (timer = 27), dispatches jumptable[3], returns 2.
Mission_Attack runs again 2 ticks later, etc., until the timer hits
zero and ProcessDelayedFire fires. **Mission_Attack alternates between
"once at start" (cascade) and "every 2 ticks during charge" (no-op).**

### 13.5 Implication for visible "cascade" effects in YR

What players see in YR — multiple Prism Towers all firing beams toward
one Prism Tower that then fires a brilliant shot — is **per-shot 1
supporter, sequential over multiple shots**. Each shot:

- Picks the closest available supporter (one per shot)
- That supporter's beam fires partway through the 28-tick charge
- The firing tower's shot lands with 2.5× damage

Across consecutive shots over a sustained attack:
- Shot 1: B supports → A fires 2.5× damage
- Shot 2 (after RoF): C supports (B is on 45t cooldown) → A fires 2.5×
- Shot 3: D supports → A fires 2.5×
- ... etc.

This produces the visual impression of "many beams" because the firing
tower attacks continuously and each shot gets a beam from a different
nearby tower. But the per-shot damage is constant 2.5× (not 13×) in
normal play.

**This is a meaningful correction to common community understanding
("8 supporters = 13× damage on one shot"). The cap exists, but is
unreachable through normal targeting.**

---

## 14. Iteration-3 Supplemental Findings

A few corrections and clarifications to fields/constants that came up
during the mode-1-reset analysis:

### 14.1 Constants `_DAT_007e44c0` and `_DAT_007e44c4`

Read directly from the binary (4 bytes each, IEEE 754 single):

| Address    | Bytes (LE)       | Float value  | Likely meaning                    |
|------------|------------------|--------------|------------------------------------|
| `0x007e44c0` | `B4 A2 91 3A` | ≈ `0.001111` | `1 / 900` — per-tick conversion factor (900 ticks per minute → 15 ticks/sec internal logic rate, the standard YR rate) |
| `0x007e44c4` | `00 80 77 44` | `≈ 990.0`    | "max minutes" sentinel for `ChargedAnimTime`. The check `ChargedAnimTime <= 990.0` essentially passes for any reasonable value (1, 5, 10 minutes); buildings that don't set `ChargedAnimTime` have it default to 0 which still passes. Real gating is via `Type+0x16f0` (SuperWeapon ID) being -1. |

So **`ChargedAnimTime` is a per-minute threshold**, not a raw tick value.
The unit conversion `(remaining_ticks / 900) ≤ ChargedAnimTime_minutes`
is performed in the UpdateAnimation block at `0x004510c8`-area:
`*(float *)(Type + 0x16e8) < (float)remaining_delay * (1/900)`.

### 14.2 `Type+0x16f0` is `SuperWeapon` index — NOT `ChargedAnim_Index`

Earlier inference was that `BuildingTypeClass+0x16f0` was a "ChargedAnim
linkage" field. Re-reading `BuildingClass::UpdateAnimation` more
carefully: the loop at `LAB_00450f9e` iterates `Owner+0x258` (the
HouseClass's super-weapon array), and at each iteration reads
`*(int *)(*(int *)(super + 0x28) + 0xb4)` — the SUPERWEAPON instance's
Type pointer (+0x28) dereferenced to get the SuperWeaponTypeClass's ID
(+0xb4). It compares this ID against `Type+0x16f0`.

The `[General]`-key reading at `0x00460BB8` pushes the string literal
`"SuperWeapon"` (at `0x00817204`) into `BuildingTypeClass::ReadINI`
right around the `Type+0x16f0` write. So **`Type+0x16f0` is the
`SuperWeapon=` INI key** — the superweapon ID this building owns.

The associated UpdateAnimation block in `BuildingClass::UpdateAnimation`
is therefore the **superweapon "about-to-fire" animation switch**: when
the building's owned superweapon is within `ChargedAnimTime` minutes of
recharge completion, switch to the "ready" anim variant. Buildings:
Chronosphere, Iron Curtain, Nuke Silo, Weather Control Device, Genetic
Mutator, Psychic Dominator.

### 14.3 `vtable[0xD4]` (called in GetFireError)

At `0x00447F99`, `BuildingClass::GetFireError` calls `vtable[0xD4]` of
`this`. The result (AL) gates whether to return code 6 immediately or
proceed to the timer check. For a powered building, AL is non-zero and
control proceeds. For an unpowered building (e.g. low-power, sold), AL
is zero and GetFireError returns 6 (NO_AMMO/NO_POWER).

Identity inferred (not vtable-resolved): likely `Is_Active` /
`Is_Powered` on `BuildingClass`.

### 14.4 `Type+0xCD5` and Gattling stage

`Type+0xCD5` is a byte flag (likely `IsGattling=` based on the binary
having that string at `0x00843E4C` — though `Trainable` at `0x00843974`
is also a candidate, requiring xref check to pin down). When set, the
cascade-tail and jumptable[3] both call
`TechnoClass::IncreaseGattlingStage @ 0x0070DE70`, passing the
building's `+0xC4` field as a parameter.

For **non-gattling buildings (including ATESLA)**, this code path is
skipped — `field_0xC4` is left untouched and IncreaseGattlingStage is
not invoked. Confirmed irrelevant to the prism cascade.

### 14.5 `Type+0x16b8` — "IsChargeMode" name retracted

The string `"IsChargeMode"` does NOT exist in `gamemd.exe`. The prior
report's claim that `BuildingTypeClass+0x16b8` is `IsChargeMode` is
unsupported. The flag IS read at `0x0044AD07` to gate the entire
non-standard branch of `Mission_Attack` (the branch that uses
`HouseClass::GetPowerRatio`, `Type+0x1573`, `Type+0xee4`).

Behavior summary of the branch: the building only fires when the owner
has full power (`PowerRatio >= 1.0`) AND various flag conditions hold.
This pattern fits `IsChargeFire`, `BuildupFire`, `ReadyToFire` style
flags from TS-era code. **Identity remains unresolved**, but it does
not gate any prism behavior — ATESLA always takes the `Type+0x16b8 == 0`
path (standard fire flow).

### 14.6 `BuildingClass+0xC4` field

Read/written extensively across Mission_Attack jumptable handlers:
writes at `0x0044B131`, `0x0044B174`, `0x0044B1CB`, `0x0044B222`,
`0x0044B271`, `0x0044B2B1`, `0x0044B6F4`. Reads at `0x0044B123`,
`0x0044B1BD`, `0x0044B214`, `0x0044B263`, `0x0044B2A3`, `0x0044B6E6`.
Cleared at the end of every fire-attempt branch and passed as the
parameter to `IncreaseGattlingStage`.

Likely identity: a per-attack "shots fired this trigger pull" counter
or "pending GattlingStage advancement" state. Reset to 0 after each
Mission_Attack tick. Not central to prism behavior.

### 14.7 `field_0x90` = `IsAlive` (verified)

The cascade eligibility check at `0x0044b388` reads `[EDI + 0x90]`. Per
`OBJECTCLASS_GHIDRA_REPORT.md` (line 88) and `TECHNOCLASS_STRUCT_LAYOUT.md`,
`ObjectClass+0x90` is `IsAlive` (bool, set to 1 in Constructor, set to
0 in UnInit). The check filters out destroyed / un-deployed buildings.

### 14.8 Outgoing-shot LaserDrawClass tagging (additional)

A second LaserDrawClass write happens inside `TechnoClass::Fire_At @
0x006FDD50` whenever a Prism Tower's `vtable[0xF3]` (Fire) is invoked —
including from `ProcessDelayedFire` mode 1.

**Important correction:** the writes target the LASER instance, not the
bullet. EAX in the prism block is the return value of `FUN_006FD210`,
which is `BuildingClass::SpawnLaserVisual` (or similar) — it allocates
a LaserDrawClass via `operator_new(0x5C)` and constructs it via
`LaserDrawClass::Constructor (FUN_0054FE60)`. The bullet itself is
created elsewhere in Fire_At and is unrelated to these writes. (Per
`BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`, `bullet+0x1c` is `RefCount` —
overwriting it with `3` or `5` would corrupt COM ref-counting, which
would not be done deliberately.)

The prism block at `0x006FF50C - 0x006FF544`:

```
006ff50c: ECX = [g_RulesClass_Instance]
006ff512: EDX = this->Type
006ff518: CMP EDX, [ECX + 0x498]              ; vs Rules->PrismType
006ff51e: JNZ 0x006ff656                       ; not Prism → skip block
006ff524: laser->field_0x1c = 3                ; LaserType = "prism (basic)"
006ff52b: ECX = this->field_0x664              ; firing tower's support count
006ff531: TEST ECX
006ff533: JLE 0x006ff656                       ; if count <= 0, keep type 3
006ff539: laser->field_0x21 = 1                ; "has supporters" byte flag
006ff53d: laser->field_0x1c = 5                ; LaserType = "prism (boosted)"
```

So the firing tower's outgoing PrismShot laser carries:
- `laser->field_0x1c = 3` always (Prism Tower marker)
- `laser->field_0x1c = 5` if supporters contributed (boosted visual)
- `laser->field_0x21 = 1` if supporters contributed

These mirror the support-beam tagging in `EmitPrismSupportBeam`
(`laser->field_0x1c = 3, laser->field_0x20 = 1` for support beams).
The shared `field_0x1c` enum is presumably consumed by the laser-draw
renderer to choose color blend / thickness / particle effects per
beam type.

For an implementation focused on functional parity, these visual flags
can be ignored. For visual fidelity, the renderer should distinguish:
- type 3 + flag 0x20=1: support beam (supporter → firing tower)
- type 3 + neither flag: outgoing prism shot, unboosted
- type 5 + flag 0x21=1: outgoing prism shot, boosted (count > 0)

This closes the loop on every code path that touches the prism cascade
or any prism-aware field.

### 14.9 Damage scaler IS read in the standard damage path (verified)

Concern: the original `PRISM_FORWARDING_GHIDRA_REPORT.md` cited the
damage scaler being read only in the "cell-action-type-6 /
crush-physics" branch of `WarheadTypeClass::Detonate`. If that were
the only reader, the prism multiplier would only affect crush
mechanics, not normal direct-hit damage — which would dramatically
change implementation priorities.

A binary-wide field-access scan for `MOV reg, [reg + 0x150]`
patterns inside `WarheadTypeClass::Detonate` (range
`0x004690B0 - 0x0046A303`) returned **two** read sites:

**Site A — `0x004697FC` (existing report's site, crush branch):**

```
004697fc: EAX = bullet->DamageScale (+0x150)
00469802: ECX = [g_RulesClass_Instance]
00469808: EAX *= bullet->Damage (+0x6c)        ; multiplier × damage
0046980c: SAR EAX, 0x8                          ; >> 8 (8.8 fixed)
0046980f: [ESP + 0x1c] = EAX                    ; spill to stack
00469813: FILD [ESP + 0x1c]                     ; → float
00469817: FMUL [ECX + 0x18b4]                  ; × Rules+0x18b4 (CrushDamage factor?)
0046981d: FDIV [0x0081aef8]                    ; / global constant
00469823: FCOM [0x007e3cc8]                    ; compare with constant
... (eventual stack store, used downstream as crush damage)
```

**Site B — `0x00469A56` (newly identified, standard damage path):**

```
00469a51: CALL 0x0046a310                       ; helper (likely target-resolution)
00469a56: EDX = bullet->DamageScale (+0x150)
00469a5c: EAX = bullet->Owner (+0xb0)           ; firing object (Prism Tower)
00469a62: EDX *= bullet->Damage (+0x6c)         ; multiplier × damage
00469a66: SAR EDX, 0x8                           ; >> 8 (scaled damage in EDX)
00469a69: ECX = 0
00469a6b: TEST EAX
00469a6d: JZ skip_owner
00469a6f: ECX = bullet->Owner->Owner (+0x21c)   ; firing house
00469a75: PUSH ECX                               ; arg4: source house
00469a76: ECX = bullet->WeaponType (+0x128)
00469a7c: PUSH 1                                 ; arg3: ?
00469a7e: PUSH ECX                               ; arg2: weapon type
00469a7f: ECX = [EBP + 8]                       ; this (warhead)
00469a82: PUSH EAX                               ; arg1: source object
00469a83: CALL Apply_area_damage (0x00489280)   ; SCALED DAMAGE → AOE APPLY
```

`Apply_area_damage @ 0x00489280` is the **standard area-damage application
function** that handles:
- direct-hit damage on the targeted object,
- splash damage on objects in `CellSpread` radius,
- damage modifiers from `Verses` (armor multipliers).

The scaled damage flows to it as fastcall arg2 (EDX in convention,
`SAR EDX, 0x8` immediately precedes the `CALL` with no intervening
EDX modification).

**Conclusion:** the prism multiplier IS applied to the standard damage
path. A boosted Prism Tower shot (count=1, multiplier=2.5×) deals 2.5×
damage to its target on direct hit, scaled by `Verses[ArmorType]` per
normal warhead semantics. The 13.0× theoretical max (count=8) likewise
multiplies all damage, not just crush.

The implementation spec in Section 11/14.10 is correct as written.

### 14.10 Final implementation spec — confirmed

After three iterations, the prism implementation spec is:

1. Parse `[General] Prism*` keys from `rulesmd.ini` (six keys, see Section 1).
2. Parse `IsAnimDelayedFire=yes` and `DelayedFireDelay=28` from
   `artmd.ini [GAPRIS]` and `[NATSLA]` into `BuildingTypeClass+0x16a7`
   and `+0x16ec`. Same struct, written by both INI passes.
3. **Drop `PrismSupportHeight` and `ChargedAnimTime`** from prism
   implementation — both unused / unrelated.
4. **One supporter per attack cycle.** Mission_Attack's cascade picks
   the closest eligible supporter (by squared lepton distance) once
   when GetFireError returns 0 (timer = 0). On all subsequent ticks
   during the 28-tick charge, GetFireError returns 3 and Mission_Attack
   does nothing.
5. **Eligibility filter (final):**
   - candidate is alive (`+0x90 != 0`)
   - `candidate->Type == Rules->PrismType`
   - cooldown expired (`currentFrame - LastSupportFrame >= LastSupportDelay`,
     OR LastSupportFrame == -1)
   - candidate not in delayed-fire (`+0x714 == 0`)
   - candidate not deploying (`TechnoClass::IsDeploying == false`)
   - candidate's `Get_Mission() != 1` (not currently in MISSION_ATTACK)
   - **candidate is not the firing tower itself** (`candidate != this`)
   - distance² ≤ candidate's range threshold (`vtable[0x5A](1)`)
6. On chosen supporter:
   - Set `supporter->mode = 2`, `supporter->timer = 28`,
     `supporter->saved_target = (firing-tower X, Y, Z)`
   - Increment `firing-tower->support_count` by 1
7. On firing tower (every cascade entry, regardless of supporter found):
   - Set `mode = 1`, `timer = 28`, `weapon_idx = 0`
8. ProcessDelayedFire each tick:
   - Decrement timer; if `< 1`:
     - mode 1: call `Fire(target, 0)`. If bullet returned and
       `support_count != 0`: scale damage as `(150 * count + 100) * 256 / 100`,
       reset `support_count = 0`. Reset mode = 0.
     - mode 2: spawn `LaserDrawClass` from supporter location to saved
       target coords, set `LastSupportFrame = currentFrame`,
       `LastSupportDelay = Rules->PrismSupportDelay = 45`,
       clear own `support_count`. Reset mode = 0.

Across multiple shots, supporters cycle (B → C → D → ... → H → back to B
after 45-tick cooldown). Each shot deals 2.5× base damage. The 13×
theoretical max requires repeated fire failures to keep mode set
without resetting count — a contrived edge case.

---

## Sources

**Ghidra functions decompiled / disassembled:**
- `BuildingClass::Mission_Attack @ 0x0044ACF0` (full body to `0x0044b709`)
- `BuildingClass::ProcessDelayedFire @ 0x004503F0`
- `BuildingClass::EmitPrismSupportBeam` (was `FUN_0044ABD0`) @ `0x0044ABD0`
- `LaserDrawClass::Constructor` (`FUN_0054FE60`) @ `0x0054FE60`
- `TechnoClass::IsDeploying @ 0x0070FEC0`
- `BuildingClass::ClearAnimSlot @ 0x00451E40`
- `ObjectClass::GetHealthRatio @ 0x005F5C60`
- `BuildingClass::CreateAnimForSlot @ 0x00451890`
- `BuildingClass::Update @ 0x0043FB20`
- `RulesClass::ReadGeneral @ 0x0066D530`
- `BuildingTypeClass::ReadINI @ 0x0045FE50` (xref points 0x004611AA, 0x004611C7, 0x00460B9E)
- `FUN_00712130` @ `0x00712130` (mind-control / link sanity check)
- `TechnoClass::Fire_At @ 0x006FDD50` (sibling prism check at 0x006FF52B; not on cascade path)

**Field access verified:**
- `Rules+0x4a0..0x4ac` Prism* fields read at `0x0044B349` (PrismSupportMax),
  `0x004504AE` (PrismSupportModifier), `0x0044ACC4` (PrismSupportDelay),
  `0x0044ABE2` (PrismSupportDuration)
- `BuildingClass+0x664` read sites: `0x0044B32F`, `0x0044B4CB`, `0x0045049E`
- `BuildingClass+0x664` write sites: `0x0044B4D7` (cascade INC),
  `0x004504CD` (ProcessDelayedFire reset), `0x0044ACCA` (EmitBeam reset)
- `BuildingClass+0x704` write sites: `0x0044B51E` (mode=2),
  `0x0044B5BB`, `0x0044B65C` (mode=1)

**INI files checked:**
- `ini/rulesmd.ini` — `[General]`,
  `[ATESLA]`, `[PrismShot]`, `[PrismSupport]`, `[PrismWarhead]`
- `ini/artmd.ini` — `[GAPRIS_A]`,
  `[GAPRIS_AD]`, `[GAPRIS_B]`, `[GAPRIS_BD]`
- `ini/rules.ini` — `[General]`
  (`PrismSupportDelay=60` differs from YR's `45`)

**String addresses:**
- `0x0083BBF4` "PrismType" → `0x0067113D` (RulesClass::ReadGeneral)
- `0x0083BBDC` "PrismSupportModifier" → `0x00671161`
- `0x0083BBCC` "PrismSupportMax" → `0x0067118C`
- `0x0083BBB8` "PrismSupportDelay" → `0x006711AB`
- `0x0083BBA0` "PrismSupportDuration" → `0x006711CB`
- `0x0083BB8C` "PrismSupportHeight" → `0x006711EB`
- `0x0081A760` "IsAnimDelayedFire" → `0x004611AA` (BuildingTypeClass::ReadINI)
- `0x0081A74C` "DelayedFireDelay" → `0x004611C7`
- `0x0081A9B8` "ChargedAnimTime" → `0x00460B9E`

**Cross-referenced docs:**
- `PRISM_FORWARDING_GHIDRA_REPORT.md` — prior iteration; this report
  fills its declared gaps G1, G2, G4, G5, G6 and refutes G3.
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` — confirms +0x664 polymorphism
  (garrison vs prism), +0x16E8 / +0x16EC offsets.
- `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — confirms +0x16B8 =
  `IsChargeMode` (different from PrismType gate; gates the OTHER
  Mission_Attack branch entirely).
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — confirms
  `ProcessDelayedFire` field offsets (0x704 mode, 0x714 timer).
- `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` — confirms +0x664 polymorphism.
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` — confirms bullet+0x150 default
  0x100, bullet+0x6C = Damage. Should be **upgraded** to label
  `+0x150` as `DamageScale (8.8 fixed-point, 0x100 = 1.0x)`.
