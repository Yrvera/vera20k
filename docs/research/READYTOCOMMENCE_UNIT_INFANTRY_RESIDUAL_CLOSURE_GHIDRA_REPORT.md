# UnitClass / InfantryClass ReadyToCommence Residual Closure — Ghidra Research Report

**Target question:** What exact active-YR inputs and concrete locomotor mechanisms complete production-safe `UnitClass::ReadyToCommence` (`0x00744270`) and `InfantryClass::ReadyToCommence` (`0x00521B60`)?
**Non-goals:** Full mission dispatch; full locomotor algorithms; fear AI beyond predicate inputs; Aircraft/Building readiness; Unit `+0x6D1/+0x6E1/+0x6E2`, Infantry `+0x2B4`, or Mission `+0xB8` writer lifecycles.
**Evidence needed to mark COMPLETE:** Each scoped field/table/branch and every active standard-YR concrete slot `+0x80` implementation is closed with binary body plus assembly/caller/xref evidence, stock/default activation is classified, persistence is stated where directly evidenced, and both class predicates have concrete allow/block fixtures.
**Stop conditions:** Stop only after the scoped open-question log contains no open item and a zero-add pass over both predicates plus discovered scoped callees adds nothing; otherwise publish `PARTIAL` with each unresolved item explicit.

**Status:** **COMPLETE** for the bounded Unit/Infantry readiness slice.
**Date:** 2026-07-21.
**Binary:** active Yuri's Revenge `gamemd.exe`; Ghidra access was read-only.
**Implementation:** research only. No Rust source was changed. The pre-existing uncommitted `src/sim/world/techno_ai.rs` change was not touched.

## 1. Outcome

The remaining Unit and Infantry readiness inputs are now closed:

1. `FUN_004A51D0` is not a global, spy-plane, or paradrop-mode query. `UnitClass::ReadyToCommence` passes `unit+0x350`; the helper returns true only when the embedded deploy/door animation tracker bytes at Unit `+0x368/+0x369` are both zero. Any active tracker blocks readiness.
2. The rest of that Unit branch is a live war-factory production-exit interlock. It reads Radio contact slot 0, recognizes a contacted `BuildingClass` whose type has `WeaponsFactory=yes`, and permits only queued `Move(2)` or `Enter(7)`. With no contact, it blocks at the exact fallback cell `(building_cell_x, building_cell_y + 1)`.
3. The active stock Unit/Infantry locomotor union for this virtual call contains six concrete classes: Drive, Ship, Hover, Walk, Teleport, and Jumpjet. Their `ILocomotion+0x80` bodies are not one generic “is driving” test.
4. Infantry `+0x68D` is a Foot-wide one-byte firing-sequence latch. Infantry sets it when a legal shot sequence is armed and clears it before the firing virtual or when the target/sequence is aborted or replaced. It is not a fear byte, a generic action byte, or merely a deploy-refresh flag.
5. Object `+0x8D` is `IsFallingDown` / fall-height-settle-active. Constructor initializes it to zero; `ObjectClass::Unlimbo` and `ObjectClass::DropIn` set it; `ObjectClass::AI` clears it on landing. Infantry readiness blocks while it is nonzero.
6. Infantry `+0x6C4` is the current animation `Doing` index. For the requested range: 27=`Deploy` and is denied by the readiness table; 28=`Deployed`, 29=`DeployedFire`, and 30=`DeployedIdle`, and all three are allowed.

The current Rust `ReadySnapshot { category, is_driving }` cannot express this contract. It returns unconditional true for Infantry and reduces Unit to `!is_driving`; both are **DRIFT**.

## 2. Evidence Method and Burden of Proof

The investigation began from the local research index and then read the cited source documents, including the prior ReadyToCommence reports, the radio-message report, production-exit research, Guardian GI research, Object/falling research, and current Rust files. Live Ghidra was used to re-open each load-bearing body. Local labels were treated only as navigation hints.

For handoff-critical conclusions, two independent evidence forms were required:

- predicate decompile plus assembly;
- concrete virtual body plus vtable/COL identity;
- exhaustive direct-offset instruction search plus owner-function decompile;
- binary table bytes plus a separate sequence-name/state-machine source;
- binary branch plus stock `rules.ini`/`rulesmd.ini` activation data;
- current Rust direct reads plus binary contract.

Representative calls used in this pass:

- `decompile_function` and `disassemble_function`: `0x744270`, `0x521B60`, `0x4A51D0`, `0x65AD40`, `0x4AFC20`, `0x69F330`, `0x514C80`, `0x75AB40`, `0x4B6610`, `0x54D0D0`, `0x5206B0`, `0x51B1F0`, `0x51BAB0`, `0x51DF70`, `0x520AE0`, `0x5216D0`, `0x5F3900`, `0x5F3E70`, `0x5F4160`, `0x5F5940`, and `0x5F6250`;
- `search_instructions(operand_pattern="0x68d]")`: 21 matches over 1,151,260 parsed instructions, not truncated;
- `search_instructions(operand_pattern="0x8d]")`: 51 matches over 1,151,230 parsed instructions, not truncated, then filtered by receiver/class context;
- `read_memory(0x7EAF7C, ...)` for the sequence property table;
- vtable/COL/type-descriptor walks and constructor writes for all six locomotors;
- `rg` over merged stock INI type lists and object sections, plus direct Rust reads.

## 3. Exact Leaf Predicates

### 3.1 UnitClass `0x00744270`

