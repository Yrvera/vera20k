# Temporal — Erase-Over-Time

This doc is the canonical reference for the **Temporal weapon system** in gamemd.exe:
the Chrono Legionnaire's "erase" mechanic.

A unit with a `Temporal=yes` warhead doesn't deal damage — it gradually erases the
target from existence over time, decrementing a `WarpHP` counter (= `target.Strength × 10`)
by the firer's weapon `Damage` value each tick. Multiple firers stack additively via a
doubly-linked chain. When WarpHP reaches 0 the target is destroyed (with building-specific
side-effects: parachuting passengers, factory queue release, super-weapon suspend).
Interrupting the firer instantly restores the target — no recovery curve.

The system has three components:
1. **WarheadTypeClass** — `Temporal=yes` flag triggers the temporal path in `WarheadTypeClass::Detonate`.
2. **TemporalClass** — per-firer object managing the erase lifecycle (`Update` tick, chain links, completion).
3. **WarpAttachClass** — visual state machine driving the beam / oscillation / WARPAWAY animation phases.

Out-of-scope:
- The damage transform itself → [`damage_formula.md`](damage_formula.md) (Temporal warheads bypass the normal damage path)
- Chrono Legionnaire's own teleport (after-erase movement) → [`../../TELEPORT_LOCOMOTION_DEEP_DIVE.md`](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)
- Visual rendering pipeline → existing canonical [`../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md`](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md) §12, [`../../TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md`](../../TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md)
- Warhead Detonate dispatch parent → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)

---

## 1. Function identity

| Field | Value |
|---|---|
| Warhead dispatch | `WarheadTypeClass::Detonate 0x004690B0` — Temporal branch at priority 4 (after MindControl/IvanBomb/ElectricAssault) |
| Initiate | `TemporalClass::InitiateWarp 0x0071AF20` |
| Per-tick update | `TemporalClass::Update 0x0071A760` (HP countdown — head of chain only) |
| Eligibility gate | `TemporalClass::CanWarpTarget 0x0071AE50` |
| Detach | `TemporalClass::DetachFromTarget 0x0071ABC0` |
| Chain damage sum | `TemporalClass::SumChainDamage 0x0071AB10` |
| Visual state machine (AI) | `TemporalClass::AI 0x006297F0` (also `WarpAttachClass::UpdateAttack`) |
| Visual phase tick | `TechnoClass::UpdateTemporalVisual 0x0070E5A0` (10-phase machine on the TARGET) |
| Sprite scale | `TechnoClass::ScaleByTemporalVisualPhase 0x0070E380` |

### Confidence

- **Content: HIGH** — `CanWarpTarget` re-decompiled live 2026-05-17; matches existing canonical doc.
- **Identity: HIGH** — all functions named in the Ghidra annotation set.
- **Binding: HIGH** — single dispatch from Detonate's Temporal branch; Update runs per-tick via `TechnoClass::AI_Update` chain.

---

## 2. The flag (verified)

| Field | Value |
|---|---|
| Offset | `wh+0x15A` |
| INI key | `Temporal=` |
| String addr | `0x00817168` (verified live 2026-05-17) |
| Parser xref | `WarheadTypeClass::ReadINI 0x0075D590` |
| Default | `false` |
| Mutually-exclusive | YES — runs in `WarheadTypeClass::Detonate` if-else cascade at priority 4 (after MindControl/IvanBomb/ElectricAssault, before Parasite) |

### Warpable target-side flag (verified)

| Field | Value |
|---|---|
| Offset | `type+0xD3A` |
| INI key | `Warpable=` |
| String addr | `0x00843778` (verified live 2026-05-17) |
| Default | `true` (most units) |

Used by `CanWarpTarget`: if `target.Type.Warpable == 0`, temporal is rejected.

### OpenToppedWarpDistance Rules constant

| Field | Value |
|---|---|
| Offset | `Rules+0xF60` |
| INI section | `[General]` |
| INI key | `OpenToppedWarpDistance=` |
| String addr | `0x0083AFD4` (verified live 2026-05-17) |
| Default | `7` (cells) |

Used by `TemporalClass::Update` to break the link when the firer is in an open-topped
transport and the transport moves more than `7 × 256 = 1792` leptons from the target.

---

## 3. The mutually-exclusive cascade

