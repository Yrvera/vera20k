# ParasiteClass (Terror Drone / Giant Squid Attachment) — Ghidra Report

Research date: 2026-04-19

Standalone manager class that handles infestation-style weapon attachments — the
Terror Drone burrowing into a vehicle and the Giant Squid grappling a ship.
Shares the `WarpAttachClass` base with `TemporalClass` (both objects are
"one attacker, one victim, periodic damage, forced release" state machines).

Active in YR skirmish: **YES**. Driven entirely by `Warhead.Parasite=yes`, which
is used by the Terror Drone's `TerrorDroneJump` weapon and the Giant Squid's
`SquidGrip` weapon.

---

## 1. Ownership & lifetime

A `ParasiteClass` is a **per-attacker** object: it lives on the unit that fires a
parasite weapon, not on the victim. It is created in `TechnoClass::Init_Managers`
(`0x006F3F40`) via:

```c
piVar3 = Techno->GetPrimaryWeapon(0);               // vtable+0x3F8
if (*piVar3 != 0) {
    Weapon = *piVar3;
    Warhead = *(WarheadTypeClass **)(Weapon + 0xac);
    if (Warhead->Parasite /* byte +0x159 */) {
        Techno->Parasite = new ParasiteClass(Techno);
    }
}
param_1[0x1a7] = ParasiteClass*;   // byte 0x69C on FootClass
```

Gates (all checked at construction time, `0x006F3F40`):
1. `Techno->GetPrimaryWeapon(0)` must return non-null (weapon slot 0).
2. `Weapon->Warhead->Parasite` (byte `+0x159`) must be set.
3. `Techno->WhatAmI() != 6` — buildings never get one (the check at
   `0x006F3FD9` rejects `WhatAmI()==6`). In practice only UnitClass (WhatAmI==4)
   uses parasite weapons.

The attach pointer back from victim to attacker is kept on the victim FootClass
at byte `+0x694` (see §2 "Victim-side field").

Allocation size: `operator_new(0x58)` in Init_Managers — ParasiteClass is 88
bytes.

---

## 2. Struct layout

### ParasiteClass (0x58 bytes)

`ParasiteClass` inherits `AbstractClass`. Offsets below are byte offsets from the
start of the object. Values labelled "verified" are reads/writes observed in at
least two distinct functions or two distinct callers.

| Offset | Type         | Field           | Source / verification                                                               |
|--------|--------------|-----------------|-------------------------------------------------------------------------------------|
| 0x00   | vtable*      | vtbl_primary    | Ctor @ `0x006292B0` writes `&vtable__ParasiteClass`                                 |
| 0x04   | vtable*      | vtbl_sec_4      | Ctor                                                                                |
| 0x08   | vtable*      | vtbl_sec_8      | Ctor                                                                                |
| 0x0C   | vtable*      | vtbl_sec_12     | Ctor                                                                                |
| 0x10-0x23 | —         | AbstractClass   | `AbstractClass__Constructor_Full` fills this                                        |
| 0x24   | TechnoClass* | **Owner**       | Ctor writes `param_2`. Read as `param_1[9]` throughout WarpAttachClass methods      |
| 0x28   | TechnoClass* | **Victim**      | Early-return in UpdateAttack if 0; cleared to 0 at end of Detach                    |
| 0x2C   | int          | DmgTimer.Start  | Ctor: `g_CurrentFrameCounter`. FootClass::ReceiveDamage re-arms with current frame  |
| 0x30   | int          | DmgTimer.Pause  | Written alongside 0x2C/0x34 in ReceiveDamage (CDTimer middle slot)                  |
| 0x34   | int          | DmgTimer.Dur    | Set to `damage*2 - TypeClass->field_0xD6C` in ReceiveDamage; to 0x32 on heal path    |
| 0x38   | int          | AtkTimer.Start  | UpdateAttack: `CurrentFrame - (+0x38) >= (+0x40)` test; `-1` ⇒ not running          |
| 0x3C   | int          | AtkTimer.Pause  | Reset to `iStack_c` alongside Start/Dur each tick fires                             |
| 0x40   | int          | AtkTimer.Dur    | Set to `Weapon.ROF` (Weapon+0xB0) on each damage tick                               |
| 0x44   | void*        | AttachedAnim?   | Cleared in Detach via `(*vtable+0xf8)()` then null — behaves like an AnimClass*     |
| 0x48   | int          | unknown_48      | Ctor zero                                                                           |
| 0x4C   | int          | unknown_4C      | Ctor zero, cleared in Detach                                                        |
| 0x50   | int          | unknown_50      | Ctor zero, cleared in Detach                                                        |
| 0x54   | byte         | InGlobalList    | Ctor zero. If set, Detach removes `this` from the global vector at `DAT_00B0F5B8`   |
| 0x55-0x57 | —         | align pad       |                                                                                     |