The Ghidra name `UnitClass__ShouldIdle` is stale in this context. Vtable placement and the caller contract identify this body as the Unit leaf `ReadyToCommence` override.

Rust-facing pseudocode, preserving branch order:

```text
unit_ready(u):
  if u.current_mission in {6, 21}: return false
  if u.byte_6E1 != 0: return false
  if u.byte_6E2 != 0: return false
  if u.byte_6D1 != 0: return false

  if u.queued_mission != 7:
    moving = u.active_locomotor.is_moving_now()
    if moving
       and u.height >= 0
       and u.current_mission != 5
       and (u.current_mission != 1 or u.attack_target != null)
       and u.mission_byte_B8 == 0:
      return false

  if u.deploy_door_tracker.byte_18 != 0
     or u.deploy_door_tracker.byte_19 != 0:
    return false

  contact0 = u.radio_contacts.slot(0)
  if contact0 != null:
    if contact0.WhatAmI() == Building(6)
       and contact0.type.WeaponsFactory
       and u.queued_mission not in {Move(2), Enter(7)}:
      return false
  else:
    b = building_under(u.coord)
    if b != null
       and b.type.WeaponsFactory
       and cell16_x(u.coord.x) == cell16_x(b.coord.x)
       and wrapping_i16(cell16_y(u.coord.y) - cell16_y(b.coord.y)) == 1:
      return false

  return true
```

Important exact details:

- The excluded current missions are numeric 6 and 21 (`0x15`). Existing mission research maps these to Sticky and Rescue; they are not “Sleep and unknown.”
- Queued `Enter(7)` bypasses the locomotor test entirely, but it still reaches the deploy/door and factory-exit checks.
- A moving Unit does not unconditionally fail. Guard(5), Attack(1) with no attack target, negative height, or nonzero Mission `+0xB8` bypass that one movement rejection branch.
- The no-contact fallback uses gamemd's signed conversion `(v + ((v >> 31) & 0xFF)) >> 8`. It requires equal X cells and a signed Y-cell delta of exactly `+1`; it is not “inside any factory footprint.”
- After the signed conversion, native stores each cell component with `MOV
  word`; equality and subtraction therefore use the low 16 bits with wrapping
  `i16` difference semantics. Fresh `0x007443EC..0x00744459` assembly on
  2026-07-23 confirms `MOV word`, `TEST AX`, and `CMP CX,1`.
- Evidence: `decompile_function(0x744270)` plus full function assembly; exact signed compares in the assembly; helper/contact bodies below.

### 3.2 InfantryClass `0x00521B60`

```text
infantry_ready(i):
  if i.current_mission in {6, 21}: return false
  if i.firing_sequence_latch_68D != 0: return false
  if i.object_is_falling_down_8D != 0: return false

  moving = i.active_locomotor.is_moving_now()
  effective = i.current_mission if i.current_mission != -1 else i.queued_mission
  if moving and effective not in {Guard(5), Hunt(15)}:
    if effective != Attack(1): return false
    if i.attack_target != null: return false

  doing = i.doing_6C4
  if doing == -1: return true
  return sequence_property_table[doing].ready_allowed != 0
```

Important exact details:

- Only `-1` is special-cased before table indexing. The predicate has no general bounds check; native callers/state writers are expected to maintain a valid Doing index.
- The initial Sticky/Rescue gate reads raw current. The moving exceptions instead
  call Infantry vtable `+0x184` three times (`0x00521BBE..0x00521BDC`).
  Infantry's slot resolves to `MissionClass::GetCurrentMission @ 0x005B3040`,
  which returns current `+0xAC` unless it is `-1`, then queued `+0xB4`.
- Moving Infantry therefore remains eligible under effective Guard(5), Hunt(15),
  or Attack(1) with no attack target. A moving attacker with a live target is not
  ready.
- Evidence: fresh `decompile_function(0x521B60)` and
  `decompile_function(0x5B3040)` on 2026-07-23, full function assembly, Infantry
  vtable slot bytes, and raw table bytes at `0x7EAF7C`.

## 4. Unit Deploy/Door and WeaponsFactory Branch

### 4.1 `FUN_004A51D0` is an embedded tracker-idle test

`UnitClass::ReadyToCommence` loads `ECX = unit + 0x350` before the call. The helper body is exactly:

```text
return byte[this+0x18] == 0 && byte[this+0x19] == 0
```

Therefore it reads Unit `+0x368/+0x369`. Independent radio/deploy research binds the embedded `+0x350` object to the Unit deploy/door animation timing tracker: `0x4A5240` activates it and `0x4A51D0` tests it idle. The earlier global/spy-plane/ParaDrop interpretation is false.

Active standard YR: **Yes, conditional**. Any Unit whose tracker is active is not ready. The branch is also present for ordinary produced vehicles; tracker-idle is required before the factory-exit sub-branch can allow readiness.

### 4.2 Radio contact slot 0

`FUN_0065AD40` returns `*(this+0xE4)[index]`. In the Unit call, `index=0`, so this is RadioClass contact slot 0, not a global object or owner pointer. Current Rust already has a capacity-bounded `Contacts` slot store and can read `radio_contacts.slot(0)`; membership-only `contains` is insufficient here because the native predicate asks specifically for slot 0.

### 4.3 `BuildingType+0x16BD` is `WeaponsFactory=yes`

The contacted-object branch requires `WhatAmI()==6` (Building), then `BuildingType+0x16BD != 0`. Existing verified type-layout/production research and the stock INI bind this byte to `WeaponsFactory=yes`.