`WarheadTypeClass::Detonate` if-else priority (from existing canonical doc):

| Priority | WH offset | INI key | Description |
|---:|---|---|---|
| 1 | `+0x155` | `MindControl=` | Mind control capture |
| 2 | `+0x157` | `IvanBomb=` | Ivan bomb attachment |
| 3 | `+0x158` | `ElectricAssault=` | Electric assault |
| 4 | **`+0x15A`** | **`Temporal=`** | **Temporal warp / erase** |
| 5 | `+0x159` | `Parasite=` | (NOTE: existing canonical doc lists Parasite at `+0x159` but priority-5 below Temporal — likely an offset typo; verify in follow-up #1) |
| 6 | `+0x15B` | (unknown special) | |
| 7 | `+0x16C` | `IsLocomotor=` | Magnetron locomotor override |
| ... | ... | ... | ... |

A warhead with both `MindControl=yes AND Temporal=yes` will mind-control (priority 1
fires first; cascade terminates). Only one branch executes per detonation.

---

## 4. The math — erasure speed

```
WarpHP_initial = target.Type.Strength × 10                  // 10× HP buffer
damage_per_tick = sum over chain of attacker.weapon.Damage  // see SumChainDamage
WarpHP -= damage_per_tick   (each tick)
```

When `WarpHP < 1`, the target is destroyed.

### Worked examples

For a Chrono Legionnaire firing `[NeutronRifle]` with `Damage=8`:

| Target | Strength | WarpHP | Ticks to erase | Real time (15 FPS) |
|---|---:|---:|---:|---:|
| Conscript | 125 | 1250 | 156 | ~10 sec |
| Rhino Tank | 400 | 4000 | 500 | ~33 sec |
| Apocalypse Tank | 800 | 8000 | 1000 | ~67 sec |
| War Factory | 1000 | 10000 | 1250 | ~83 sec |

With Elite `[NeutronRifleE]` (`Damage=16`): half the time.

With 2× Chrono Legionnaires stacking: half the time. 3× = third the time. Etc.

### `SumChainDamage` (`0x0071AB10`)

```c
int SumChainDamage(this, depth) {
    int sum = 0;
    if (this->NextInChain != NULL && depth < 51):
        sum = this->NextInChain->SumChainDamage(depth + 1);
    weapon = this->Owner->GetWeapon(this->Owner->GetCurrentWeaponIndex());
    int myDamage = weapon->Damage;     // weapon+0xA4
    this->DamagePerTick = myDamage;
    return myDamage + sum;
}
```

The recursion is depth-capped at 51 to prevent infinite loops on broken chains.

### Confidence (math)

- **Content: HIGH** — formula verified in existing canonical doc.
- **Identity: HIGH** — single Update function consumes the formula; single SumChainDamage helper.
- **Binding: HIGH** — Update runs every tick on the head of the temporal chain (via TechnoClass::AI_Update).

---

## 5. `CanWarpTarget` — eligibility gate (verified live)

`TemporalClass::CanWarpTarget` at `0x0071AE50`, decompiled live 2026-05-17:

```c
bool CanWarpTarget(target) {
    if target == NULL: return false

    type = target->GetTechnoType()                       // vtable+0x84
    if type.byte+0xD3A (Warpable) == 0:
        return false                                      // 1. Warpable=no → immune

    if target->IsInvulnerable() (vtable+0x160) != 0:
        return false                                      // 2. Iron Curtain / similar → immune

    if target.WhatAmI() == 1 (Infantry):                  // 3. Infantry-going-to-Grinder gate
        dest = target.GetDestination()                    // FootClass::GetDestination
        if dest != NULL && dest.WhatAmI() == 6 (Building):
            if dest.Type.byte+0x16BD != 0:                // 4. Grinder flag on dest building
                target_cell = Get_Cell_At(target.coords)
                building_in_cell = Look_up_building_in_cell(target_cell)
                if building_in_cell == dest:              // target is currently ON the grinder
                    return false                          // 5. don't temporal-erase a grinding infantry

    return true
}
```

### Summary of immunities

| Source | Mechanism |
|---|---|
| `Warpable=no` on TypeClass | type+0xD3A flag — explicit immunity |
| Iron Curtain | vtable+0x160 (IsInvulnerable) returns true |
| Force Shield | Same — IsInvulnerable covers both |
| Infantry on Grinder cell (entering refinery-style structure to be ground up) | Grinder flag on building type+0x16BD + same-cell check |

**Not immune:**
- Buildings — special handling but they CAN be temporal'd (with consequences — see §7)
- Mind-controlled units — NOT immune (and the MC link is freed in InitiateWarp)
- Units already being temporal'd by a DIFFERENT firer — joined to the chain
- Units already being temporal'd by THIS firer — detached and re-targeted

### Confidence

- **Content: HIGH** — live decomp 2026-05-17.
- **Identity: HIGH** — function named.
- **Binding: HIGH** — only caller is InitiateWarp.

---

## 6. `InitiateWarp` — establishing the link

`TemporalClass::InitiateWarp` at `0x0071AF20`. Flow:

```c
void InitiateWarp(target) {
    // 1. Kill spawned units (Carrier-style spawns)
    if (target.SpawnManager): SpawnManagerClass::Kill_All_Spawns()

    // 2. Free mind-controlled victims (controller being temporal'd = release all)
    if (target.CaptureManager): CaptureManagerClass::FreeAll()

    // 3. If our owner already has a temporal target, detach first
    if (Owner.OwnTemporal != NULL && Owner.OwnTemporal.Target != NULL):
        DetachFromTarget()

    // 4. Gate
    if (!CanWarpTarget(target)): return
    if (Owner.TemporalTargetingMe != NULL): return       // can't temporal if WE are being temporal'd

    // 5. Establish link
    this.Target = target

    if (target.TemporalTargetingMe == NULL):
        // FIRST temporal targeting this unit
        target.TemporalTargetingMe = this
        // Initialize WarpHP
        this.WarpHP = target.Type.Strength * 10
        // Building-specific: radar event, EVA "Unit under attack"
        if (target is building && target.Owner == g_PlayerPtr):
            CreateRadarEvent(target.coords)
            VoxClass::PlayEVA(-1)
            target.Owner.NeedsRecalc = true
            BuildingClass::StartCloaking()                // (actually UN-cloaks for visibility)
            target.Owner.NeedsRebuild = true
    else:
        // STACKING — insert into doubly-linked list
        existing_head = target.TemporalTargetingMe
        this.PrevInChain = existing_head
        this.NextInChain = existing_head.NextInChain
        existing_head.NextInChain = this
        if (this.NextInChain != NULL):
            this.NextInChain.PrevInChain = this

    // 6. Mark target
    target.IsBeingWarpedOut = true                        // target+0x270

    // 7. If attacker is Gattling, advance stage
    if (Owner.Type.IsGattling): Owner.UpdateGattlingStage(1)

    // 8. Force visual refresh
    target.UpdateVisual(2)

    // 9. If target has its own temporal weapon, detach IT from its target
    if (target.OwnTemporal != NULL && target.OwnTemporal.Target != NULL):
        target.OwnTemporal.DetachFromTarget()

    // 10. Update fog/shroud for the player who can see target
    if (g_PlayerPtr): target.UpdateFog()
}
```

---

## 7. `Update` — per-tick erasure

`TemporalClass::Update` at `0x0071A760`. Only the **head** of the chain runs this.

```c
void Update() {
    target = this->Target

    // SAFETY: if we're somehow not the head despite running, reset
    if (target && target.TemporalTargetingMe == this && this.PrevInChain != NULL):
        target.TemporalTargetingMe = NULL
        target.IsBeingWarpedOut = false
        ClearLinkedList()
        return

    // OPEN-TOPPED RANGE CHECK
    if (Owner && Owner.IsOpenTopped):
        dist = Sqrt_Approx(distance_sq(Owner.coords, target.coords))
        if (dist > Rules.OpenToppedWarpDistance * 256):     // +0xF60, default 7
            DetachFromTarget()
            return

    // SUM CHAIN DAMAGE
    chainDamage = (NextInChain) ? SumChainDamage() : 0
    weapon = Owner.GetWeapon(Owner.GetCurrentWeaponIndex())
    myDamage = weapon.Damage                                  // weapon+0xA4
    this.DamagePerTick = myDamage

    // DECREMENT
    this.WarpHP -= (myDamage + chainDamage)

    // COMPLETION
    if (this.WarpHP < 1):
        if (target == NULL):
            // Already gone
            clear_all_fields()
            Owner.StopAction(0, 1)
        else:
            // Play WARPAWAY anim at target position
            new AnimClass(Rules.WarpAway, target.coords, ...)    // Rules+0x340

            // (experience transfer if owner type has the flag set)

            if (target is building):
                if (building.OccupantCount > 0):
                    SpawnUnitsWithParachute(0)
                SuperClass::Suspend(0)
                BuildingClass::UndockUnit()
                target.ReceivedDamage(owner)                     // vtable+0x3b8
                target.Destroy(owner)                             // vtable+0xe0
                target.Remove()                                    // vtable+0xf8
                target.Owner.NeedsRebuild = true
            else:
                if (target.GetLocomotor()): locomotor.Destroy()
                target.ReceivedDamage(owner)
                target.Destroy(owner)
                target.Remove()

            Owner.StopAction(0, 1)

        // Clean up
        this.Target = NULL
        this.NextInChain = NULL
        this.PrevInChain = NULL
        this.TimerAux = NULL
        this.TimerStart = NULL
}
```

---

## 8. Stacking via doubly-linked chain

The chain is anchored at `target.TemporalTargetingMe (target+0x278)`. Each TemporalClass
has `PrevInChain (+0x40)` and `NextInChain (+0x44)`.

```
target.TemporalTargetingMe → [Temporal_Head] ↔ [Temporal_2] ↔ [Temporal_3] ↔ ...
                              PrevInChain=NULL                                  NextInChain=NULL
```

### Rules

- Only the **head** runs `Update` for HP countdown.
- `SumChainDamage` recursively walks `NextInChain` (depth-capped at 51) summing weapon damage.
- Total damage per tick = head's damage + sum of all NextInChain's damages = sum across the entire chain.
- When a non-head attacker dies / detaches, list splices normally.
- When the **head** dies, the next member inherits the role and the **remaining WarpHP transfers**:

```c
// In DetachFromTarget when head is detaching with a next:
target.TemporalTargetingMe = this.NextInChain
this.NextInChain.PrevInChain = NULL
this.NextInChain.WarpHP = this.WarpHP     // transfer remaining
```

So losing a Chrono Legionnaire mid-erase doesn't reset progress — the next attacker
in line picks up where the dead one left off.

### Confidence

- **Content: HIGH** — fully decompiled in canonical doc.
- **Identity: HIGH** — single chain structure on each target.
- **Binding: HIGH** — head-only Update + chain damage sum are the only consumers.

---

## 9. Detach paths

`TemporalClass::DetachFromTarget` at `0x0071ABC0`. Three cases:

### Case A: Head, no next (sole attacker)

```
target.TemporalTargetingMe = NULL
target.IsBeingWarpedOut = false
target.UpdateVisual(2)               // restore normal appearance
(building: recalc, stop cloak)
```

The target **snaps back instantly** — no recovery curve, no gradual unwarping. The
`IsBeingWarpedOut` flag is cleared and the visual warp factor resets.

### Case B: Head with next

```
target.TemporalTargetingMe = this.NextInChain
this.NextInChain.PrevInChain = NULL
this.NextInChain.WarpHP = this.WarpHP    // transfer
```

### Case C: Middle / tail

```
if (this.NextInChain): this.NextInChain.PrevInChain = this.PrevInChain
if (this.PrevInChain): this.PrevInChain.NextInChain = this.NextInChain
```

### Detach triggers

- `TemporalClass::Update` open-topped range check break
- `Owner.OwnTemporal->Target` re-targeting (InitiateWarp step 3)
- Owner death / removal
- Owner enters transport
- Owner chronoshifted away

---

## 10. Building-specific completion behavior

When the target is a Building (`target.WhatAmI() == 6`) and erasure completes:

1. **Occupants parachute out.** `SpawnUnitsWithParachute(0)` — any garrisoned infantry escape.
2. **Super-weapon charging suspended.** Any SW that was charging in this structure is paused.
3. **Undock.** `BuildingClass::UndockUnit()` — any unit docked at this building (harvester etc.) is undocked.
4. **House rebuild flag.** Owner's `NeedsRebuild` is set (for AI re-planning).
5. **Target killed via standard chain.** `ReceivedDamage → Destroy → Remove`.

### Cloaking behavior on building target

When InitiateWarp targets a building, `BuildingClass::StartCloaking()` is called — but
this is really the building's **decloak** path (the function name is misleading). The
building is forced visible while being erased so the player can see it.

---

## 11. Key field offsets

### WarheadTypeClass

| Offset | Field |
|---|---|
| `+0x15A` | `Temporal` |

### TechnoTypeClass

| Offset | Field |
|---|---|
| `+0xD3A` | `Warpable` |
| `+0x16BD` | (Grinder flag — per CanWarpTarget infantry check) |
| `+0xCCE` | `Teleporter` (probable — used by WarpAttachClass::Detach for valid-cell search) |

### TechnoClass instance

| Offset | Field |
|---|---|
| `+0x270` | `IsBeingWarpedOut` (bool) |
| `+0x274` | `OwnTemporal` (TemporalClass*) |
| `+0x278` | `TemporalTargetingMe` (head of chain) |
| `+0x328` | `WarpFactor` (float — visual translucency) |
| `+0x1A4` | `VisualPhaseState` (0..10) |

### TemporalClass

| Offset | Field |
|---|---|
| `+0x24` | `Owner` (TechnoClass*) |
| `+0x28` | `Target` (TechnoClass*) |
| `+0x40` | `PrevInChain` (TemporalClass*) |
| `+0x44` | `NextInChain` (TemporalClass*) |
| `+0x48` | `WarpHP` (int) — also reused as anim state in AI path |
| `+0x4C` | `DamagePerTick` (int) |
| `+0x50` | `SubFrameCounter` |

### RulesClass

| Offset | Field | INI section |
|---|---|---|
| `+0x338` | `WarpIn` (AnimType*) | `[General]` |
| `+0x33C` | `WarpOut` (AnimType*) | `[General]` |
| `+0x340` | `WarpAway` (AnimType*) | `[General]` |
| `+0xF60` | `OpenToppedWarpDistance` (int cells) | `[General]` |
| `+0x1866` | Temporal beam color RGB (3 bytes) | `[AudioVisual]`-style |

---

## 12. Retail YR INI chain (`ini/rulesmd.ini`)

```ini
[DVDP]                 ; Chrono Legionnaire
  Primary=NeutronRifle

[NeutronRifle]
  Damage=8
  ROF=120
  Warhead=ChronoBeam
  IsRadBeam=yes

[NeutronRifleE]        ; Elite
  Damage=16
  Warhead=ChronoBeam

[ChronoBeam]           ; the warhead
  Temporal=yes
```

So the Chrono Legionnaire's primary fires `NeutronRifle` which uses `ChronoBeam`
warhead. ChronoBeam has `Temporal=yes` triggering the temporal path. Damage=8 means
8 WarpHP-per-tick. ROF=120 means a fire trigger every 120 frames (~8 sec), though the
temporal damage is per-AI-tick not per-fire (the firer maintains the link).

---

## 13. TS-legacy filter

- **`Temporal=` warhead flag**: LIVE in YR. Used by Chrono Legionnaire.
- **`Warpable=` TypeClass flag**: LIVE in YR. Set on most units to allow temporal targeting; set to `no` on a few specific types for immunity.
- **`OpenToppedWarpDistance=` Rules constant**: LIVE in YR. Battle Fortress passenger temporal exists.
- **Building temporal mechanics**: LIVE in YR.
- **`Culling=` warhead flag** (priority warhead+0x174): existing canonical doc flags this as "appears to be active in YR (the ChronoBeam warhead could theoretically have it). However, standard YR warheads don't set Culling=yes." Open follow-up #4. May be TS-only dead code reached from the AI state-4 path.

No fully-dead TS-only branches identified in the core temporal system.

---

## 14. Edge cases

| Case | Behavior |
|---|---|
| Two Chrono Legionnaires target same Conscript (`Strength=125`) | WarpHP=1250, chainDamage=16/tick → 78 ticks (~5 sec) |
| Chrono Legionnaire targets a unit already mind-controlled | InitiateWarp Step 2 frees all MC victims (`CaptureManager::FreeAll`), then proceeds. Target's MC controller link is broken. |
| Chrono Legionnaire is itself mind-controlled | Now belongs to a different house. The CL continues to erase its own original target (the temporal link survives ownership change unless the controller dies, which triggers MC FreeAll and resets) |
| Chrono Legionnaire is chronoshifted out mid-erase | `TemporalClass::InitiateWarp` Step 4 calls `DetachFromTarget` on owner.OwnTemporal. Target snaps back instantly. |
| Battle Fortress carrying a CL fires temporal, then moves > 7 cells away | Update's range check fails → DetachFromTarget. Erasure abandoned. |
| Target enters Iron Curtain mid-erase | IsInvulnerable becomes true. CanWarpTarget would block NEW erase but existing chain continues (the gate is only checked at InitiateWarp time). Open follow-up #2: does Update re-check immunity? |
| Target is killed by normal damage during erase | Standard ReceiveDamage path handles destruction. Chain detach via Owner cleanup. |
| Target is a SuperWeapon-charging building | On erase completion, SW is suspended (`SuperClass::Suspend(0)`). Charge progress is lost. |
| Target is a Factory with units queued | UndockUnit path runs; queue release behavior is in the building destruction chain. |
| Stacking 3+ Chrono Legionnaires | Up to 51-deep chain (SumChainDamage depth cap). Each adds their weapon Damage to the per-tick decrement. |
| Re-target while mid-erase | `Owner.OwnTemporal.DetachFromTarget()` (Step 3 of InitiateWarp) — old target snaps back, new target starts at full WarpHP. |

---

## 15. Open follow-ups

1. **Parasite / Temporal cascade priority swap.** Existing canonical doc says priority 4 is Temporal (`+0x15A`), priority 5 is Parasite (`+0x159`) — but the offsets are reversed (Parasite at smaller offset than Temporal). The cascade priority order may need verification by re-reading `WarheadTypeClass::Detonate`. Priority: MEDIUM (parity-relevant if both flags are set on the same warhead — though no retail warhead does this).
2. **Update-time immunity re-check.** Does `TemporalClass::Update` re-check `CanWarpTarget` each tick? Or is the gate only at initiation? Priority: MEDIUM — affects whether Iron Curtain mid-erase saves the target.
3. **WarpAttachClass `Detach` cell-search for the CL teleport.** When erase completes, the firing CL relocates to a "valid cell." The placement search algorithm and fallback (if no valid cell) needs documentation. Priority: LOW (visual/positioning detail).
4. **`Culling=` warhead flag in temporal state-4 path.** The AI state-4 completion branch references `warhead+0x174 (Culling)`. Is this live in YR? No retail warhead is known to set it. Priority: LOW.
5. **Experience-transfer flag identity.** Existing canonical doc says "if `owner->TypeClass->field_0xc8e`" controls experience transfer. INI key identity for this offset not traced. Priority: LOW.
6. **Beam color `Rules+0x1866`.** RGB triplet at this offset. Should this be `Rules.TemporalBeamColor` or similar? Not in the existing doc. Priority: LOW.
7. **Mind control of a CL DURING erase.** When the CL's owner changes via MC, does the temporal link transfer ownership too? The link's `Owner` pointer is the CL itself, not the house, so the link survives ownership change. Verify by inspection. Priority: LOW.
8. **Warpable= default in TypeClass constructor.** Many units don't set `Warpable=` explicitly. Confirm default is `true`. Priority: LOW.

---

## 16. Sources

- Live decompilation of `TemporalClass::CanWarpTarget` at `0x0071AE50` (2026-05-17) — confirmed `Warpable +0xD3A`, IsInvulnerable vtable+0x160, infantry-on-Grinder gate at type+0x16BD.
- Live xrefs (2026-05-17):
  - `"Temporal"` at `0x00817168`
  - `"Warpable"` at `0x00843778`
  - `"OpenToppedWarpDistance"` at `0x0083AFD4`
- Existing canonical doc: [`../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md`](../../TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md) (687 lines, 2026-04-04, ~90% confidence per its own confidence note) — primary source for everything in this systems doc. Migrated content here.
- Existing canonical doc: [`../../TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md`](../../TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md) — complementary pipeline coverage.
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`mind_control.md`](mind_control.md) (CaptureManager::FreeAll trigger from Temporal), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md), [`can_target_gates.md`](can_target_gates.md) (gate #62), [`anti_air_dispatch.md`](anti_air_dispatch.md).