Fields `0x48..0x54` form the parasite's entry in a `DynamicVectorClass<ParasiteClass*>`
registered in the `VectorClass` ctor chain at `DAT_00AC4910`. Two parasite
vectors exist: the **type vector** (registered by the generic one-arg ctor at
`0x00629210`) and the **active-attach vector** at `DAT_00B0F5B8` which tracks
"parasites currently inside a host" (managed in Detach, `0x15` byte).

### Victim-side field (on FootClass)

The victim remembers its attacker via a pointer on its own FootClass:

| Offset | Type         | Field           | Source / verification                                                               |
|--------|--------------|-----------------|-------------------------------------------------------------------------------------|
| 0x694  | FootClass*   | ParasiteAttacker| FootClass::AI tail @ `0x004DAEEB` dereferences `(attacker+0x69C)->vtbl[23]` (AI). Detach clears victim[0x694] |
| 0x328  | byte         | IsParasited     | Cleared at end of Detach (victim[0x328] = 0); matches victim-side "has parasite" flag |

**The attacker-side manager pointer** is at `+0x69C` on FootClass, per
`Init_Managers` (`param_1[0x1A7]`). The same offset is read on the attacker
FootClass by the victim's per-tick dispatch. This is the `ParasiteClass*` itself.

> Discrepancy: `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` labels `+0x694` as `Team*`
> and `+0x69C` as "unknown_69C". Both of those identifications are wrong; the
> FootClass::AI tail (§4) dispatches ParasiteClass::AI through these exact
> pointers. `ADDRESS_MAP.md` already flags `+0x694` as the parasite attacker.
> This report does not edit FOOTCLASS_COMPLETE — that amend belongs in its own
> pass.

---

## 3. WarheadTypeClass INI gate

Only one flag gates this whole system:

| INI key        | Warhead offset | Verified via                                                         |
|----------------|----------------|----------------------------------------------------------------------|
| `Parasite`     | `+0x159`       | `Init_Managers` filter at `0x006F3FAD`; matches `WARHEAD_DETONATE_GHIDRA_REPORT §Parasite` |

Unrelated offsets often confused with this one:
- `+0x155` = MindControl
- `+0x156` = Poison
- `+0x157` = IvanBomb
- `+0x15A` = Temporal (note: FOOTCLASS_NON_MOVEMENT_FIELDS.md lists +0x156 as
  "Poison (parasite)" which is misleading — Poison and Parasite are separate flags.)

---

## 4. Per-tick dispatch (how UpdateAttack gets called)

`ParasiteClass` has no tick of its own from the global game loop. It is driven
by the **victim's** FootClass::AI:

```c
// FootClass::AI  @ 0x004DA530, tail block at 0x004DAEEB
if (param_1[0x1a5] /* victim+0x694 = ParasiteAttacker */ != 0) {
    FootClass *atk  = param_1[0x1a5];
    ParasiteClass *p = *(ParasiteClass **)(atk + 0x69C);
    p->vtbl[0x5C/4]();          // WarpAttachClass::UpdateAttack
}
```

Ordering: after TryEnterTransport, before IPiggyback finalize. This means a
parasited unit drives its attacker's damage cadence from inside its own AI —
the attacker FootClass itself is in limbo (`InLimbo=1`) while attached, so the
attacker's own FootClass::AI is not what ticks the parasite.

`vtable[0x5C/4]` on ParasiteClass resolves to `WarpAttachClass::UpdateAttack`
at `0x00629FD0` (xref from `0x007EF8EC` in the ParasiteClass vtable).

---