Stock sections carrying `WeaponsFactory=yes` are:

- `rules.ini`: `GAWEAP`, `GAYARD`, `NAWEAP`, `NAYARD`;
- `rulesmd.ini` additions/overrides: the above plus `YAWEAP`, `YAYARD`.

The Ready predicate itself does not test `Naval`. The semantic set therefore includes land war factories and naval yards. Current Rust's production helper `exact_land_vehicle_exit_factory` deliberately excludes naval producers and is not a parity-safe replacement for this predicate's `WeaponsFactory` byte.

Active standard YR: **Yes, conditional and common** during normal vehicle/naval production exit. Existing production research independently proves successful non-naval war-factory creation unlimbos the product, assigns Guard, establishes the Radio link, and enters the exit state machine.

### 4.4 Unit fixtures

All unspecified gates are clear and the deploy/door tracker is idle:

| Fixture | Inputs | Native result |
|---|---|---|
| Contacted factory blocks unrelated queue | contact slot 0=`GAWEAP`, type `WeaponsFactory=yes`, queued=Guard(5) | **false** |
| Contacted factory permits Move | same, queued=Move(2) | **true**, subject to earlier gates |
| Contacted factory permits Enter | same, queued=Enter(7) | **true**, subject to tracker/other gates; movement test bypassed |
| Geographic fallback exact hit | no contact; Unit cell `(40,51)`; `GAWEAP` anchor `(40,50)` | **false** |
| Geographic fallback 16-bit wrap | no contact; Unit raw cells `(65536,65537)`; factory anchor raw cells `(0,0)` | **false** |
| Geographic fallback one-cell miss | no contact; Unit cell `(41,51)` or `(40,52)` | **true**, subject to other gates |
| Active deploy/door tracker | either Unit `+0x368` or `+0x369` nonzero | **false** before contact logic |

## 5. Concrete `ILocomotion+0x80` Implementations

### 5.1 Identity and stock binding census

The virtual is stable at ILocomotion vtable byte offset `+0x80`, but its mechanism is class-specific. Vtable-minus-four COL walks and type descriptors identify:

| Concrete class | ILocomotion vtable | COL / type descriptor | Slot `+0x80` target |
|---|---:|---:|---:|
| DriveLocomotionClass | `0x7E7EB0` | COL `0x7FFDE8`; TD `0x820248` | `0x4AFC20` |
| HoverLocomotionClass | `0x7EACFC` | COL `0x803228`; TD `0x8254B8` | `0x514C80` |
| ShipLocomotionClass | `0x7F2D8C` | COL `0x8093A0`; TD `0x83F880` | `0x69F330` |
| TeleportLocomotionClass | `0x7F5000` | COL `0x80C178`; TD `0x844538` | `0x4B6610` |
| WalkLocomotionClass | `0x7F69F8` | COL `0x80D240`; TD `0x847BF0` | `0x75AB40` |
| JumpjetLocomotionClass | `0x7ECD68` | COL `0x804C88`; TD `0x829648` | `0x54D0D0` |

Walk constructor `0x75AA90` writes `0x7F69F8`; Jumpjet constructor `0x54AC40` writes `0x7ECD68`. The Hover and Jumpjet small slot bodies were not accepted on label trust: where Ghidra lacked a function boundary, raw memory plus `disassemble_bytes(dry_run=true)` was used without creating or changing functions.

A merged base-then-`*md` stock INI census found:

- 80 VehicleTypes: Drive 52, Ship 13, Jumpjet 6, Hover 4, Teleport 3, and two list artifacts without a locomotor binding (`DeathDummy`, `YDUM`);
- 65 InfantryTypes: Walk 60, Teleport 3, Jumpjet 2;
- no active Unit/Infantry type-list binding to Fly, Rocket, Mech, Tunnel, or DropPod.

The Magnetron `LocomotorBeam` path can temporarily install Jumpjet on a target, so Jumpjet's Unit/Infantry relevance is broader than only the statically Jumpjet-bound types. It does not add another concrete predicate.

### 5.2 Exact bodies

#### Drive `0x004AFC20`

Returns true if the owner's embedded turning/timer helper at owner `+0x388` is active. Otherwise it requires all of:

1. Drive slot `+0x10` says moving;
2. the `head_to` coordinate at interface `+0x3C/+0x40/+0x44` is not the exact three-dword null coordinate sentinel;
3. owner virtual `+0x538` returns a signed value greater than zero.

Drive slot `+0x10` at `0x4AFB80` first tests one coordinate triple at interface `+0x30/+0x34/+0x38`; otherwise it examines the second triple and treats matching owner X/Y as stopped (Z is not part of that equality).

Active standard YR: **Yes; dominant stock vehicle path.**

#### Ship `0x0069F330`

The same logical mechanism as Drive, using Ship's own sentinels and slot `+0x10` body at `0x69F290`: turning/timer active, or `Is_Moving && nonnull head_to && owner speed > 0`.

Active standard YR: **Yes; stock naval path.**

#### Hover `0x00514C80`

Returns true only when:

1. Hover slot `+0x10` is true; and
2. the double at interface `+0x44` is ordered and not equal to `0.0`.

The second condition is not a raw-magnitude nonzero test. Fresh disassembly on
2026-07-23 confirms `FLD [ESI+0x44]`, `FCOMP [0x007E2800]`, `FNSTSW AX`,
`TEST AH,0x40`, then `JNZ` to false at `0x00514C8F..0x00514CA8`. Both signed
zero and every NaN/unordered operand set the tested C3 bit and return false;
finite or infinite nonzero values of either sign return true.

