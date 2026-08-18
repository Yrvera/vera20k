# Selection Lifecycle — When Does a Unit Auto-Leave `g_CurrentObjects`?

**Primary addresses:**
- `ObjectClass::Conceal` — `0x005F4D30` (the actual Deselect caller)
- `FUN_0065AA80` — Limbo tail (calls Conceal)
- `ObjectClass::UnInit` — `0x005F65F0` (destruction entry point, calls Limbo)
- `TechnoClass::ChangeOwner` — `0x007014A0` (explicit Deselect on owner-change)
- `ObjectClass::Destroy` — `0x005F5280` (conditional Deselect on destruction)

**Overall confidence:** HIGH — every trigger traced end-to-end to the `vtable+0x150` (Deselect) call.

**Active in YR:** Yes. All of these fire in a standard YR skirmish.

---

## 1. Overview

Three canonical paths cause a unit to automatically leave `g_CurrentObjects`:

1. **Limbo chain** — Entering a transport, garrison, bunker; parasite host entry;
   deploy/undeploy morph; unit destruction; chrono-miner teleport. Each calls
   `vtable+0xD4` (Limbo), which chains to `ObjectClass::Conceal`, which calls
   `vtable+0x150` (Deselect) and then sets `+0x81 = InLimbo`.

2. **ChangeOwner chain** — Engineer capture, mind control (via CaptureManager),
   Spy infiltration owner-change. `TechnoClass::ChangeOwner` explicitly calls
   Deselect *at the top* if the previous owner was the local player and the
   unit was selected.

3. **Destroy path** — `ObjectClass::Destroy` conditionally calls Deselect (skipped
   for human-player-owned objects because those go through Limbo → Conceal first).

**Notably absent from the auto-deselect list:** Chrono-warp. The teleport
locomotion sets `+0x271 IsBeingWarped` but does **not** Limbo the unit. Chrono
Legionnaires stay selected through the entire warp animation, which is the
visible original-game behavior.

This invalidates a claim in the first `SELECTION_SYSTEM_GHIDRA_REPORT` draft
that "there is no call to Deselect in the Limbo path." There is — it's just
one tail-call level deep. See Correction in §7.

---

## 2. Key Offsets Involved

| Offset | Size | Field | Read/written by |
|---|---|---|---|
| `+0x81` | byte | **InLimbo** | Set=1 at end of Conceal; gate for re-entry |
| `+0x83` | byte | **IsSelected** | Cleared by Deselect (vtable+0x150) |
| `+0x87` | ptr  | **Owner (HouseClass\*)** | Written by ChangeOwner; compared against `g_PlayerPtr` |
| `+0x90` | byte | **IsAlive** | Cleared to 0 by UnInit |
| `+0xB0` | ptr  | **MindControlledBy** (TechnoClass\*) | Written by CaptureManagerClass |
| `+0x271` | byte | **IsBeingWarped** | Set by TeleportLocomotionClass::InitiateWarp; **does not trigger deselect** |
| `+0x41A` | byte | **IsLocalPlayerOwned** | Written by ChangeOwner (1 if new owner == g_PlayerPtr) |
| `+0x41B` | byte | **IsControlledByHuman** | Gate in TechnoClass::Select (separate flag) |

---

## 3. Core Logic (the three auto-deselect paths)

### 3.1 Limbo / Conceal chain

`ObjectClass::Conceal` at `0x005F4D30`:

```pseudocode
int Conceal(this):
    if !g_GameActive OR this.InLimbo != 0:
        return 0                              // already limbo — no-op

    this.vtable[0x150]()                      // ← DESELECT
    this.vtable[0xDC](1)                      // Mark/unmark cell occupation
    this.vtable[0x124](0)                     // clear some state machine
    DisplayClass.RemoveFromLayer(this)
    AnimClass.Detach()                        // unhook attached anims
    VocHandle.Stop()                           // stop voice/sfx
    [type-specific cleanup via vtable+0x88 → TypeClass]
    Tactical.DirtyScreenRect(bbox, 1)
    this.vtable[0x11C]()                      // yet more cleanup
    this.InLimbo = 1                          // ← SETS LIMBO FLAG
    this.field_0x80 = 0
    return 1
```