## 5. UpdateAttack (damage tick) — `0x00629FD0`

```c
void WarpAttachClass::UpdateAttack(this)
{
    if (this->Victim == 0) return;

    // Special branch when attacker is of a type whose Warhead.Owner.cce is
    // set AND whose type has field_0xD97 set. Delegates to TemporalClass::AI.
    // Both flags default to 0 in YR — no standard YR unit takes this path.
    TechnoTypeClass *atkT = this->Owner->GetTechnoType();
    if (atkT->field_0xCCE && atkT->field_0xD97) {
        TemporalClass::AI(this);
        return;
    }

    WeaponTypeClass *wpn = this->Owner->GetPrimaryWeapon(0);
    // Fire-rate gate: AtkTimer (+0x38..+0x40)
    if (this->AtkTimer.Start != -1) {
        int elapsed = CurrentFrame - this->AtkTimer.Start;
        if (elapsed >= this->AtkTimer.Duration) goto FIRE;
        remaining = this->AtkTimer.Duration - elapsed;
    }
    if (remaining != 0) return;

FIRE:
    this->AtkTimer.Start    = CurrentFrame;
    this->AtkTimer.Pause    = 0;
    this->AtkTimer.Duration = wpn->ROF;                 // Weapon+0xB0

    // Echo the same timer onto the victim FootClass at +0x6A0..+0x6A8.
    this->Victim->field_0x6A0 = CurrentFrame;
    this->Victim->field_0x6A4 = 0;
    this->Victim->field_0x6A8 = Warhead->field_0x170;

    // Spawn one particle system from RulesClass+0x1020 (Prop01Particles-style)
    // Spawn one AnimClass from WarheadTypeClass+0xF8[random] if wh+0x104 != 0
    // Nudge victim by ±2 pixels for shake (vtable+0x3D8)

    this->Victim->vtbl->TakeDamage(
        dmg     = Warhead->Verses * scale,
        warhead = Warhead,
        source  = this->Owner,
        ...);                                           // vtable+0x16C
}
```

Key observations:
- The damage cadence is the attacker's weapon ROF — not a fixed constant.
- `this->Victim->field_0xF/0x54` etc. are `TechnoTypeClass` reads, not `HouseClass`.
- The two-flag `TemporalClass::AI` fallback at the top does **NOT** fire for
  vanilla Terror Drone or Giant Squid — neither type sets both `+0xCCE` and
  `+0xD97`. This branch is TS-era / modded-weapon cover; safe to ignore for YR
  fidelity.

---

## 6. Damage relay on the host (`FootClass::ReceiveDamage`, `0x004D6FA0`)

When the **victim** takes damage from any source while parasited, two side
effects fire (verified from decompilation at `0x004D735F`):

```c
// Block A: warhead-forced eject (Sonic warhead, e.g. Dolphin vs Giant Squid)
if (warhead != 0 && warhead->Sonic /* +0x14B */ && victim->ParasiteAttacker != 0) {
    WarpAttachClass::Detach(victim->ParasiteAttacker->Parasite);
    if (source != 0) source->vtbl[0x3C8/4]();
}

// Block B: threshold-based eject arming
FootClass *atk = victim->ParasiteAttacker;
if (atk != 0 && source != atk) {
    if (damage > atk->Type->SuppressionThreshold /* +0xD6C */) {
        ParasiteClass *p = atk->Parasite;
        p->DmgTimer.Start    = CurrentFrame;
        p->DmgTimer.Pause    = 0;
        p->DmgTimer.Duration = damage * 2 - atk->Type->SuppressionThreshold;
    }
}

// Block C: healing forces eject after 50 frames
if (victim->ParasiteAttacker != 0 && damage < 0) {
    ParasiteClass *p = victim->ParasiteAttacker->Parasite;
    p->DmgTimer.Start    = CurrentFrame;
    p->DmgTimer.Pause    = 0;
    p->DmgTimer.Duration = 50;       // ~3.3s @ 15 fps
    WarpAttachClass::Detach(p);
}
```