The slot `+0x10` bytes at `0x514C30` return false only when both coordinate triples at interface `+0x14..+0x1C` and `+0x20..+0x28` equal the native null-coordinate sentinel.

Active standard YR: **Yes; stock hover vehicles.**

#### Walk `0x0075AB40`

The current Ghidra label `Is_To_Have_Shadow` is stale for this vtable slot. It returns true only when all of:

1. slot `+0x10` (`0x75AB30`) returns the byte at interface `+0x30` as nonzero;
2. owner double `+0x578` is ordered strictly greater than `0.0`;
3. the destination coordinate at interface `+0x24/+0x28/+0x2C` is not the exact null sentinel.

Existing speed research independently binds owner `+0x578` to the current applied speed fraction.

Active standard YR: **Yes; dominant stock infantry path.**

#### Teleport `0x004B6610`

Slot `+0x80` is a thunk to slot `+0x10`; `0x718080` returns true exactly when byte interface `+0x30` (class state `+0x34`) equals 1.

Active standard YR: **Yes; stock chrono Unit and Infantry types.**

#### Jumpjet `0x0054D0D0`

The raw slot body loads dword interface `+0x4C` (class state `+0x50`) and returns true iff the state is neither 0 nor 2.

Active standard YR: **Yes; stock Jumpjet Unit/Infantry types and conditional Magnetron-installed Jumpjet state.**

### 5.3 Shared-interface conclusion

The common virtual is sufficient only if each Rust locomotor variant preserves the concrete state it reads. A single `is_driving`, `movement_target.is_some()`, or `phase != Idle` approximation is not proven equivalent and is therefore **DRIFT**. In particular:

- turning alone can make Drive/Ship report moving;
- Hover additionally requires a nonzero floating/speed double;
- Walk requires a moving byte, positive applied speed, and nonnull destination;
- Teleport tests one exact state value;
- Jumpjet excludes exactly two state values.

## 6. Infantry Residual Fields

### 6.1 Foot `+0x68D`: armed firing-sequence latch

The exhaustive direct-offset scan found the following scoped lifecycle:

| Address | Owner | Write/read | Meaning in context |
|---:|---|---|---|
| `0x4D33C6` | Foot constructor | write 0 | born clear; proves Foot-wide byte |
| `0x51B20E` | Infantry SetTarget override | write 0 | changed target aborts armed sequence |
| `0x51BDE7..0x51BE01` | Infantry AI | read, then write 0 | if animation total reaches zero, clear and restore Deployed/Ready sequence |
| `0x51DF70` | Infantry Fire_At override | write 0 | clears immediately before `TechnoClassFireAtSpawnsBullet` |
| `0x520912` | Infantry Fire_At_Target | write 1 | legal shot sequence selected/armed |
| `0x520A03` | Infantry Fire_At_Target | write 0 | firing no longer legal / abort result |
| `0x520AD2` | Infantry Fire_At_Target | write 0 | no-target early return |
| `0x520DE0`, `0x520DF9` | DoType sequencer | write 0 | prone-fire sequence 40/41 is reselected after movement stops |
| `0x4DBD28` | Foot ComputeChecksum | read | included in the legacy deterministic checksum surface |

`InfantryClass::Fire_At_Target @ 0x5206B0` establishes the closed loop:

1. with no target, clear the byte and return;
2. if clear and firing is legal, select the appropriate body sequence and set it to 1;
3. while set, wait until current animation frame `+0xF8` equals the selected fire-frame anchor;
4. call the firing virtual; `InfantryClass::Fire_At_Override @ 0x51DF70` clears the byte before bullet spawn;
5. target loss, target replacement, animation exhaustion, failed fire validation, or prone-fire reselection also clears it.

`FootClass::Locomotion_AI` and `FUN_005216D0` independently treat it as a busy gate that suppresses locomotion animation changes or other eligibility while a shot is armed. This supports the name **armed firing-sequence latch**; it does not justify a broader generic “busy” meaning.

Active standard YR: **Yes, conditional and common whenever Infantry arms a shot.** Its ReadyToCommence effect lasts from sequence arming until the fire/abort clear.

### 6.2 Object `+0x8D`: `IsFallingDown`

Direct writer/consumer lifecycle:

| Address | Owner | Effect |
|---:|---|---|
| `0x5F3975` | Object constructor | initializes byte to 0 |
| `0x5F416A` | `ObjectClass::DropIn` | sets byte to 1 |
| `0x5F5965` | `ObjectClass::Unlimbo` | sets byte to 1 after the playfield gate |
| `0x5F3F11..0x5F3F86` | `ObjectClass::AI` | if set, integrates Z/fall delta; clears it when effective height is below 1 and landing is committed |
| `0x521B89` | Infantry ReadyToCommence | nonzero blocks readiness |

The same Object AI branch updates Z, applies parachute/no-parachute fall clamps, changes mission on landing, and terminates an attached chute animation. Existing Object-substrate and paradrop reports independently identify the byte as fall/height-settle active. It is not InLimbo (`+0x81`) and not OnBridge (`+0x8C`).

Active standard YR: **Yes, conditional.** It is visible during normal paradrop descent, bridge-collapse `DropIn`, and the Unlimbo height-settle window.

### 6.3 Doing `+0x6C4` and table `0x7EAF7C`