**`FUN_0065AA80`** is the Foot/Unit/Infantry/Aircraft Limbo tail:

```pseudocode
void FUN_0065AA80(this):
    if !this.InLimbo:
        this.vtable[0x280](3)                 // pre-limbo hook (param = mission?)
    ObjectClass.Conceal(this)                 // ← chains to the deselect
```

Call chain from `FootClass::Limbo` (`0x004DB260`) → `FUN_006F6AC0` (`TechnoClass::Limbo`
helper) → `FUN_0065AA80` → `Conceal` → `vtable+0x150` (Deselect).

`UnitClass::Limbo`, `InfantryClass::Limbo`, `AircraftClass::Limbo` all delegate to
`FootClass::Limbo`. `BuildingClass::Limbo` has its own path but ultimately chains to
Conceal as well.

### 3.2 ChangeOwner path

`TechnoClass::ChangeOwner` at `0x007014A0`:

```pseudocode
void ChangeOwner(this, new_owner):
    if new_owner == this.Owner:
        return                                // no-op

    if this.Owner == g_PlayerPtr AND this.IsSelected != 0:
        this.vtable[0x150]()                  // ← DESELECT

    this.vtable[0x3C8]()                      // clear Archive target
    this.vtable[0x480]()                      // abandon production
    ...
    this.Owner = new_owner
    this.field_0x41A = (new_owner == g_PlayerPtr) ? 1 : 0
    ...
```

**Interpretation:** The deselect only fires when the losing owner was the local
player. For spectators or AI losing a unit, no deselect happens (because the
selection list belongs to the local player and nothing needs cleanup).

`BuildingClass::ChangeOwner` (`0x00448260`) is a larger function handling
building-specific side-effects (walls, power, radar, sell refunds), and it
tail-calls `TechnoClass::ChangeOwner` for the owner update — so buildings use
the same Deselect gate via delegation.

### 3.3 Destroy path

`ObjectClass::Destroy` at `0x005F5280`:

```pseudocode
void Destroy(this, param2):
    ...LineTrail cleanup...
    owner = this.GetOwner()
    skip_deselect = false
    if param2 == 0:                           // standard destruction
        if this.IsFoot AND this.vtable[0x328]():
            skip_deselect = true              // attached/embedded in something
        if owner != NULL AND (owner.IsHumanPlayer OR skip_deselect):
            goto cleanup                      // ← SKIPS Deselect
    this.vtable[0x150]()                      // ← DESELECT (fallback path)
cleanup:
    if DisplayClass.LastRefObject == this:
        DisplayClass.SetLastRefObject(0)
```

**Why skip for human-player-owned?** Because that object will shortly go through
`UnInit → Limbo → Conceal`, which deselects on its own. Calling Deselect here
would be redundant.

### 3.4 UnInit (the destruction entry point)

`ObjectClass::UnInit` at `0x005F65F0`:

```pseudocode
void UnInit(this):
    if this.AttachedBomb: BombClass.Defuse()
    if this.IsFoot:       FootClass.EMPPassengers(0)
    FUN_007258D0(this)                         // some global cleanup
    this.vtable[0xD4]()                        // ← LIMBO (which deselects)
    this.IsAlive = 0                           // +0x90 cleared
    PendingDeleteList.push(this)               // deferred cleanup
```

So the destruction flow is **UnInit → Limbo → Conceal → Deselect**. Fires on
unit death, building demolition, sold buildings, MCV that deploys, and any
explicit `Destroy` call.

---

## 4. Per-event behavior (what's auto-deselected vs what's not)