`TypeClass+0xD6C` is the `SuppressionThreshold` INI key (verified in
`TechnoTypeClass::ReadINI` — string `s_SuppressionThreshold_008436EC` paired
with `param_1[0x35b]` write). The parasite code reuses this field as the
"damage to host needed to shake the terror drone loose" — the Terror Drone's
`SuppressionThreshold` doubles as its ejection threshold. For the Giant Squid
it works the same way but with the Squid's type value.

---

## 7. Attach (`WarpAttachClass::Attach`, `0x0062A980`)

This is the other half of the state machine — the call that actually binds
attacker to victim and is the only place `victim+0x694` is written non-zero.
Invoked from `WarheadTypeClass::Detonate` (`0x004690B0`) in the Parasite branch
(`Warhead+0x159 != 0`) after the `!MindControl !IvanBomb !Electric` filter.

Signature: `void Attach(ParasiteClass *this, TechnoClass *victim)`.

```c
void Attach(this, victim)
{
    // Reset damage-cycle accumulators (also covers re-attach from a prior victim)
    this->field_0x48 = 0;
    this->field_0x4C = 0;
    this->field_0x50 = 0;
    if (this->AttachedAnim /* +0x44 */) {
        this->AttachedAnim->vtbl[0xF8/4]();
        this->AttachedAnim = 0;
    }
    if (this->InGlobalList /* byte +0x54 */) {
        // Remove from DAT_00B0F5B8 (active-attach DynamicVector)
        idx = DAT_00B0F5B8.IndexOf(this);
        if (idx != -1) DynamicVector::Erase(idx);
        this->InGlobalList = 0;
    }

    this->AtkTimer.Start    = CurrentFrame;
    this->AtkTimer.Pause    = 0;       // uninitialized in decomp; zero at runtime
    this->AtkTimer.Duration = 0;       // first UpdateAttack will set to Weapon.ROF

    // Predicate: WarpAttachClass::CanAttach — see §7.1
    if (!CanAttach(this, victim)) {
        // FALLBACK PATH: victim unreachable. Try to place the attacker at a
        // passable cell near the victim (Rules+0x55C "ParasiteFinder" — the
        // inner `TechnoType->Locomotor` check). If placement fails, attacker
        // dies outright via its own vtable+0xF8 (RemoveThis).
        CellClass *c = FindNearbyPassableCell(attacker.type.Rules[0x55C]);
        if (!attacker->vtbl->CanEnterCell(c)) {
            attacker->vtbl[0xF8/4]();   // kill attacker
            return;
        }
        attacker->vtbl[0x48C/4](0,0,0,0);  // Unlimbo
        UpdateFogBorder(attacker.Coord, sight_range-3, sight_range+2, 0);
        if (!attacker->vtbl[0x4AC/4]()) {
            attacker->vtbl[0x3C8/4](0);   // ScatterFromMindControl (cleanup)
            attacker->vtbl[0x480/4](0,1); // cell occupation toggle
        }
        attacker->vtbl[0x484/4](0,1);     // mark in lists
        return;
    }

    // SUCCESS PATH: victim is reachable and legal target.
    ILocomotor *loco = attacker->Locomotor;   // attacker+0x674
    Coord3 v = victim.Coord;                  // victim+0x9C..0xA4
    loco->vtbl[0x70/4](loco, -1, v.X);        // hide-and-track setup

    victim->ParasiteAttacker  = attacker;     // victim+0x694 = attacker FootClass*
    this->Victim              = victim;       // parasite+0x28 = victim TechnoClass*
}
```

### 7.1 CanAttach predicate (`0x0062A8E0`)

```c
bool CanAttach(ParasiteClass *this, TechnoClass *victim)
{
    if (victim == NULL)                           return false;
    if (victim->InLimbo /* +0x81 */)              return false;
    if (!victim->IsAlive /* byte +0x90, read as param_2[0x24] high byte */) return false;
    if (victim->HealthPoints /* +0x6C */ == 0)    return false;
    if (victim->ParasiteAttacker /* +0x694 */)    return false;   // already parasited
    if (!victim->Type->Parasiteable /* +0xD38 */) return false;
    if (victim->field_0x2E4 != 0)                 return false;   // in-transit / mind-controlled
    // Naval-parasite bridge exclusion: if attacker type has +0xCCE set
    // (hypothesised "Naval"), reject if victim is on a bridge cell.
    if (attacker.Type->field_0xCCE && victim->GetCell()->IsBridge)
        if (!CellClass::IsOnBridgeSurface(victim->GetCell())) return false;
    return true;
}
```