`InfantryClass::DoType_Sequencer @ 0x520AE0` indexes the per-type SequenceData array as `TypeData+0xE3C + Doing*0x24`, proving `+0x6C4` is the current body-sequence index. Its deploy cases, Guardian GI research, live fire selection, and stock art data give the requested mapping:

| Doing | Sequence | `0x7EAF7C + Doing*4` bytes | Ready reads byte 0 | Result |
|---:|---|---|---:|---|
| 27 (`0x1B`) | Deploy | `00 00 00 01` | 0 | **block** |
| 28 (`0x1C`) | Deployed | `01 00 00 01` | 1 | **allow** |
| 29 (`0x1D`) | DeployedFire | `01 00 00 01` | 1 | **allow** |
| 30 (`0x1E`) | DeployedIdle | `01 00 00 01` | 1 | **allow** |

#### Complete permission-table closure (2026-07-23 live recheck)

`decompile_function(address="0x00523D00", program="gamemd.exe")` shows the
sequence-name table iterating from `0x008255C8` to `<0x00825670`, exactly
`0xA8 / 4 = 42` entries. `read_memory(address="0x007EAF7C", length=168,
program="gamemd.exe")` confirms that the Ready property table is therefore 42
four-byte records, indexed `0..=41`; storage beginning at `0x007EB024` is
unrelated adjacent data.

The complete byte-zero permission classification is:

- **allow:** `0,1,2,3,4,6,8,9,10,16,17,18,19,22,23,24,25,26,28,29,30,33,37,38,39,40,41`
- **block:** `5,7,11,12,13,14,15,20,21,27,31,32,34,35,36`

Equivalent first-column bytes in index order:

```text
01 01 01 01 01 00 01 00 01 01 01 00 00 00 00 00
01 01 01 01 00 00 01 01 01 01 01 00 01 01 01 00
00 01 00 00 00 01 01 01 01 01
```

There is still no generalized bounds branch in
`InfantryClass::ReadyToCommence @ 0x00521B60`: only `Doing == -1` bypasses the
table. An authoritative writer must maintain `Doing in {-1, 0..=41}`. Rust must
not clamp or synthesize a result for another raw value; such a value is an
authority invariant failure and blocks production activation of this leaf.

Corroborating state-machine facts:

- completed Deploy case `0x1B` changes to `0x1C`;
- Infantry firing while Doing is `0x1B..0x1E` selects `0x1D`;
- Undeploy is `0x1F`, completes to Ready(0), and is separately denied by table byte 0;
- stock `[GuardianGISequence]` supplies `Deploy`, `Deployed`, `DeployedFire`, and `Undeploy` frames.

Active standard YR: **Yes** for Guardian GI and GI/deployer state machines. The table also applies to every Infantry Doing value, not only deployers.

### 6.4 Infantry fixtures

All unspecified gates are clear:

| Fixture | Inputs | Native result |
|---|---|---|
| Deploy transition | idle Walk locomotor; Doing=27 Deploy | **false** (table byte 0) |
| Stable deployed | idle Walk locomotor; Doing=28 Deployed | **true** |
| Deployed firing pose, latch already clear | idle Walk locomotor; Doing=29 DeployedFire; `+0x68D=0` | **true** |
| Shot armed | same but `+0x68D=1` | **false** before Doing table |
| Falling infantry | Doing=28; Object `+0x8D=1` | **false** |
| Moving under Guard | Walk `Is_Moving_Now=true`; current=Guard(5); Doing=28 | **true** |
| Moving with queued Guard fallback | same; current=None(-1), queued=Guard(5); Doing=28 | **true** |
| Moving under Move | same; current=Move(2) | **false** |
| Moving Attack without target | current=Attack(1); `+0x2B4=null`; Doing=28 | **true** |
| Moving Attack with target | current=Attack(1); `+0x2B4!=null` | **false** |

## 7. Persistence and Determinism — Bounded Statement

Two similarly named native surfaces must not be conflated:

1. `ObjectClass__Save @ 0x5F6250` is a CRC/checksum-style field feeder, not the savegame stream body. It explicitly feeds Object `+0x8D` to the byte helper. `FootClass__ComputeChecksum @ 0x4DBAD0` explicitly feeds Foot `+0x68D` at `0x4DBD28`.
2. Savegame stream persistence uses `FUN_0065AC40 -> AbstractClass::Save @ 0x410320`, which writes raw virtual-class-sized bytes. The raw body mechanically covers Object `+0x8D`, Foot `+0x68D`, and Infantry `+0x6C4` before class-specific load cleanup/constructor behavior.

The bounded readiness investigation did not re-audit final post-load survival for each specific Unit/Infantry loader. Therefore:

- **verified:** the bytes participate in the cited CRC/raw-stream surfaces;
- **not claimed:** that every one survives every native save/load round trip unchanged after class-specific load constructors and fixups;
- **active classification:** raw savegame stream is **Yes, conditional on save/load**; the `FootClass__ComputeChecksum` caller chain is **No in normal active play** according to existing verified caller research (its sole caller is a zero-caller legacy convoy routine).

This distinction is deliberate. Do not cite `0x5F6250` as the savegame serializer.

## 8. Active Standard-YR Classification