| Event | Auto-deselect? | How | Evidence |
|---|---|---|---|
| Unit death (HP reaches 0) | **Yes** | UnInit → Limbo → Conceal → Deselect | `ObjectClass::UnInit` at `0x005F65F0` |
| Building demolished | **Yes** | same chain | same |
| Building sold | **Yes** | Destroy/UnInit path | same |
| Unit enters transport (IFV, Flak, Amphibious) | **Yes** | Limbo → Conceal → Deselect | FootClass::Limbo chain |
| Unit enters garrison / bunker | **Yes** | Limbo chain (occupant is `Limbo`'d) | GARRISON_OCCUPANT_SYSTEM doc + Conceal trace |
| Terror Drone enters host (parasite) | **Yes** | Limbo chain (drone becomes invisible via Limbo) | PARASITE_CLASS doc + Conceal trace |
| MCV deploys → ConYard | **Yes** | MCV UnInit'd; new BuildingClass is a fresh object, not in selection | UnInit path |
| Prism Tank deploys | **Yes** (the UnitClass) | Same — deployed form is a different object | UnInit path |
| GI deploys (garrison-ready form) | **No** | The same InfantryClass stays; no Limbo. Mission changes, mutation. | FootClass::Limbo not called |
| Engineer captures building | **Yes** | TechnoClass::ChangeOwner → Deselect if old owner was local | `0x007014D5` |
| Spy infiltrates (no owner change on infiltrator) | **No** | Infiltrator returns and is still alive/selected | — |
| Mind control (enemy Yuri on our unit) | **Conditional** (see §7 Open Q) | Likely via CaptureManager → vtable+0x3D0 eventually calling ChangeOwner; not fully verified | CaptureManagerClass::CaptureUnit at `0x00471D40` |
| Chrono-warp (Chrono Legionnaire, Chrono Miner) | **No** | InitiateWarp sets `+0x271` but never calls Limbo or Deselect | `TeleportLocomotionClass::InitiateWarp` at `0x00719400` |
| Chronosphere teleport (by player) | **No** (likely) | Same locomotion path | same |
| IronCurtain applied | **No** | Only sets duration; visual tint only | IRONCURTAIN_FORCESHIELD doc |
| Cloak activated | **No** | Cloak state toggle; unit remains selectable to owner | CLOAKING_STEALTH_SYSTEM doc |
| Magnetron lift (Yuri levitating a vehicle) | **Depends on owner** | If an enemy Magnetron lifts you, no explicit deselect; enemy tanks ≠ your selection anyway | MAGNETRON_SYSTEM doc |
| Player defeated | **Explicit** | `FUN_00637270` and `FUN_0063A4B0` call `Unselect_All()` on observer transition | SELECTION_SYSTEM report |
| Observer mode toggle | **Explicit** | Same two functions | SELECTION_SYSTEM report |

---

## 5. Integration Points

### Who calls Conceal

| Caller | Context |
|---|---|
| `FUN_0065AA80` | Foot/Unit/Infantry/Aircraft Limbo tail |
| `AnimClass::Limbo`, `AnimClass::Constructor` | Animation lifecycle |
| `BulletClass::Constructor` | Bullet creation (bullets enter/leave limbo) |
| `ParticleSystemClass::Constructor` | Particle system init |
| `TerrainClass::Limbo` | Terrain objects (trees, etc.) |
| `OverlayClass::Destructor` | Ore overlays, etc. |
| `BuildingLightClass::Destructor` | Building light instance destruction |
| `VoxelAnimClass::Destructor` | Voxel animation destruction |
| `FUN_00437030` | (BuildingLight-related) |
| `FUN_006B501A` | One additional call site (not traced) |

### Who calls Limbo directly (`vtable+0xD4`)

Most callers are internal to the UnInit/Destroy chain. A minority of game events
invoke Limbo explicitly — MCV deploy, transport passenger absorption, garrison
occupant install.

### Who calls ChangeOwner

- `EngineerClass`-related capture code (see ENGINEER_CAPTURE doc)
- Trigger actions / mission scripts
- Crate pickups that transfer ownership (rare)
- Superweapon effects (none in standard YR)

---

## 6. Current Rust Implementation Status

Rust's current auto-deselect points (from scan):

| Trigger | gamemd.exe | Rust `src/` |
|---|---|---|
| Entity death | Always (via UnInit → Limbo → Conceal) | `combat/mod.rs:468` `process_deaths()` sets `selected=false` before animating |
| Transport passengers killed when transport dies | Via Limbo chain on the transport (passengers get EMP'd first) | `combat/mod.rs:428` also clears passenger `selected` — matches intent |
| Entering transport | Always (Limbo chain) | **Missing** — `PassengerRole::Inside{..}` doesn't clear `selected` |
| Entering garrison / bunker | Always (Limbo chain) | **Missing** — `garrison_slot` doesn't clear `selected` |
| MCV deploy | New BuildingClass is fresh; MCV UnInit'd | `world_spawn.rs` despawns MCV — effectively matches |
| Engineer capture / sell | ChangeOwner or Destroy path | Partial — `execute_capture` despawns entity |
| Mind control (enemy) | Via ChangeOwner if path confirmed | **Missing** — not implemented |
| Chrono-warp | **No auto-deselect** (sets `+0x271` only) | `teleport_state` doesn't clear `selected` — matches original |
| Defeat / observer transition | Explicit `Unselect_All` | **Missing** — no observer mode yet |

### Concrete gaps to fix

1. **Transport-enter / garrison-enter / parasite-enter should deselect** — these
   are the main playable bugs. Currently a unit that walks into a Flak Track
   stays in our selection list.
2. **Engineer capture path** — we despawn the engineer on capture which is fine,
   but the captured BUILDING changes owner and should be deselected if the
   previous owner was the local player.
3. **Mind control** — not implemented in sim yet; when it is, it must call the
   equivalent of `ChangeOwner → Deselect`.

### Things we already got right (by accident or design)

- Entity death clears `selected` — matches Conceal.
- `dying: bool` flag exists — matches the "already marked for cleanup" semantic.
- Chrono-warp does not clear selection — matches the original's quirk.

---

## 7. Open Questions — all resolved

1. **Mind control → ChangeOwner path** — **resolved**. The relevant vtable slot
   is `+0x3D4`, NOT `+0x3D0`. `CaptureManagerClass::CaptureUnit` at
   `0x00471D40` calls `(**(iVar5 + 0x3D4))(uVar3)` — that's
   `TechnoClass::ChangeOwner(attacker_owner)`. The paired restore call is in
   `CaptureManagerClass::FreeUnit` at `0x00471FF0`:
   `(**(*param_2 + 0x3D4))(piVar2[1], 1)` — `ChangeOwner(original_owner, 1)`.
   So mind control auto-deselects via the normal ChangeOwner path. The Yuri
   takes-your-unit color change happens because the Owner field (+0x87) flips.

   (The nearby `vtable+0x3D0` call, which I earlier confused for ChangeOwner,
   is actually `FUN_0070F850` — a "clear archive/production/mission targeting"
   helper called during various state resets.)

2. **`vtable+0x124` slot** — **resolved: `TechnoClass::DoCloak` at `0x004D3780`**.
   Takes an int state parameter (0=uncloak, 1=cloak, 2=no-op, 3=reset).
   Conceal calls `vtable+0x124(0)` to force-uncloak before hiding a unit.

3. **`vtable+0x328` slot** — **resolved: `SensorCountForHouseAtMyCell` at
   `0x0070D420`**. Returns the sensor count for the local player at the
   object's current cell. Used by `ObjectClass::Destroy` to check whether
   the dying object is within the player's sensor range. (The "skip-deselect"
   logic is: if player-owned OR sensor-visible, skip the explicit Deselect
   because the Limbo chain handles it.)

4. **Chrono-warp mid-warp Limbo status** — **resolved: does NOT set InLimbo**.
   `TeleportLocomotion::InitiateWarp` only sets `+0x270` (IsWarpingOut) and
   `+0x271` (IsBeingWarped). The selection block comes from `vtable+0x1D4`
   (`IsWarpingOut`) being consulted by `ObjectClass::Select` — existing
   selection persists, but re-selection during the warp gap is rejected.

5. **Observer/spectator transitions `FUN_00637270` and `FUN_0063A4B0`** —
   both in `PlanMgr.cpp` (confirmed via the string `s_D:\ra2mdpost\PlanMgr_cpp`
   used by both). `FUN_00637270` is called from `Main_Tick` and processes the
   AI planning manager's tick (plan progress, phase transitions). When the
   plan ends or transitions, it calls `Unselect_All` and shows a HUD message
   from string table 0xCA1. `FUN_0063A4B0` has no xrefs — likely invoked via
   function pointer from campaign scripting.

---

## Sources

**Ghidra addresses decompiled this investigation:**

| Address | Name | Purpose |
|---|---|---|
| `0x005F4D30` | `ObjectClass::Conceal` | Central deselect + set InLimbo |
| `0x005F5280` | `ObjectClass::Destroy` | Conditional deselect on destruction |
| `0x005F65F0` | `ObjectClass::UnInit` | Destruction entry → Limbo → Conceal |
| `0x0065AA80` | (Limbo tail) | Calls Conceal at end of Foot/Unit Limbo |
| `0x006F6AC0` | (TechnoClass::Limbo helper) | Calls FUN_0065AA80 at end |
| `0x004DB260` | `FootClass::Limbo` | Calls FUN_006F6AC0 at end |
| `0x007440B0` | `UnitClass::Limbo` | Delegates to FootClass::Limbo |
| `0x007014A0` | `TechnoClass::ChangeOwner` | Explicit Deselect on owner change |
| `0x00448260` | `BuildingClass::ChangeOwner` | Tail-calls TechnoClass::ChangeOwner |
| `0x00471D40` | `CaptureManagerClass::CaptureUnit` | Mind control — sets MindControlledBy |
| `0x004723B0` | `CaptureManagerClass::DecideUnitFate` | AI branch post-capture |
| `0x00719400` | `TeleportLocomotionClass::InitiateWarp` | Chrono-warp; sets +0x271 only |

**Adjacent reports referenced:**
- `SELECTION_SYSTEM_GHIDRA_REPORT.md` (predecessor; contained the incorrect
  "Limbo does not deselect" claim that this report refutes)
- `SELECTION_GATES_GHIDRA_REPORT.md` (companion; covers the static/dynamic gate
  functions)
- `OBJECTCLASS_GHIDRA_REPORT.md` (IsSelected flag origin)
- `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`
- `BUNKER_SYSTEM_GHIDRA_REPORT.md`
- `PARASITE_CLASS_GHIDRA_REPORT.md`
- `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`
- `ENGINEER_CAPTURE_GHIDRA_REPORT.md`
- `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
- `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`
- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `TELEPORT_LOCOMOTION_DEEP_DIVE.md`

---

## Correction to the first SELECTION_SYSTEM report

The §3 "IsSelected flag" subsection stated:

> "**Unit-death does not call this [Deselect] directly**; dead/limboed units keep
> `IsSelected=1` but are ignored because the InLimbo flag (+0x81) gates all
> follow-up actions."

**This is wrong.** Unit death does call Deselect — indirectly through:
`UnInit` → `vtable+0xD4 (Limbo)` → `FUN_0065AA80` → `ObjectClass::Conceal` →
`vtable+0x150 (Deselect)`. The `IsSelected` byte IS cleared on death.

The predecessor report's confusion was from reading the local body of `Unselect_All`
and not chasing the Limbo chain. Fixed here.