The `Parasiteable` INI key maps to **TechnoTypeClass+0xD38** — confirmed by the
predicate above (not `0x714F86` which is the reader-function address cited in
`TECHNOCLASS_SYSTEMS_GHIDRA_REPORT §7.2`).

### 7.2 Why the parasite-attach path is a non-virtual shared method

`WarpAttachClass::Attach` at `0x0062A980` is a standalone C++ method, not a
vtable slot on the victim. Earlier reports (`WARHEAD_DETONATE_GHIDRA_REPORT`
§Parasite) described the attach as `target->ReceiveParasite() // vtable 0x3C8`.
That is incorrect on two counts:

1. `TechnoClass::vtable+0x3C8` is **ScatterFromMindControl**, not a parasite
   receiver (verified in `CLOAKING_INTERACTIONS_REPORT.md` and
   `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`).
2. The single `vtbl+0x3C8` call inside the parasite branch of
   `WarheadTypeClass::Detonate` is on the **attacker** (cleanup path), not the
   victim. The victim binding happens inside `Attach()` a few lines later.

The earlier report's `FUN_0071AF20 // additional parasite logic` note is also
wrong — that address is `TemporalClass::InitiateWarp`, in the Temporal branch
of the same outer if-else. See §13.

---

## 8. Detach (`WarpAttachClass::Detach`, `0x0062A4A0`)

Releases the attacker from the victim. Two exit paths:

**A. Die-attached path.** If the attacker's TypeClass has `Naval` (`+0xCCE`)
set (Giant Squid), Detach does not try to place
the attacker at the victim's cell; it skips straight to the cleanup tail and
the attacker is killed via `vtable+0xF8` on itself (`RemoveThis`-equivalent).
In vanilla this is the Squid behavior.

**B. Place-at-cell path.** Otherwise the attacker is unlimboed at a cell near
the victim. Steps:

1. Pick a coord.
   - Non-naval attacker: choose a random one of eight directional offsets
     around victim, scaled via `DAT_0089F68A` (cached table).
   - Fall back to victim's own cell if `CellClass::CheckCellPassability` fails.
2. `WarpAttachClass::CanPlaceAtTarget` (`0x0062AB40`) — gate using victim's
   locomotor class id (field 0xEC on the target's locomotor VTable struct —
   rejects classes 2, 3, 6 under extra constraints; allows 0x4A..0x63 and
   0xCD..0xE6 ranges).
3. `HouseClass::IsHumanPlayer()` AND attacker has the "was-on-radar" flag at
   `+0x432` → clear flag and call attacker vtable `+0x14C` (radar removal).
4. If attacker has a non-null `+0x10D` field (squad/team record), run
   `FUN_006EA500` with attacker — likely "remove from team list".
5. Attacker vtable `+0x4AC` (`IsNotPlaceable`-like). If it returns false:
   - `TechnoClass::SetGhostCell`, then attacker vtable `+0x3C8` (unstealth),
   - then attacker vtable `+0x480(0)` (cell-occupation toggle).
6. Attacker vtable `+0x484` (mark/unmark in map lists), then
   `+0x488(0,0,0,0,0)` — `Unlimbo` (canonical "place at coord").
7. Arm attacker's post-detach idle timer at `+0x6A0..+0x6A8` with duration
   `Weapon.ROF * 3` — this is the "recoil" window where the Terror Drone cannot
   re-target while it climbs out.
8. Clear parasite state: `this->field_0x4C = 0`, `this->field_0x50 = 0`,
   `this->field_0x48 = 0`; if `AttachedAnim (+0x44)` is non-null call its
   vtable `+0xF8` (anim remove) then null it.
9. If `this->InGlobalList (+0x54)` is set, remove `this` from the
   `DAT_00B0F5B8` active-attach vector, decrement `DAT_00B0F5C8` count.
10. Clear victim state: `victim->field_0x328 = 0`, re-arm victim's
    `+0x6A0..+0x6A8` idle timer, `victim->ParasiteAttacker (+0x694) = 0`,
    `this->Victim = 0`.