| Mechanism | Active YR | Reason |
|---|---|---|
| Unit/Infantry leaf predicates | **Yes** | live vtable overrides used by mission commence authority |
| Unit deploy/door tracker gate | **Yes, conditional** | active when either embedded tracker byte is nonzero |
| Unit WeaponsFactory contact gate | **Yes, common conditional** | normal factory/yards production exit with Radio slot 0 |
| Unit geographic fallback | **Yes, conditional** | no-contact recovery at exact anchor-south cell |
| Drive/Ship/Hover/Walk predicates | **Yes** | stock type-list bindings |
| Teleport/Jumpjet predicates | **Yes, conditional by type/state** | stock bindings; Jumpjet can also be installed by Magnetron path |
| Fly/Rocket/Mech/Tunnel/DropPod for Unit/Infantry Ready | **No stock type-list binding in this slice** | not part of the active stock Unit/Infantry concrete union |
| Foot `+0x68D` readiness gate | **Yes, conditional/common** | armed Infantry fire sequence |
| Object `+0x8D` readiness gate | **Yes, conditional** | falling/height settle |
| Doing permission table | **Yes** | current Infantry sequence gate |
| `FootClass__ComputeChecksum` chain | **No in normal live play** | legacy zero-caller chain; not a per-frame sync hash |
| Savegame raw-body coverage | **Yes, conditional on save/load** | live IPersist stream path; final class-specific byte survival not claimed here |

## 9. Current Rust Disparity

### 9.1 Existing surface

`src/sim/mission/verb.rs` currently defines:

```rust
pub struct ReadySnapshot {
    pub category: ReadyCategory,
    pub is_driving: bool,
}
```

and returns `!is_driving` for Unit, unconditional true for Infantry. This was intentionally a structural placeholder, but the relevant facts are no longer unchecked.

Useful existing Rust state already present:

- `MissionCom.current` / `queued` and `attack_target`;
- `LocomotorState.kind`, ground/air phases, speed fields, Hover throttle, Jumpjet state surfaces;
- `DriveLocomotionRuntime.destination/head_to`, drive-track and turning state;
- slot-preserving `radio_contacts.slot(0)` and `dock_entered_with`;
- object type/position and occupancy/building lookup;
- `parachute_state`, `droppod_state`, and other partial falling state;
- `Animation.sequence` with `SequenceKind::{Deploy, Deployed, DeployedFire, DeployedIdle, Undeploy}`;
- `deploy_state` and combat/fire-frame machinery.

Required state is still missing or not authoritative at this boundary:

- exact per-concrete-locomotor `is_moving_now` methods and any native state those methods cannot yet reconstruct;
- Unit embedded deploy/door tracker state;
- parsed `WeaponsFactory=` byte (current production inference is narrower and excludes naval yards);
- Foot `+0x68D` armed firing-sequence latch with same-tick set/clear order;
- one Object-wide `IsFallingDown`/height-settle gate covering Unlimbo and DropIn, not only parachutes;
- native Doing/permission mapping as an authoritative readiness input;
- the already researched Unit bytes and Mission `+0xB8` integration, which are outside this residual report but still required by the full Unit predicate.

### 9.2 Parity verdict

| Rust behavior | Verdict | Why |
|---|---|---|
| Unit `!is_driving` | **DRIFT** | loses mission exceptions, concrete locomotor mechanisms, three Unit bytes, Mission `+0xB8`, tracker, Radio, WeaponsFactory, and geographic fallback |
| Infantry unconditional true | **DRIFT** | loses excluded missions, firing latch, falling, concrete locomotor result, attack-target exception, and Doing table |
| generic boolean `is_driving` | **DRIFT** | no proof it is equivalent across six concrete bodies |
| infer `WeaponsFactory` from `Factory=UnitType && !Naval && ExitCoord` | **DRIFT** | native reads the independent byte and includes naval yards |
| infer `+0x68D` from “animation is Attack” | **UNCHECKED/DRIFT until proven** | native latch has exact arming and pre-fire clear order distinct from the visible Doing value |
| infer Object `+0x8D` only from `parachute_state` | **DRIFT** | native byte also covers generic Unlimbo settle and bridge `DropIn` |

## 10. Implementation Handoff

Keep the dispatcher out of scope. Close this as a pure, exact leaf-authority layer over explicit snapshots/state.

| Required effect | Binary evidence | Rust surface | Acceptance test | Do not do |
|---|---|---|---|---|
| Replace generic `is_driving` with a variant-dispatched exact `is_moving_now` contract for the six classes | six slot `+0x80` bodies and vtable/COL identities | `src/sim/movement/locomotor.rs`, Drive runtime/state modules | `ready_locomotor_drive_matches_turn_move_headto_speed_conjunction`; corresponding Ship/Hover/Walk/Teleport/Jumpjet tests | do not use only `movement_target.is_some()` or `phase != Idle` |
| Implement exact Unit leaf branch order | `0x744270` decompile+assembly | `src/sim/mission/verb.rs`; a richer Unit snapshot or entity method | `unit_ready_wf_contact_allows_only_move_or_enter` | do not flatten exceptions into one generic busy flag |
| Add Unit deploy/door tracker state and test both bytes | `0x4A51D0`; caller passes `unit+0x350` | Unit runtime component | `unit_ready_blocks_when_either_deploy_door_tracker_byte_is_active` | do not call it spy-plane/global/paradrop state |
| Parse and retain `WeaponsFactory=` independently | BuildingType `+0x16BD`; stock INI sections | `src/rules/object_type.rs` | `weapons_factory_flag_includes_land_factories_and_naval_yards` | do not reuse land-only production inference |
| Read Radio slot 0, then exact no-contact cell fallback | `0x65AD40`; `0x744270` signed cell compares | `Contacts::slot(0)`, occupancy/building lookup | `unit_ready_no_contact_blocks_only_exact_factory_anchor_south_cell` | do not use any-contact membership or whole-footprint approximation |
| Add/route Foot firing-sequence latch with native write order | `0x5206B0`, `0x51DF70`, exhaustive `+0x68D` scan | Infantry combat/animation state | `infantry_ready_blocks_from_fire_sequence_arm_until_prefire_clear` | do not derive it solely from Doing or cooldown |
| Add Object-wide falling/settle gate | `0x5F3900`, `0x5F4160`, `0x5F5940`, `0x5F3E70` | lifecycle/falling state shared by Unlimbo, DropIn, paradrop | `infantry_ready_blocks_during_unlimbo_and_dropin_height_settle` | do not equate it with InLimbo, OnBridge, or parachute-only state |
| Encode Doing permission table semantics | `0x521B60`; bytes at `0x7EAF7C`; GGI state machine | `Animation.sequence` plus an explicit native Doing mapping/table | `infantry_ready_deploy_blocks_but_deployed_fire_and_idle_allow` | do not index by Infantry type or CurrentMission; do not bounds-normalize values native leaves unchecked |
| Preserve predicate order and early exits | full Unit/Infantry assembly | leaf methods/tests | `ready_predicate_precedence_firing_and_falling_before_locomotor_and_doing` | do not reorder if a future input has same-tick side effects or debug assertions |