---

## 9. Known callers of Detach (xref `0x0062A4A0`)

| Address | Caller                               | Reason                                                |
|---------|--------------------------------------|-------------------------------------------------------|
| 0x006CC7B7 | SuperClass::Launch                 | Superweapon (Chronosphere) strips parasites from warped targets |
| 0x00710058 | TechnoClass::PerformDeploy         | Target deploys (e.g. MCV deploy) — parasite kicked off |
| 0x00629C62 / 0x0062985F / 0x00629E69 | TemporalClass::AI | Shared base sibling — temporal warp clears parasites on targets |
| 0x006F4DA6 | TechnoClass::Receive_Radio         | Radio message `0x17` (enter transport) and siblings — parasite cannot follow into transport |
| 0x004D735F / 0x004D740E | FootClass::ReceiveDamage | Healing or warhead-force-eject paths (see §6)        |
| 0x004DEB6E | TechnoClass::StartFidget           | Fidget/anim-reset — rare, typically dead-code in YR   |
| 0x007195CF | TeleportLocomotionClass::InitiateWarp | Chronosphere teleport on the victim forces detach  |
| 0x0073A18D | UnitClass::Mission_Enter           | Victim enters any transport/refinery/building        |

No standalone game-loop tick calls Detach. All paths are event-driven.

---

## 10. TechnoTypeClass-side INI gating

| INI key                | TypeClass offset | Description                                                             |
|------------------------|------------------|-------------------------------------------------------------------------|
| `Parasiteable`         | `+0xD38`         | Victim type must be parasiteable (verified via CanAttach + ReadINI)    |
| `Naval`                | `+0xCCE`         | Attacker-type naval flag; Squid-path indicator in Detach/Attach        |
| `SuppressionThreshold` | `+0xD6C`         | Attacker-type damage threshold that arms the DmgTimer in `ReceiveDamage` block B (see §6) |
| `ImmuneToPoison`       | (reader ~0x843704 referenced) | Disables Poison AND Parasite on this type              |

`+0xD38 Parasiteable` is newly verified (not present in earlier reports under a
correct offset). `+0xD6C` reference is confirmed from the ReceiveDamage
decompile but its INI key name is not yet identified — candidate is
`ParasiteEjectDamage` or similar; flagged for follow-up.

---

## 11. Tiberian Sun legacy notes

ParasiteClass / WarpAttachClass itself is **not** TS legacy — it is live code
in standard YR for Terror Drone and Giant Squid. However a few code paths
within the file are TS-era or modded-unit shaped:

- The `TemporalClass::AI` fallback inside `UpdateAttack` (§5) requires
  `TechnoType->field_0xCCE && TechnoType->field_0xD97`. Neither Terror Drone
  nor Giant Squid set both. Treat as dormant unless verified in a mod.
- `TechnoClass::StartFidget` as a Detach caller looks like TS inheritance — YR
  fidget animations don't normally reach the WarpAttachClass path.
- Random-direction placement at §7 step 1 uses a 3-of-8 direction table that
  the Giant Squid never reaches (naval path goes through `+0xCCE` instead).

---

## 12. Implementation hints for Rust (research only, not a plan)

The Rust port can model ParasiteClass as a small state machine per parasite
attack, not as a per-entity sub-object. The only persistent state needed is:

- `attacker: EntityId` (must exist, InLimbo while attached)
- `victim:   EntityId`
- `dmg_timer: CDTimer` armed by host-damage events (not per tick)
- `atk_timer: CDTimer` cycling at `attacker.weapon.rof`
- `eject_threshold: i32` (from attacker TypeClass+0xD6C)
- `idle_recoil: CDTimer` set on detach, duration `rof*3`

Dispatch cadence must match gamemd: the victim's AI ticks the parasite, not
the attacker's AI (the attacker is in limbo). A single per-tick pass over
`victim.parasite_attacker.is_some()` reproduces the behaviour.

Do not fabricate a "parasite list" as a first-class system — gamemd stores
`ParasiteClass*` as a manager slot on the attacker FootClass and a back-pointer
on the victim. The global `DAT_00B0F5B8` vector is an index for Detach cleanup,
not a gameplay driver.

---

## 13. Confidence summary