Suggested API shape (illustrative, not a mandated C++ port):

```text
ReadyAuthority::unit(entity, world_read_view) -> bool
ReadyAuthority::infantry(entity, world_read_view) -> bool
LocomotorState::is_moving_now_exact(owner_state) -> bool
```

The world view is needed only for Unit contact/building lookup. The locomotor helper should dispatch by `LocomotorKind` to variant-specific state. Keep it read-only and deterministic.

## 11. Stale-Document Replacement Wording

Use these exact replacements when the older documents are next audited; this report does not edit them.

1. `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`, Unit special branch:

   > `UnitClass::ReadyToCommence` calls `FUN_004A51D0` with `this = Unit+0x350`; the helper tests the Unit's embedded deploy/door animation tracker idle (`Unit+0x368==0 && Unit+0x369==0`). If idle, Ready examines Radio contact slot 0. A contacted `BuildingClass` with `BuildingType+0x16BD (WeaponsFactory=yes)` blocks unless queued mission is Move(2) or Enter(7). With no contact, the exact fallback blocks at the WeaponsFactory anchor cell plus Y=1. This is not a global, spy-plane, or ParaDrop-mode query.

2. The same document and `READYTOCOMMENCE_S5_BLOCKER_CLOSURE_AND_FEAR_SEQUENCE_GATE_GHIDRA_REPORT.md`, Infantry table:

   > `Infantry+0x6C4` is current animation Doing. `DAT_007EAF7C` is a four-byte-per-Doing property table; Ready reads byte 0. Doing 27=Deploy has byte 0 and blocks; 28=Deployed, 29=DeployedFire, and 30=DeployedIdle have byte 1 and allow. It is not an Infantry type-index table.

3. `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `0x75AB40`:

   > At Walk ILocomotion vtable offset `+0x80`, `0x75AB40` is the `Is_Moving_Now` predicate consumed by ReadyToCommence, despite the stale `Is_To_Have_Shadow` label. It returns true only when slot `+0x10`'s moving byte is nonzero, owner applied speed fraction `+0x578` is strictly positive, and the Walk destination coordinate is not the native null sentinel.

4. `GGI_GHIDRA_REPORT.md`, Infantry `+0x68D`:

   > `Foot+0x68D` is an armed firing-sequence latch. Infantry sets it after selecting a legal firing sequence, waits for the fire-frame anchor, and clears it before the firing virtual/bullet spawn; target loss/change, animation exhaustion, failed firing, and prone-fire reselection also clear it. AI may restore Deployed or Ready when an armed sequence aborts, but “RefreshDeployedSeq” is too narrow and reverses cause/effect.

5. `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` and the S5 deferred note for Object `+0x8D`:

   > Object `+0x8D` is `IsFallingDown` / fall-height-settle active. Object constructor clears it; Unlimbo and DropIn set it; ObjectClass::AI integrates falling state and clears it on landing. Infantry ReadyToCommence blocks while it is nonzero.

6. Any document saying a single stable locomotor interface proves a generic Rust boolean is enough:

   > The virtual slot is stable, but exact semantics are concrete-class-specific. Drive, Ship, Hover, Walk, Teleport, and Jumpjet read different state and predicates. Rust must preserve each concrete mechanism before exposing their common boolean result.

## 12. Negative Facts / Do Not Do

- Do not rename `0x4A51D0` as a global spy-plane, paradrop, or mission-mode query.
- Do not treat Unit `+0x350` as the contacted building; it is an embedded Unit tracker.
- Do not infer `WeaponsFactory` from land-only production fields; the native predicate includes `GAYARD/NAYARD/YAYARD`.
- Do not compact “factory footprint” and exact anchor-south fallback into one test.
- Do not replace all six locomotor predicates with `movement_target`, `phase`, current speed alone, or a category-level boolean.
- Do not call Foot `+0x68D` fear, panic, generic action, or only deploy refresh.
- Do not call Object `+0x8D` InLimbo, OnBridge, or parachute-only.
- Do not use CurrentMission or Infantry type index for the Doing table.
- Do not treat Doing 27..30 as mission values; they are animation sequence indices.
- Do not add a bounds clamp to the native table formula without separately proving all downstream equivalence.
- Do not use `ObjectClass__Save @ 0x5F6250` as savegame-stream proof; it is a CRC/checksum-style surface.
- Do not implement the full mission dispatcher as part of this bounded leaf fix.

## 13. Coverage Ledger and Final Open-Question State

| Scoped item | Status | Evidence | Residual |
|---|---|---|---|
| Unit `0x744270` exact branch order | verified | decompile + full assembly | none |
| `0x4A51D0` receiver/meaning | verified | caller register flow + helper body + deploy/door tracker research | none |
| Radio slot 0 and `+0x16BD` | verified | `0x65AD40` body + Unit branch + type/INI research | none |
| no-contact exact cell math | verified | decompile + assembly signed conversion/compares | none |
| stock factory activation | verified | stock INI + production path | none |
| active Unit/Infantry locomotor union | verified | merged type-list census + constructor/vtable identity | none |
| six concrete slot `+0x80` bodies | verified | decompile or raw-byte dry-run disassembly + COL | none |
| Infantry `+0x68D` writers/meaning | verified | exhaustive instruction scan + firing closed loop | none |
| Object `+0x8D` writers/meaning | verified | exhaustive filtered scan + Object AI/Unlimbo/DropIn + prior reports | none |
| Doing 27..30 names and Ready bytes | verified | raw table + sequencer/fire state machine + stock art | none |
| persistence statement | verified to bounded claim | CRC bodies + verified raw-stream research | class-specific final post-load survival intentionally not claimed |
| Unit fixture | verified by predicate walk | contacted factory + geographic fallback | none |
| Infantry fixture | verified by predicate walk | latch/falling/Doing/movement cases | none |

Final scoped questions:

- `[RESOLVED]` What is `0x4A51D0`? Embedded Unit deploy/door tracker idle test.
- `[RESOLVED]` What does the Unit contact branch protect? `WeaponsFactory=yes` production-exit readiness, with Move/Enter exceptions and exact no-contact recovery cell.
- `[RESOLVED]` Which active concrete locomotors can reach Unit/Infantry Ready in stock YR? Drive, Ship, Hover, Walk, Teleport, Jumpjet.
- `[RESOLVED]` What is Infantry `+0x68D`? Armed firing-sequence latch.
- `[RESOLVED]` What is Object `+0x8D`? Falling/height-settle active.
- `[RESOLVED]` What are Doing 27..30 and their Ready results? Deploy=block; Deployed/DeployedFire/DeployedIdle=allow.

### Zero-add and adversarial pass

The final pass re-decompiled both leaf predicates and rechecked the discovered helper/table inputs. No new scoped read, call, or writer was added. Five adversarial questions were applied:

1. Could `0x4A51D0` still be global due decompiler receiver loss? **No:** Unit assembly forms `this+0x350`; helper reads only `+0x18/+0x19` from that receiver.
2. Could `+0x16BD` mean only land war factory? **No:** stock naval-yard sections carry the same `WeaponsFactory=yes` byte and the predicate has no Naval check.
3. Could one generic moving boolean be mechanism-equivalent? **Unproven and contradicted by distinct concrete reads; verdict DRIFT.**
4. Could `+0x68D` be merely deploy refresh? **No:** it is set in shot arming, cleared immediately before bullet spawn, and operates in non-deployed firing paths.
5. Could Doing 27..30 be mission codes or type indices? **No:** DoType indexes per-type SequenceData by the same field, deploy/fire transitions name the indices, and Ready's table uses the Doing value directly.

Within the stated stop conditions, the bounded Unit/Infantry residual investigation is **COMPLETE**.

## Remaining Uncertainty

None for the bounded predicate/writer/locomotor scope.

The bounded investigation did not re-audit final class-specific post-load survival for each Unit/Infantry loader. The report therefore preserves only the directly evidenced raw save-stream coverage and checksum facts; it does not claim that every scoped byte survives every class-specific post-load cleanup path.

## Sources

- Fresh read-only Ghidra: `decompile_function`, `disassemble_function`, `disassemble_bytes(dry_run=true)`, `search_instructions`, `read_memory`, and vtable/COL/type-descriptor walks at `0x744270`, `0x521B60`, `0x4A51D0`, `0x65AD40`, `0x4AFC20`, `0x69F330`, `0x514C80`, `0x75AB40`, `0x4B6610`, `0x54D0D0`, `0x5206B0`, `0x51B1F0`, `0x51BAB0`, `0x51DF70`, `0x520AE0`, `0x5216D0`, `0x5F3900`, `0x5F3E70`, `0x5F4160`, `0x5F5940`, `0x5F6250`, and table `0x7EAF7C`.
- Existing research reconciled: `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`, `READYTOCOMMENCE_S5_BLOCKER_CLOSURE_AND_FEAR_SEQUENCE_GATE_GHIDRA_REPORT.md`, `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md`, `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `GGI_GHIDRA_REPORT.md`, `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md`, plus the cited Radio, production-exit, Object/falling, and bridge DropIn reports.
- Stock activation/data: merged `ini/rules.ini` and `ini/rulesmd.ini` Unit, Infantry, locomotor, and `WeaponsFactory=` definitions, with YR `*md` overrides taking precedence.
- Current Rust reviewed: `src/sim/mission/verb.rs`, `src/sim/movement/locomotor.rs`, `src/rules/object_type.rs`, and the directly related Drive runtime, Radio contacts, animation/combat, lifecycle/falling, occupancy, and entity-state modules found through `rg`.