| Claim                                                                            | Confidence |
|----------------------------------------------------------------------------------|------------|
| FootClass+0x69C = ParasiteClass* on the **attacker**                             | 99% (Init_Managers write + FootClass::AI read + existing docs) |
| FootClass+0x694 = attacker FootClass* on the **victim**                          | 99% (Attach writes it, AI dispatch reads it, Detach clears it — 3 independent sites) |
| ParasiteClass size = 0x58                                                        | 95% (operator_new(0x58) in Init_Managers)                 |
| Owner @ +0x24, Victim @ +0x28                                                    | 99% (ctor + all callers)                                  |
| DmgTimer @ +0x2C..+0x34, AtkTimer @ +0x38..+0x40                                 | 90% (UpdateAttack + ReceiveDamage agree on offsets and CDTimer shape) |
| Damage cadence = attacker weapon ROF                                             | 90% (written from Weapon+0xB0 each fire)                  |
| Detach places attacker at victim-adjacent cell                                   | 85% (direction-table + cell-passability logic)            |
| Naval (`+0xCCE`) path kills attacker instead of placing                          | 95% (INI key verified from ReadINI; behavior matches Squid) |
| Eject threshold = `SuppressionThreshold` (`+0xD6C`)                              | 95% (INI key verified from ReadINI; read in ReceiveDamage) |
| Sonic warhead (`+0x14B`) forces eject                                            | 95% (flag name verified from WarheadReadINI + Detach path)  |

---

## 14. Follow-ups for later iterations

- ~~Identify the INI key at `TechnoTypeClass+0xD6C`~~ — **done 2026-04-19: `SuppressionThreshold`.**
- ~~Confirm `TechnoTypeClass+0xCCE`~~ — **done 2026-04-19: `Naval`.**
- ~~Confirm `Warhead+0x14B`~~ — **done 2026-04-19: `Sonic`.**
- AMEND `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` to correct the `+0x694 = Team*` claim (is `ParasiteAttacker*`) and `+0x69C = unknown` (is `ParasiteClass*`).
- AMEND `WARHEAD_DETONATE_GHIDRA_REPORT.md` §Parasite section: `target->ReceiveParasite() // vtable 0x3C8` is wrong — the real attach is a standalone call to `WarpAttachClass::Attach` at `0x0062A980`, and the `FUN_0071AF20 // additional parasite logic` note actually names `TemporalClass::InitiateWarp` in the Temporal branch.

---

## Change log

- **2026-04-19, initial write.**
- **2026-04-19, continuation.** Closed the "where does `victim+0x694` get set" gap by locating `WarpAttachClass::Attach` at `0x0062A980` (previously `FUN_0062A980`) and its `CanAttach` predicate at `0x0062A8E0`. Added §7/§7.1/§7.2. Corrected `Parasiteable` offset: was listed as "(reader at 0x714F86)" in `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT §7.2` with no byte offset on TypeClass, now confirmed to be **TypeClass+0xD38**, verified via (1) `CanAttach` predicate at `0x0062A8E0` reading the flag, and (2) a consistent size/position slot adjacent to the other parasite-related type fields at `+0xD6C`. Added §7.2 correction note about `WARHEAD_DETONATE`'s incorrect `vtable+0x3C8 = ReceiveParasite` claim (real vtable+0x3C8 is `ScatterFromMindControl`, confirmed in `CLOAKING_INTERACTIONS_REPORT`). Upgraded `victim+0x694` confidence from 95% to 99% after third independent confirmation.
- **2026-04-19, third pass.** Identified three INI keys that were unnamed in the prior write: `Warhead+0x14B = Sonic` (verified via `WarheadTypeClass::ReadINI_Body` string/offset pairing and its use in `FootClass::ReceiveDamage` block A), `TechnoTypeClass+0xCCE = Naval` (verified via `TechnoTypeClass::ReadINI` string/offset pairing and its use in `CanAttach`/`Detach` naval-branch discriminator), `TechnoTypeClass+0xD6C = SuppressionThreshold` (verified via `TechnoTypeClass::ReadINI` string/offset pairing and its use in `FootClass::ReceiveDamage` block B threshold compare). All three confidences now 95%. Two independent angles used for each.
