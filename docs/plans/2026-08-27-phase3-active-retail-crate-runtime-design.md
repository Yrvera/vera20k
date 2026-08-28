# Phase 3 active-retail crate runtime design

**Status:** design candidate; implementation is forbidden until fresh read-only
critics return zero findings.

**Supersedes:** `docs/plans/2026-07-23-crate-authority-design.md` for Phase 3
ordinary-retail closure. The July design is not implementation authority: it
contains stale initial-count, slot, water, Mark, trigger, effect-radius,
FreeMCV, and ingress assumptions.

**Native authority:**
`docs/research/PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md`.

## Goal

Replace the current scenario-start-only crate scatter with the exact active
Yuri's Revenge crate mechanism: persistent slots and timers, native placement
and ghost acceptance, deterministic pickup and immediate replacement, the
eight stock-reachable effects, exact movement-call continuations, active map
action and building ingress, presentation ordering, persistence, and hashing.

This is a promoted prerequisite for Phase 3 Ship/SQD closure. Native
`ParasiteClass::Attach` reaches Ship ForceTrack, which synchronously invokes
crate pickup while SQD is limboed and then continues against the same deferred-
lifetime object. An end-of-tick crate queue cannot preserve that behavior.

## Completion boundary

The mechanism closes only when all of the following are true:

- every stock-reachable branch and RNG draw matches the native evidence;
- every installed-retail producer is implemented or explicitly excluded by
  the census in the authority report;
- no visible-overlay shortcut substitutes for the 256 native slot records;
- all thirteen pickup callsites use one synchronous transaction and reproduce
  their caller-specific continuation;
- persistent slot, latch, trigger, multiplier, and fixed-content state survives
  snapshot round trips and affects the broad deterministic hash;
- the retail multiplayer quick checksum remains unchanged where native omits
  direct slot state;
- focused crate, movement, trigger, combat, snapshot, and scheduler tests pass;
- fresh implementation critics report zero unresolved, approximate, missing,
  or residual behavior.

## Evidence-led corrections

The builder must remove, not preserve, these current or stale assumptions:

- Initial count is signed
  `min(maximum, max(minimum, human_session_node_count))`. AI seats do not
  contribute.
- Random placement owns a slot and timer after hard-precheck acceptance even
  if allocation, construction, Unlimbo, passability, occupation, slope, or
  Mark fails. That result is an accepted ghost and stops retries.
- Stock flat, empty Water accepts and visibly Marks WCRATE because the water
  identity selects Float passability and retail `[Water] Float=100%`; only the
  exact post-precheck failure predicates below create accepted timed ghosts.
- Placement goes through crate Overlay construction/Mark semantics; direct
  `OverlayGrid::place_overlay` is not equivalent.
- `[Powerups]` field three is water eligibility.
- Radius effects use exact 3D distance and live Ground display order without
  an owner filter.
- HealBase walks owner-matching Logic entries; it is not a building-only query.
- Parsed `[CrateRules] FreeMCV` is not the live pickup override gate. The
  session `Bases` option is.
- Pickup's return value is locomotor control flow, not a consumed flag.
- Trigger repeat mode belongs to `[Tags]` field zero. `[Triggers]` field three
  is the enabled flag: nonzero admits construction, subject to the selected
  Scenario difficulty flag. Zero never means enabled.
- `CarriesCrate` exists but is disabled throughout installed retail data by
  `TruckCrate=no`/`TrainCrate=no`; `CrateBeneath` and action 108 are the active
  ordinary-map ingress paths.

## Architecture fit

### Ownership

Create a crate subsystem directory by replacing `src/sim/crates.rs` with:

```text
src/sim/crates/
  mod.rs          public Simulation-facing contract
  state.rs        CrateAuthority and CrateSlot
  placement.rs    random/specific placement, Mark/ghost, clear, regen
  pickup.rs       selection, remaps, synchronous transaction, returns
  effects.rs      eight active effects and common presentation tail
  tests.rs        focused native fixtures
```

`CrateAuthority` owns only crate slot/latch state. `Simulation` continues to
own RNG, overlays, entities, houses, terrain, trigger runtime, animation,
sound, shroud, and the frame scheduler. Static crate and Powerups data remains
in `RuleSet`; map-authored scenario flags and tags remain in map/session
authority.

Crate mutation also returns an ordered transient presentation batch to the
existing sim-to-app drain boundary. Placement appends
`DirtyScreenRect { union_rect, force: false }` followed by
`CellRedraw { cell, frame }`; removal appends only the dirty-screen request,
before its two Cell writes. The app-side tactical invalidation adapter owns
viewport/exploration/queue gates and stamps. These requests are not serialized,
hashed, or converted into radar dirties. Accepted placement ghosts carry the
native zero rectangle but still carry both ordered requests; removal ghosts
and identity mismatches carry none. Tests inject tactical adapter state so the
native gates and pre-mutation ordering can be asserted without moving camera
state into deterministic crate authority.

### Persistent state

Add to `Simulation`:

```rust
pub(crate) struct CrateAuthority {
    pub(crate) slots: [CrateSlot; 256],
    pub(crate) pickup_any_latch: bool,
}

pub(crate) struct CrateSlot {
    pub(crate) start_frame: i32,
    pub(crate) timer_aux: u32,
    pub(crate) duration_frames: i32,
    pub(crate) cell_x: i16,
    pub(crate) cell_y: i16,
}
```

Fresh/reset slot state is exactly `{-1,0,0,0,0}`. `(0,0)` alone is empty;
overlay visibility is not occupancy. Do not replace the table with a map keyed
by cell: duplicate ghost coordinates and ascending physical slot order affect
clear, regeneration, and RNG.

`pickup_any_latch` is active even though stock maps have no Event-50 condition:
every surviving pickup from a `CrateTrigger=yes` overlay sets it, the next
leading trigger rung attempts Event 50 in global tag order, then clears it.

Replace `ScenarioSession.game_mode_nonzero` as the stored authority with a
serialized/hashed exact `NativeGameMode` discriminator. Phase 3 admits only
`Campaign=0` and `OfflineSkirmish=5`; existing boolean consumers derive
`mode != Campaign`. Raw LAN 3, WOL 4, and every other raw value are rejected at
the app/loading launch boundary before `MatchLaunchDescriptor`,
`ScenarioDescriptor`, or `ScenarioSession` construction. The front-end
`SkirmishLaunchMode.id` is an MPModes roster identity, not native `g_GameMode`:
offline Cooperative ID 3 and Unholy ID 4 remain valid and still construct raw
mode 5. No direct test/helper/snapshot/replay path may bypass the checked mode
constructor.

Add a separate serialized/hashed signed `ScenarioSession.trigger_difficulty_raw`
authority. Snapshot it once at Scenario bootstrap: Campaign copies the
campaign launch difficulty and OfflineSkirmish copies the raw
`[MultiplayerDialogSettings] AIDifficulty` launch value. Stock offline retail
therefore stores zero. The native Trigger mapping is literal
`0=Easy, 1=Medium, 2=Hard`; do not invert it to match the unrelated House-AI
terminology in which zero is called Hard. `GameOptions.ai_difficulty` is only
the launch seed and per-opponent House difficulty is a separate owner; after
bootstrap every Trigger constructor and Action 53 reads the Scenario field.
Raw values outside `0..=2` are preserved rather than normalized.

## Rule and map data

### CrateRules and Powerups

Expand `src/rules/ruleset.rs::CrateRules` with native-width values:

- signed i32 minimum, maximum, and solo money;
- native-double regen and parsed radius converted to signed leptons;
- three crate overlay identities;
- Silver/Wood/Water fixed crate-type mappings;
- optional HealCrateSound and UnitCrateType;
- parsed `FreeMCV`, retained for data parity but not used as the live override;
- a fixed `[PowerupEntry; 19]` in the native canonical order.

Do not use installed INI values as constructor defaults. `RulesClass__
Constructor @0x00665650` initializes this exact state:

| Field | Offset | Constructor value |
|---|---:|---:|
| `FreeMCV` | `+0x040` | false |
| `WoodCrateImg`, `CrateImg`, `WaterCrateImg` | `+0x0F8/+0x0FC/+0x100` | null/null/null |
| `HealCrateSound` | `+0x718` | `-1` |
| `SoloCrateMoney` | `+0x1140` | `2000` |
| `UnitCrateType` | `+0x1148` | null |
| `SilverCrate`, `WoodCrate`, `WaterCrate` | `+0x1464/+0x1468/+0x146C` | `2/0/0` (`HealBase/Money/Money`) |
| `CrateMinimum`, `CrateMaximum` | `+0x1470/+0x1474` | signed `1/255` |
| `CrateRegen` | `+0x1678` | binary64 10.0, bits `0x4024000000000000` |
| `CrateRadius` | `+0x172C` | signed `640` leptons (`2.5` cells) |

`RulesClass__ReadCrateRules @0x0066B900` returns without a store when the
section is absent. When present it reads, in order: FreeMCV; Wood, common, and
Water image; Heal sound; minimum; maximum; radius; regen; UnitCrateType; solo
money; Silver, Wood, and Water fixed mappings. Every call supplies the current
field as its default, so a later missing key retains the exact prior value.
Active `INIClass::Put_String @0x00528660` removes an entry whose value is empty
after trimming, so `Key=` is indistinguishable from absence for every reader
below and retains current state; it is not a present empty-token case. Apply
layers in native order: constructor, base RULESMD, optional LANGRULE, selected
mode payload, then map at `FullInit 0x0068774F`. MISSIONMD is read before the
rules reset and cannot win. The optional later TMCJ4F pass is absent from an
ordinary stock run. Installed LANGRULE, mode payloads, and all 184 maps contain
no nonempty CrateRules override, so retail's base-RULESMD result is final. Use
the existing `RulesLayerStack`/type-allocation pass order; do not flatten a
missing later key into the constructor value.

`CrateRadius` uses `CCINIClass__ReadRange @0x00474620`, not an ordinary int or
cell-fixed helper. It calls native ReadDouble with sentinel `-1.0`; an exact
`-1.0` or unordered/NaN comparison returns the supplied current signed i32.
Otherwise it multiplies the widened binary64 value by exact 256.0, runs the
53-bit/chop FISTP-to-i64 kernel, and consumes the low signed dword. There is no
clamp or saturation. Thus `2.5 -> 640`, retail `3.0 -> 768`, `-0.5 -> -128`,
`-1 -> prior`, `-1% -> -2`, and `-100% -> prior`; nonfinite/out-of-i64 becomes
x87 integer-indefinite with low dword zero, while finite i64 values outside
i32 wrap through the low dword. The ReadDouble text path is `%f` to binary32,
then widened; any `%` multiplies by binary64 0.01.

`CrateRegen` uses that same `%f`/widen/optional-percent ReadDouble path and
stores the returned qword with no additional conversion. Missing retains
exact 10.0 or the prior layer; retail `3` stores exact binary64 3.0 bits
`0x4008000000000000`. A present malformed nonempty ReadDouble token has the
same native address-layout-dependent scanf alias accident documented for
country speed; exclude it as invalid input rather than silently substituting
zero.

Minimum, maximum, and solo money are signed ReadInt values with no clamp.
Decimal mode is CRT `atoi`: leading whitespace/sign and a decimal prefix are
accepted, trailing junk is ignored, no digits yields zero, and overflow wraps
to signed i32. A `$` marker or `h`/`H` hex form selects `%x`; failed hex
conversion retains the supplied current default. FreeMCV uses the
case-insensitive first-character table (`0/F/N`, `1/T/Y`) and retains current
on absence or any other first character. Image strings use a 128-byte native
buffer: absent (including authored empty) retains current, `none`/`<none>` stores
null, known names resolve existing OverlayTypes, and unknown names allocate in
the ordinary Overlay registry (OOM is invalid-domain null). Heal sound
absent/unknown retains its prior index. UnitCrateType absent
retains, `none` stores null, and an unknown nonempty name follows ordinary
UnitType find-or-allocation. Silver/Wood/Water absent retain; a known canonical
name selects the fixed index from the complete table
`Money, Unit, HealBase, Cloak, Explosion, Napalm, Squad, Darkness, Reveal,
Armor, Speed, Firepower, ICBM, Invulnerability, Veteran, IonStorm, Gas,
Tiberium, Pod` = `0..18`, and any unknown nonempty name stores Money index zero.

Installed retail then overrides constructor state to signed `1/255`, radius
768, regen qword 3.0, solo money 5000, `HealBase/Money/Money`, images
`CRATE/CRATE/WCRATE`, `HealCrate`, null UnitCrateType, and FreeMCV true.

`PowerupEntry` contains signed weight, resolved live animation identity/index,
water-eligibility byte, and native binary64 data. The baseline is executable-
image global state, not a `RulesClass` constructor field: weights are
`[50,20,1,3,5,5,20,1,1,10,10,10,1,3,1,1,1,1,1]`, all animation indices are
`-1`, and all data/water bits are zero.

`RulesClass__ReadPowerups @0x00673E80` is a typed per-pass state transition.
An absent `[Powerups]` section preserves all 19 slots. A present section visits
all canonical names in fixed order and calls `ReadString` with the exact
128-byte-buffer default `"0,NONE,0"`; therefore an omitted canonical key does
not preserve its whole slot. It stores weight zero, resolves bare `NONE`
against the live animation registry (stock has no such animation and obtains
`-1`, but a mod may have allocated one), preserves water because token `0` is
neither yes nor no, and preserves data because the fourth token is absent.
An authored empty row is removed by INI loading. If every entry is empty or a
comment, the whole section is absent and preserves all state; if another
nonempty canonical or unknown row keeps the section live, that empty row is a
missing key and takes the mixed fallback above.

Tokenization is CRT `strtok` with comma as the delimiter set: leading,
consecutive, and trailing commas collapse rather than representing empty
fields, so later values shift left. Each surviving token is trimmed at its
field parser. Token one uses direct decimal CRT `atoi` with prefix parsing and
signed i32 wrapping; it has no `$`/hex/`h` mode. Token two performs a live
case-insensitive animation lookup: `<none>` and unknown names store `-1`, the
first current match wins, and allocation in a later pass does not repair an
earlier `-1`. Token three changes water only for case-insensitive exact `yes`
or `no`; any other present token and an absent token retain the prior byte.
Token four uses direct CRT `atof` to binary64, not the `%f`-to-f32
`ReadDouble` path; invalid/nan/inf text becomes `+0.0`, signed zero and
overflow/underflow follow CRT behavior, and a `%` anywhere in the token
multiplies through x87 by exact binary64 `0.01` bits `0x3F847AE147AE147B`.
Tokens after four are ignored.

Advance this table inside `RulesPassProcessor::apply_pass` from each original
`IniFile`, after that pass has allocated the animation names visible at native
ReadPowerups time. Carry the completed typed table in `ProcessedRulesLayers`
and have `RuleSet::from_processed_rules` consume it; never reconstruct it from
the merged compatibility `IniFile`. `RuleSet::from_ini` must enter the same
single-pass typed path. The native sequence is static baseline -> optional
MISSIONMD Process -> registry/rules reset (which does not clear the Powerup
globals) -> RULESMD -> optional LANGRULE -> selected mode -> scenario/map ->
optional TMCJ4F. Current `RulesLayerStack` covers the active typed RULESMD/
LANGRULE/mode/map suffix. Installed MISSIONMD and TMCJ4F contain no Powerups,
LANGRULE is absent, and installed mode payloads contain none, so omitting those
no-op sources does not alter active results; add them as explicit pass kinds
when their owning campaign loaders enter supported runtime rather than faking
them through a flattening projection.

Installed retail's only positive-weight slots resolve exactly as follows:
Money `(20,MONEY,true,2000.0/0x409F400000000000)`, Unit
`(20,null,false,0)`, HealBase `(10,HEALALL,true,0)`, Reveal
`(10,REVEAL,true,0)`, Armor `(10,ARMOR,true,1.5/0x3FF8000000000000)`, Speed
`(10,SPEED,true,1.2/0x3FF3333333333333)`, Firepower
`(10,FIREPOWR,true,2.0/0x4000000000000000)`, and Veteran
`(20,VETERAN,true,1.0/0x3FF0000000000000)`. The remaining eleven weights are
zero, but their canonical slots and parsed values remain represented so index
identity and explicit-action boundaries do not drift.

The complete RULESMD winning arrays are weights
`[20,20,10,0,0,0,0,0,10,10,10,10,0,0,20,0,0,0,0]`, water
`[1,0,1,1,1,0,0,1,1,1,1,1,1,1,1,1,1,0,0]`, animation identities
`[MONEY,<none>,HEALALL,CLOAK,<none>,<none>,<none>,SHROUDX,REVEAL,ARMOR,
SPEED,FIREPOWR,CHEMISLE,ARMOR,VETERAN,<none>,<none>,<none>,<none>]`, and data
bits `[409F400000000000,0,0,0,407F400000000000,4082C00000000000,0,0,0,
3FF8000000000000,3FF3333333333333,4000000000000000,0,3FF0000000000000,
3FF0000000000000,0,4059000000000000,0,0]`. Legacy retail maps `SOV07S` and
`SOV08U` actively override all 19 rows; their final weights are respectively
`[100,0,0,0,0,0,0,0,0,20,20,20,0,0,10,0,0,0,0]` and
`[100,0,0,0,0,0,0,0,5,20,20,20,0,0,30,0,0,0,0]`, with base animation,
water, and data values retained by their authored complete rows.

Parse the crate spatial sounds from `[AudioVisual]`:
`CratePromoteSound`, `CrateMoneySound`, `CrateRevealSound`, `CrateFireSound`,
`CrateArmourSound`, `CrateSpeedSound`, and `CrateUnitSound`. `HealCrateSound`
remains `[CrateRules]` authority. Their seven RulesClass slots at
`+0x1E4..+0x1FC` construct to `-1` and installed retail resolves, in effect
order, the Voc identities `CrateMoney`, `CrateReveal`, `CrateFirePower`,
`CrateArmor`, `CrateSpeed`, `CrateFreeUnit`, and `CratePromoted`; preserve
identity rather than executable-local numeric index. Preserve the existing
`RuleSet.bridge_warheads.c4_name` owner for `[CombatDamage] C4Warhead=` and its
`ResolvedRuleHandles.c4` projection: native `RulesClass+0xFA8` constructs null
and installed retail resolves the existing `Super` WarheadType. This is
mandatory active state for positive-weight HealBase. Only the separate random
Explosion handler is evidence-backed unreachable in ordinary stock because
its weight is zero; that exclusion does not remove or weaken the shared Rules
warhead owner.

### Type flags

Extend existing rule owners, without a crate-only shadow registry:

- OverlayType: `CrateTrigger=` alongside existing `Crate=`.
- UnitType: `CrateGoodie=` and `CarriesCrate=`.
- BuildingType: `CrateBeneath=` and `CrateBeneathIsMoney=`.

`CrateGoodie` participates in Unit-effect selection. `CrateBeneath` participates
in the active building unplace/death path. `CarriesCrate` is parsed and its
exact death gate is implemented so installed retail proves a no-op from the
false scenario flags rather than from missing code.

### Scenario and trigger data

Add optional `TruckCrate` and `TrainCrate` bools to `BasicSection`; resolve
missing values to native false in scenario/session state and serialize/hash
that resolved state.

Add `attached_tag_id: Option<String>` to `MapEntity` using category-specific
columns: Unit/Aircraft 7, Infantry 8, Structure 6. Preserve the tag on spawned
`GameEntity` as an interned stable reference.

Expose `[Tags]` in source order as `{repeat_mode, name, trigger_type_head_id}`:
field zero is the repeat mode, field one is the display name, and field two is
the head TriggerType ID. Remove the fabricated Trigger-field repetition. Parse
`[Triggers]` field three as the authored enabled flag (`value != 0`), preserve
difficulty fields four/five/six as the Easy/Medium/Hard admission flags, and
do not reinterpret field seven as repetition. Constructor enable state is
`field3 && fields[trigger_difficulty_raw]` for raw difficulty `0..=2`; an
out-of-range raw difficulty uses field three alone.

Preserve the linked TriggerType chain and the textual event/action chunks so
runtime construction can reproduce native ownership. Tags are globally polled
in `[Tags]` source order. A Tag's TriggerInstances are reverse
TriggerType-chain order. Each TriggerType's Event list is reverse textual
CSV-chunk order because Events push-front; its Action list remains textual
CSV-chunk order because Actions append through the prior node.

## Placement contract

### Bootstrap

`place_scenario_start_crates` becomes a thin `Simulation` adapter that calls
`CrateAuthority::place_random` once per signed requested iteration when the
session Crates option is on. It does not retry failed calls and does not add AI
houses to the human-node count. Bootstrap has no game-mode gate.

### Random placement

`place_random` performs exactly:

1. first empty slot, ascending; full table returns false without RNG;
2. retain that slot across at most 1000 attempts;
3. per attempt draw X then Y from the active Map rectangle;
4. choose Float(5) FNPC from a water origin, Track(1) otherwise;
5. call the existing nearby-cell service with the verified native tuple and
   radius `min(SizeW+SizeH,32)`;
6. hard reject only invalid playfield-mode-1 or an existing overlay identity;
7. on hard rejection, retry without timer RNG;
8. on precheck acceptance, re-fetch the snapped destination Cell. Exact
   `LandType +0xEC == 2` selects `WaterCrateImg`; every other dword value
   selects `WoodCrateImg`. `CrateImg` is never used. Origin land type controls
   only Float/Track snapping and does not select the image;
9. allocate/construct the selected Overlay and attempt Unlimbo/Mark, retaining
   accepted-ghost status on every failure after the two hard prechecks;
10. union the two native object rectangles and issue tactical DirtyScreenRect
    with force zero, including the zero-rectangle request for an accepted
    ghost, then restore the saved editor flag;
11. return accepted from the validator, issue the native cell-redraw helper on
    the snapped cell, and only then store the slot coordinate;
12. consume the timer draw and store the remaining slot words;
13. return true after accepted visible or accepted ghost.

Use an explicit outcome enum:

```rust
enum CratePlacementOutcome {
    HardRejected,
    AcceptedVisible,
    AcceptedGhost,
}
```

The overlay/Mark adapter must reproduce the constructor and Mark gates, while
keeping allocation/constructor/Unlimbo/Mark failure injectable in tests. After
the two hard prechecks, an active ground `TerrainClass` found by
`FUN_0047C550(cell,0)` alone makes the constructor skip
`ObjectClass__Unlimbo`, producing a ghost; ordinary Cell occupation is not
that constructor gate. If Unlimbo reaches `OverlayClass__Mark @0x005FC570`, a
stock crate ID rejects only when slope byte `Cell+0x11C > 4`; slopes 0..4
remain eligible.

Mark checks selected overlay identity in this precedence order: equality with
current `WaterCrateImg` selects SpeedType Float/5; otherwise equality with
current `CrateImg` or `WoodCrateImg` selects Track/1. Thus Water wins if the
configured pointers alias. `CellClass__CheckCellPassability @0x004834A0`
receives required zone `-1`, required level `-1`, movement zone 0,
`ignore_infantry=0`, `ignore_vehicles=0`, and bridges allowed. When
`Cell.Flags & 0x100` is set, it selects the unmasked low byte of
`AltOccupationFlags Cell+0x128`; otherwise it selects unmasked
`OccupationFlags Cell+0x124`. Exactly zero passes and any nonzero bit rejects.
For a non-bridge selection, the chosen Float/Track terrain-row value must be
nonzero; selecting the bridge/deck field bypasses the later zero underlying-
terrain-speed rejection. A failed gate leaves an accepted ghost. Rust need not
materialize native orphan Overlay object IDs after Mark failure; it must
preserve ghost slot, cell byte, timer, and RNG.

Random visible placement leaves OverlayData `0xff`; a ghost does not write it.
In installed retail, flat empty Water plus WCRATE is visible because Water
Float is 100%. Active ground TerrainClass, slope above 4, nonzero selected
occupation, zero non-bridge terrain speed, and injected Mark/allocation
failures are accepted ghosts; an existing overlay is a hard rejection.

The presentation tail is explicit and representation-neutral. Every accepted
visible or ghost placement emits a tactical dirty-screen request before its
cell-redraw request; a hard rejection emits neither, and placement never emits
radar dirty. The cell-redraw service reproduces
`FUN_006DA7D0 @0x006DA7D0`: enqueue only when the suppression global is zero,
the Cell's last-redraw frame `+0x5C` differs from the current frame, the Cell is
explored (`+0x12C & 8`) or has forced byte `+0x138`, its projected rectangle
intersects the widened tactical viewport, and the queue contains fewer than
799 entries. Success stamps `+0x5C` with the frame, clears `+0x138`, and sets
the tactical redraw flag. These presentation events are not serialized sim
authority, but their native order and gates are observable and testable.

### Specific-cell placement

`place_specific(origin, full_data_dword)` snaps once before scanning slots.
Invalid snap or full table returns false. On acceptance:

- exactly `0x14` performs no data post-write;
- every other dword writes its low byte, including `0x114 -> 0x14`;
- visible and ghost outcomes both return true;
- a non-`0x14` low-byte write occurs only after the complete dirty-screen,
  cell-redraw, slot-coordinate, and timer tail and emits no second invalidation;
- no Crates or game-mode gate is hidden inside this helper;
- no coordinate deduplication is added.

### Timer, clear, and regeneration

Timer construction uses the current pre-increment `binary_frame` and
`NativeF64Bits`/`native_x87` helpers:

```text
lower = regen * 450.0
upper = regen * 1800.0
r = RandomRanged(0, 0x7ffffffe)
duration = x87_ftol(lower + r / 2147483646.0 * (upper - lower))
aux = high dword of stored upper double
```

Clear first invokes a crate-specific removal adapter, then clears the slot
coordinate even when removal found a ghost or mismatch, preserves remaining
duration with signed wrapping elapsed arithmetic, and writes `start=-1`. A
prior `start=-1` preserves duration. Do not call generic
`OverlayGrid::clear_overlay`, which resets unrelated Rust Cell state and emits
the wrong post-mutation dirty event.

For nonzero mode the adapter bounds-checks the Cell and accepts only overlay
identity exactly equal to the current Rules CrateImg, WoodCrateImg, or
WaterCrateImg. A match obtains and unions the two native Cell rectangles,
appends `DirtyScreenRect(force=0)` before mutation, then writes only overlay
identity to none (`Cell+0x44=-1`) and OverlayData to zero (`Cell+0x11E=0`).
Every other Cell field, including Rust `wall_owner`, remains unchanged. It
emits no CellRedraw and no radar dirty. A ghost or identity mismatch emits no
request and changes no Cell, while its caller still clears/preserves slot
timer state.

Mode zero has no slot lookup and inlines the same dirty-before-identity/data
tail, but accepts any live OverlayType whose native `Crate +0x2AA` byte is
true, not only the three current Rules identities. `MapClass__RemoveCrateAtCell
@0x0056C020` finds the first matching occupied slot in nonzero mode; the slot's
`ClearAndPreserveTimer @0x004A1750` calls removal before coordinate/timer
mutation.

Regeneration runs only for nonzero game mode plus session Crates, scans all
slots ascending, and uses native expiration predicates. Each expired slot is
cleared and immediately calls random placement once. Do not snapshot the slot
list: a replacement above the cursor must be observed later in the same pass.

Insert the rung in `src/sim/world/mod.rs` after live-object/effect/alpha-
equivalent work and immediately before the first Phase-7 factory sweep, with
House work later. Use the pre-increment master frame.

## Synchronous pickup transaction

Expose one Simulation-level API:

```rust
fn pickup_crate_at(
    &mut self,
    cell: (u16, u16),
    collector_id: u64,
) -> NativePickupReturn;
```

`NativePickupReturn` is an explicit Zero/One enum. Never call it `consumed`.
The transaction releases all entity borrows before invoking triggers,
damage, spawning, Uninit, animation, or locomotor callbacks. It carries a
stable ID and re-fetches after every synchronous mutation.

Exact order:

1. return One for null/missing collector, absent/non-crate overlay, or
   nonzero-mode passive owner;
2. when `CrateTrigger=yes` and collector has an AttachedTag, synchronously
   raise Event 49, ignore callback return, then re-fetch the collector;
3. if callback killed/uninitialized collector, return Zero with crate, latch,
   RNG, and effects unchanged;
4. set `pickup_any_latch=true` for every surviving CrateTrigger pickup;
5. select fixed or weighted type;
6. resolve side BaseUnit, session-Bases free-MCV override, anti-stack and water
   remaps in native order;
7. perform no WOL-stat mutation in supported Phase 3 modes 0/5; raw mode 4 is
   unreachable because launch construction rejected it before Simulation;
8. remove the first matching slot/overlay through the crate-specific adapter;
   its dirty request precedes the two Cell writes, which precede slot/timer
   mutation;
9. in nonzero mode with Crates enabled, attempt one immediate random
   replacement;
10. remap Squad to Money;
11. execute the final effect;
12. execute its common presentation tail and return its native control value.

RNG order is fixed selection, replacement attempts X/Y, successful replacement
timer, then effect draws. Predetermined OverlayData below 19 skips selection.

### Evidence-closed mode-4 boundary

Native exact mode 4 is Westwood Online, not ordinary Skirmish 5 or LAN 3. It
increments a postgame-only counter owned by the collector's House after all
free-MCV/anti-stack/terrain/water remaps and before removal, replacement, the
late Squad-to-Money remap, and the effect. Pre-count remaps increment Money
index 0; Squad increments 6 and then executes Money. Event-49 death counts
nothing; owner transfer credits the owner re-read at count time. Increment is
wrapping i32 and never rolls back.

Native storage is an embedded 512-dword CounterClass at `House+0x4B70`, reset
to logical length 19. It survives House save/load, is absent from House CRC,
and has no gameplay or local-UI reader. Its sole reader is the match-end WOL
serializer, which network-byte-orders the nineteen counters, trims after the
last nonzero index, and emits `4*used_count` bytes as `CRA<player digit>`; all
zero emits length zero. Unused physical slots, index-19 ignore, and negative
underwrite are not reachable from crate dispatch.

Accordingly this Phase 3 ordinary-skirmish implementation adds no counterfeit
House counter or partial WOL packet support. Tests prove exact modes 0/5 never
mutate such state and raw mode 4 fails the checked launch constructor before
Simulation. If Phase 13 later enables WOL, its owner must add a serialized
`[i32;19]`, omit it from gameplay/world hashes and retail checksum, preserve
the exact count/remap order and wrapping overflow, and own `CRA#` emission.

## Selection and guards

- Weighted selection sums signed weights, draws inclusive `[1,total]`, and
  selects the first cumulative sum at least the roll.
- Mode zero bypasses MP guards. Signed data zero applies image overrides in
  CrateImg, WoodCrateImg, WaterCrateImg order; retail CRATE identity collision
  therefore ends at Wood/Money. Solo Money is fixed and draws nothing.
- Free-MCV forces Unit only at zero owned buildings, funds strictly above 1500,
  zero owned side BaseUnit, and session Bases enabled.
- Unit remaps above 50 units; Squad above 100 infantry.
- Cloak/Armor/Speed/Firepower remap when the picker already has the modifier.
  Speed also remaps Aircraft. Firepower also requires the picker capability
  vslot.
- Veteran remaps non-trainable or already-elite picker.
- Unit/Squad remap on Water or Beach, then the independent selected-entry
  water byte remaps any disallowed Water result.

All comparisons are strict and use native category/count authority. Tests must
exercise 50/51, 100/101, and 1500/1501.

## Eight active effects

Effects use the final selected/remapped entry for sound and animation.

### Money

Mode nonzero consumes inclusive `[ftol(data), ftol(data)+900]`; mode zero uses
fixed SoloCrateMoney without a draw. Add credits with wrapping i32 semantics.
In mode zero only, a local-human picker credits `g_PlayerPtr`; every other path
credits the picker House. Credits mutate first. Picker-owner-local-human then
gates CrateMoneySound; MONEY animation follows.

### Unit

Use `UnitCrateType` when present. Otherwise draw candidates indefinitely from
the full UnitType registry until `CrateGoodie` and the exact session-Bases /
BaseUnit rules accept one. Every rejected type consumes a draw.

A non-null `UnitCrateType` is forced directly and consumes no candidate draw.

Construct for the picker owner; try Unlimbo at crate center, then one movement-
typed FNPC using the chosen UnitType movement field corresponding to native
`UnitType+0x67C`, with facing zero, and nearby Unlimbo. Success plays
CrateUnitSound for a local-human owner,
returns Zero, and skips common animation. Type/allocation failure reaches the
Unit common tail and returns One. Double placement failure destroys the child,
then runs Money with Unit's already-loaded data rather than reloading the
Money entry. Stock Unit data is zero, so this late path draws 0..900, performs
the normal credit/sound sequence, creates MONEY animation, and returns One. A
pre-handler Unit-to-Money guard remap does reload Money data and therefore
draws the normal 2000..2900.

### HealBase

Picker-owner-local-human sound occurs before mutation. Walk the live Logic
vector in native order and re-read its live count after every candidate; do
not snapshot the vector. For each non-null owner match, call the real damage
receiver with `health-strength`, zero distance,
the existing `ResolvedRuleHandles.c4` selected by
`RuleSet.bridge_warheads.c4_name` from `[CombatDamage] C4Warhead=` (installed
`Super`), and `(0,1,1)` flags; invoke even at zero. Do not create a crate-owned
duplicate. The WarheadType identity and its live receiver semantics are not a
dummy healing tag. HEALALL animation is last.

### Reveal

Call `MapClass__BlackoutShroud`'s equivalent first. A non-null picker House gets
`MapIsClear=true` before every later gate. A remote House then returns from map
work immediately: it does not evaluate mode policy, write Visionary/cells,
run Paranoid passes, refresh radar, or request redraw. For the local House in
supported raw mode 5, no cells are spared. Evaluate its persisted/hashed
`Visionary` latch; true skips all map work. Otherwise execute exactly:

1. `ParanoidRevealAll(0,0)`, snapshotting live Techno count and walking forward;
2. set local `Visionary=true`;
3. visit every allocated map-diamond cell once in native anti-diagonal order;
4. write the equivalents of `Cell+0x130=0`, `+0x134=0`,
   `+0x12C |= 0x18`, and `+0x140 |= 0x03`, preserving unrelated bits;
5. `ParanoidUnrevealAll(0,0)` with its own forward count snapshot;
6. refresh radar;
7. flag screen redraw mode 2, including tactical redraw and map draw-cache
   generation increment.

For raw `[Map] Size` width `N` and height `M`, the iterator covers exactly
`M*(2*N-1)` cells satisfying `N < x+y`, `x-y < N`, `y-x < N`, and
`x+y <= N+2*M`. This uses the native allocated-diamond frame, not the current
compact owner flag alone. The stock mode-5 `N=M=80` fixture writes all 12,720
cells, including `(7,85)`, `(13,91)`, and `(93,145)`.

For exclusion evidence, native raw modes 3/4 spare three direct-write cells
only when a selected MPModes object exists and its virtual predicate is false:
`(7,N+5)`, `(13,N+11)`, `(M+13,N+M-15)`. There is no clamp, lookup, sentinel,
or deduplication. Cooperative or null selected-mode pointers spare none. Phase
3 does not implement this unreachable branch because raw 3/4 construction is
rejected before Simulation; offline Cooperative/Unholy roster IDs still use
raw mode 5 and write every cell.

After `BlackoutShroud` returns, attempt `CrateRevealSound` even for a remote or
already-Visionary picker. REVEAL animation is last at crate-cell ground Z+200;
missing type or allocation failure suppresses only the animation.

### Armor, Speed, Firepower, Veteran

Walk the live Ground display layer in current order and re-read its capacity
after every candidate; do not snapshot it or substitute entity-store order.
Use crate-center ground Z, exact candidate virtual coordinates, native
approximate square root followed by x87 ftol, and strict distance `< radius`.
Do not owner-filter. Armor, Speed, and Firepower do not add an alive/limbo
filter.

- Armor: exact-one Techno field `*= data`; any affected local-human owner
  latches Armor EVA; picker-owner-local-human sound; animation.
- Speed: exact-one non-Aircraft Foot field `*= data`; any affected
  PlayerControl owner latches Speed EVA; picker-owner-local-human sound;
  animation.
- Firepower: exact-one Techno field `*= data`; any affected local-human owner
  latches Firepower EVA; picker-owner-local-human sound; animation. Candidate
  capability is not retested.
- Veteran: alive/marked trainable Techno; for integer i while `(double)i<data`,
  which means `ceil(data)` iterations for positive fractional data, apply
  the native Veteran-to-Elite, standard-Rookie-to-Veteran, residual-Rookie
  sequence; no EVA; picker-owner-local-human sound; animation.

Use `NativeF64Bits` for all three per-instance crate multipliers. Preserve the
existing Armor consumer. Add a distinct Foot speed-crate field consumed by the
exact current-speed arithmetic; do not reuse locomotor `SimFixed`. Add a
Techno firepower field consumed by the attacker damage pipeline. Do not route
it into GetROF: that function's `Techno+0x2E4` branch is the unrelated bunker
reciprocal partner pointer, and the Firepower handler writes only
`Techno+0x160`. Constructor/default is exact 1.0; snapshot and hash store the
raw bits. Both verified damage consumers preserve native ordering:
`House.Firepower * Techno.CrateFirepower * base_damage`, then one native ftol.
Crate Firepower never changes ROF, burst state, or an enable flag.

### Exact Speed consumer prerequisite

The Speed effect is not closed by storing `Foot+0x580`; every later movement
budget must consume it through the exact `FootClass__GetCurrentSpeed @
0x004DB1A0` stages. Use the existing deterministic `X87Chop53` kernel. Define
`ftol_low32(x)` as chop-mode `FISTP` to signed i64 followed by consumption of
the low signed dword, not a saturating Rust cast. The ordinary active formula
is:

```text
stage1_product = x87_mul53(
    i32_to_x87(native_type_speed_i32),
    exact_f32_to_f64(HouseType.GetSpeedBonus(type_category)_bits)
)
stage1_product = x87_mul53(stage1_product, load_f64(Foot.SpeedCrate_bits))
stage1 = ftol_low32(stage1_product)
stage2 = if HasWeaponAbility(FASTER) {
    ftol_low32(x87_mul53(i32_to_x87(stage1), load_f64(Rules.VeteranSpeed_bits)))
} else {
    stage1
}
result = ftol_low32(
    x87_mul53(i32_to_x87(stage2), load_f64(Foot.CurrentSpeedFraction_bits))
)
```

There is no integer boundary between House speed and crate speed, one optional
boundary after VeteranSpeed, and a mandatory boundary before the stored
current fraction. Do not fuse them. `HouseClass__GetSpeedBonus @ 0x0050C050`
dispatches on the supplied **TechnoType** category: AircraftType `3` selects
`HouseType+0x130 SpeedAircraftMult`, InfantryType `0x10` selects `+0x128
SpeedInfantryMult`, UnitType `0x28` selects `+0x12C SpeedUnitsMult`, and every
other category returns exact `1.0f`. Ship is a Unit locomotor and therefore
uses `SpeedUnitsMult`; there is no naval or building speed field. All three
fields are parsed/stored as `NativeF32Bits`, default exact one, retain their
prior raw bits when a later rules layer omits the section or key, and are not
clamped. Active retail assigns none of them, so installed values remain one;
`1.15` and `115%` both narrow to f32 `0x3F933333` through the native `%f`,
widen, percent, then f32-store path. Malformed nonempty numeric text is a
native address-layout-dependent scanf alias accident and is excluded as
unsupported input, not approximated as an exact mechanism.

`VeteranSpeed` uses the native `%f` binary32 parse promoted to
`NativeF64Bits` (stock bits `0x3FF3333340000000`); stock Speed crate data is
binary64 `0x3FF3333333333333`. Parse `FASTER` independently in Veteran and
Elite ability lists; Elite inherits the Veteran bit. The existing ObjectType
raw `Speed=` remains source authority, but its native 0..255 scaled dword must
be produced by one shared exact helper rather than by dividing a lepton-rate
approximation. Native `Speed=` first clamps the signed INI integer to 0..100,
then computes `(raw << 8) / 100`, and caps the stored result at 255; stock
Infantry `Speed=4` therefore stores 10.

`ftol_low32` must also reproduce the invalid-domain result: NaN, infinity, or
signed-i64-overflow converts to x87 integer-indefinite
`0x8000000000000000`, whose consumed low dword is zero. The exact f32 load may
not reject signed zero, subnormal, infinity, or NaN before this native path;
those raw categories require deterministic kernel fixtures even though retail
HouseType data is ordinary finite one.

Infantry is the sole override of owner vslot `+0x538`.
`InfantryClass__GetMovementSpeed @ 0x00521D80` calls the common helper first,
then applies only this signed-i32 post-adjustment:

```text
N = FootClass__GetCurrentSpeed(this)
if !Infantry.IsProne: return N
if InfantryType.Crawls:
    return wrapping_i32(N - trunc_toward_zero(N / 3))
return wrapping_i32(N + trunc_toward_zero(N / 2))
```

There is no negative/zero early return, saturation, x87 operation, terrain,
health, or fear input in this wrapper. Positive Crawls values happen to equal
`ceil(2*N/3)`, but that identity is not the signed contract. The Crawls
constructor/art fallback is true; installed `WEEDGUY` omits the key and retains
true.

Walk `ProcessMovement` writes exact current fraction one immediately before
its single Infantry speed query and feeds the adjusted i32 directly to its
FILD/sin/cos displacement. Stop/no-destination exits write zero and return
before querying. Walk applies no terrain or damaged-health modifier. Walk
`Is_Moving_Now` does **not** consume the integer: it checks the locomotor moving
byte, ordered-positive current fraction, then non-null destination. Walk body
animation likewise uses moving state plus WalkRate/IdleRate; do not make either
ready state or animation depend on a cached speed integer.

Hover is an active Unit consumer. Its ordinary Move path queries the same
common i32 once and then multiplies it by Hover locomotor `+0x4C`; only the
zero-budget plus exact-facing startup-reset arm queries a second time. Drive
and Ship query once per reached track pass; same-Process retry queries again
before masking only the fresh contribution. The moving archive-target
projection is Unit-only. Mech and Tunnel direct consumers are TS-dormant with
no installed YR ownership.

The remaining locomotor census closes eligibility separately from movement
consumption. The handler mutates `Foot+0x580` for every exact-one
non-Aircraft Foot regardless of locomotor; an eligible type is not entitled to
an invented displacement effect:

- Jumpjet's six installed Vehicles `DISK,HIND,SCHD,SCHP,SHAD,ZEP` and two
  Infantry `JUMPJET,LUNR` are eligible and persist/checksum the crate qword,
  but Jumpjet displacement does not consume it. `Process @0x0054AEC0 ->
  State3_Translate @0x0054BFF0 -> UpdateCoordinates @0x0054D0F0` ramps the
  locomotor's own current `+0x70` toward target `+0x78`, converts `+0x70` at
  `0x0054D55A..0x0054D575`, and applies facing sin/cos. It publishes Jumpjet
  current/max to owner vslot `+0x544` at `0x0054D19D..0x0054D1AE` but never
  calls owner `+0x538`. Speed crates therefore alter persistent Foot state,
  apparent-speed/Unit-leading queries, and crate presentation, not Jumpjet
  displacement.
- Teleport's installed Vehicles `CMIN,CMON,SMON` and Infantry
  `CCOMAND,CIVAN,CLEG` are eligible and persist/checksum the qword, but
  `StateMachineTick @0x007192F0` derives its timer from 3D distance divided by
  Rules `+0xBF4`, then applies `+0xBF8/+0xBFC/+0xC00` gates, while position
  commit `@0x00718260` is immediate. Neither uses linear owner speed. A full
  operand census
  finds `FootClass__GetCurrentSpeed @0x004DB1D5` as the sole runtime read of
  `Foot+0x580`; Teleport has no `+0x538` dispatch. Its movement remains
  unchanged after a Speed crate.
- All eight installed Fly Aircraft `ASW,BEAG,BPLN,CARGOPLANE,HORNET,ORCA,
  PDPLANE,SPYP` and three Rocket Aircraft `V3ROCKET,DMISL,CMISL` are rejected
  by the pickup category-2 gate,
  retaining crate multiplier one. Fly independently uses
  `ftol(Type+0x678 * FlyCurrentFraction+0x48)` in `Process @0x004CD600`/
  `0x004CFE20`; Rocket `Process @0x006622C0` uses Rocket trajectory and
  acceleration fields. Neither reads the crate qword. This is both pickup and
  consumer exclusion, not a generic Aircraft speed approximation.
- Parachute is a temporary descent state, not a locomotor. `ObjectClass::AI
  @0x005F3E70` computes Z through vslot `+0x1D0` plus `Object+0x2C FallRate`,
  commits at `0x005F3F2C`, then adjusts/clamps FallRate through Rules
  `+0x7B8/+0x7BC` at `0x005F3FCB/0x005F3FEE`; no speed helper participates.
  An eligible non-Aircraft Foot retains the qword during descent and landed
  Walk begins consuming it at `0x0075BFC0`.
- DropPod GUID `4A582745`, Tunnel `4A582743`, and Mech `55D141B8` have no
  installed retail binding and are TS-dormant. No separate field reader
  exists: the full `+0x580`
  operand census contains only construction/write/checksum and the common
  helper read.

`LocomotionClass::Apparent_Speed @0x0055AD10` is only a forwarding wrapper to
owner `+0x538`, not an alternative movement formula. Do not call it
speculatively for locomotors whose native movement paths do not. Its result can
nonetheless reflect the crate qword when a real ApparentSpeed consumer calls
it. `TechnoClass::Resolve_ArchiveTarget_Coords @0x0070BD00` likewise queries a
moving Unit target at `0x0070BD4C`; Speed may change that target-leading
projection without changing Jumpjet/Teleport displacement.

Promote a shared per-Foot speed state on `GameEntity` containing the raw
`NativeF64Bits` crate multiplier and current-speed fraction. The native
fraction qword is authoritative: migrate the Drive/Ship
`current_speed_fraction: SimFixed` writer/reader chain to it rather than keep a
shadow fixed-point authority. Target terrain/slope/health and acceleration
remain upstream fraction producers and must not be re-applied in the helper.
Constructor crate multiplier is exact 1.0 and fraction is exact positive zero.

The shared setter accepts a binary64 input and executes ordered comparisons,
not Rust `clamp`/`min`/`max`:

1. compare with 1.0; ordered `>=` stores exact 1.0 bits;
2. otherwise compare with 0.0; ordered `<=` or unordered stores exact positive
   zero bits;
3. only an ordered strict interior `(0,1)` stores the original input bits
   unchanged.

Therefore `+infinity -> 1.0`, `-infinity -> +0.0`, every NaN payload/signaling
form takes the unordered second-compare path and becomes `+0.0`, negative zero
becomes positive zero, and positive subnormals/interior values preserve all 64
bits. Every selected result writes its low dword first and high dword second,
matching the future-state-visible native store sequence. Delete the SimFixed
fraction authority; upstream Drive/Ship target,
terrain/slope/health, acceleration, and braking producers must feed their
native binary64 result through this setter. The snapshot-version bump rejects
older layouts rather than inventing a SimFixed-to-native migration.

The target fraction is a second locomotor-owned `NativeF64Bits`, not a
`SimFixed`. Drive `Process_Movement @0x004B2630` tail
`0x004B357F..0x004B3E27` and the instruction-equivalent Ship tail
`0x006A32D3..0x006A3476` produce it as follows:

1. Compute the signed reference level from the owner's current cell `+0x11B`,
   adding four when owner `OnBridge +0x8C` is nonzero. Resolve the active
   movement candidate. If `abs(reference_level - candidate.Level) >= 2`, use
   the Road row; otherwise use the candidate's `LandType +0xEC` row.
2. Index the native f32 table at `land_row * 9 + SpeedType +0x67C`, load that
   f32, and materialize it as binary64. Ordered values above one become exact
   one; equal, lower, and unordered values remain unchanged.
3. Compare exact-coordinate ground heights, not cell levels. Only a Unit
   applies slope: uphill uses `Rules +0x768` for Track and `+0x778` otherwise;
   downhill uses `+0x770` for Track and `+0x780` otherwise. Materialize the
   x87 product as a qword. Equal height and non-Units do not multiply.
4. Exact zero or unordered becomes exact `0.5`; negative nonzero survives.
   Then obtain the health ratio. Ordered ratio at or below
   `ConditionYellow +0x1700`, and unordered ratio, multiply by exact `0.75`.
5. Selector `+0x58 < 64` stores the result in the locomotor target qword.
   Selector `>= 64` leaves the target unchanged and compares the result with
   the owner's live current fraction: exact equality or unordered skips the
   write, otherwise the shared setter receives the result.

The terrain table parser reads through native `ReadDouble`, whose text path
scans `%f`, widens binary32, and, for a percent token, multiplies by binary64
0.01. It compares that result with one; ordered `>=1` stores exact f32 one,
while the lower/unordered arm calls `ReadDouble` again and stores f32. A
missing section performs no stores, leaving a fresh BSS row zero or a prior
reload row unchanged. A present section defaults missing keys to one, forces
Winged to f32 one, and stores Buildable as a byte. Rules slope and ObjectType
acceleration/deceleration overrides also pass through the f32-to-f64
`ReadDouble` boundary. Missing keys preserve the supplied current qword
without conversion. Rules constructs all four slopes at exact one; active
retail explicitly supplies `1.0,1.2,1.0,1.2`, yielding downhill bits
`0x3FF3333340000000`. Absent per-type acceleration keys retain constructor
binary64 defaults. This target producer runs only from actual
`Process_Movement`, not from a generic every-tick helper.

Drive `Process_Drive_Track @0x004B0F20` block
`0x004B0F69..0x004B1295` and Ship `@0x006A05F0` block
`0x006A0639..0x006A095D` consume target `T` and current `C` in this order:

1. When `Accelerates +0xDBD` is false, call the owner setter with `T`, skip
   passive/selector/ramp/convoy work, then query current speed.
2. Otherwise a Unit whose UnitType `Passive +0xE0C` is true skips every speed
   write, as does selector `>= 64`. `Passive` defaults false and active retail
   contains no assignment, so this gate is preserved but stock-unreachable.
3. Resolve the stored locomotor destination and replace its Z with ground
   height plus 416 only when destination Cell flags `+0x140 & 0x100` is set.
   Use wrapping signed i32 deltas from owner exact XYZ and a caller-specific
   x87 helper that performs `z*z + y*y + x*x`, materializes a qword, then uses
   `Sqrt_Approx` and chop. The existing x/y/z helper is output-equivalent for
   active retail map deltas but is not instruction-order exact. Drive and Ship
   use separate bridge globals but both produce the same 416 offset. Thus
   planar 100 plus bridge Z 416 produces native distance 427, below stock 500.
4. For strict signed `D < SlowdownDistance`, compute
   `C - f64(type_speed) * DeaccelerationFactor`, materialize the qword, and
   apply the ordered floor at promoted-f32 `0.3`
   (`0x3FD3333340000000`). NaN survives this max and the setter canonicalizes
   it to zero. Otherwise, when owner `+0x3CD` is set, use promoted-f32 rate
   `0.0015` (`0x3F589374C0000000`) and floor `0.1`
   (`0x3FB99999A0000000`).
5. `CurrentlyCrushing +0x6B5` overrides either brake candidate with ordered
   min of `T` and exact binary64 `0.2` (`0x3FC999999999999A`); unordered
   selects `T`. Write the chosen qword back to target and call the setter.
   Otherwise a brake candidate goes to the setter.
6. Without braking, ordered `C < T` or unordered computes
   `C + AccelerationFactor`, caps at `T` with unordered selecting `T`, then
   calls the setter. Ordered `C > T` computes
   `C - f64(type_speed) * DeaccelerationFactor`, floors at `T` only when
   ordered `T > candidate`, then calls the setter. Exact equality performs no
   owner write.

Stock acceleration is exact binary64 `0.03`
(`0x3F9EB851EB851EB8`), deceleration `0.002`
(`0x3F60624DD2F1A9FC`), and slowdown distance 500. An explicit INI override
uses the widened-f32 `ReadDouble` result instead. Drive obtains type speed
through the Unit virtual slot while Ship reads `+0x678` directly; they are
numerically identical for all active retail Units.

Installed retail overrides are part of the data contract: DNOA, DNOB, V3, and
SQD use acceleration `0.01` (`0x3F847AE140000000`); DRON and CAOS use both
factors `5` (`0x4014000000000000`); SMIN uses deceleration `0.2`
(`0x3FC99999A0000000`). Every other installed type retains the exact
constructor qwords for missing keys.

After the normal accelerated, nonpassive, selector-below-64 branch, both
Drive and Ship Unit locomotors propagate the owner's now-normalized qword.
Starting at owner `+0x6C8`, apply the initial linked member, then advance and
stop on null or before a newly reached self-linked terminal; an initially
self-linked first member is therefore applied once. Invoke each applied
member's virtual setter in native order. Stable-ID/re-fetch iteration is
required across synchronous writes. Nonaccelerated and all skipped branches
do not propagate. Every path then invokes the owner's current-speed query. A
same-Process retry repeats target/current updates, propagation, and the query,
but adds zero fresh speed to residual; it does not reuse the first query.

Top-level scheduling is also part of the fraction contract. An active Track
executes Track(0) first; if completion creates a new path leg, it runs
`Process_Movement` then Track(1), whose retry adds no second fresh speed. With
no active Track gate, `Process_Movement` runs before Track(0) in the same tick.
Sinking clears the path head and target qword without calling the owner setter.
Fully idle state clears current only for ordered `C > 0`; zero, negative, and
unordered skip. Terminal `Process_Movement` abort sets selector/path head to
`-1` and unconditionally sends zero through the setter while preserving the
target qword.

Forced-track success writes the locomotor target qword to exact one, resets
the cursor, and preserves residual/short-byte state; it does not write the
owner current fraction. Its callers then provide the owner writes: land
war-factory selectors 66 set exact `0.5` before building state 3; tank-bunker
install selectors 67..70 and undock/eject selector 71 set exact one. The
misnamed `ReleaseDockedHarvester` path is bunker reciprocal release, not
refinery unloading. Action 128 relocation and `PerformDeploy` use selector
`-1` with no direct current write. The existing Rust behavior that makes
forced track set both target and current, and labels every `>=64` selector
bunker-only, must be removed.

The generic owner-writer census additionally includes arrival/EnterIdle zero
only when NavQueue is empty; a nonempty queue pops/reissues and returns first.
Move reissue writes one only for mission-slot result 2, locomotor not moving,
and non-null NavCom. Eligible on-screen Unit/Infantry Wave writes zero before
rocking/damage. Chrono placement writes one after Unlimbo/destination and
before facing/occupation.

Tube is category-specific. Blocked Unit and Infantry both send exact zero
through the shared owner setter, retain Tube state, and let the wrapper
continue. Unit success sends exact one after its `+0x18C` write. Ordinary
Infantry success sends exact one before its `+0x18C` write. The Infantry arm
whose Tube comparison equals object `+0x5A4` instead invokes virtual `+0x174`
and deliberately preserves the prior raw current fraction. Every Tube arm
leaves Drive/Ship target unchanged. Active TubeMovement owns the entire object
turn, so no later generic same-tick movement update may overwrite zero, one,
or the preserved qword. Unit setter evidence is `0x00735F66..0x00735F6A` and
`0x00736047..0x0073604F`; Infantry is
`0x0051B8F8..0x0051B8FC`, `0x0051BA79..0x0051BA81`, with preserve vcall at
`0x0051BA6F`. Low-bridge auto Tube remains active without explicit `[Tubes]`
data.

This Drive/Ship target/ramp producer is not a license to omit other Speed-crate
consumers. Hover and Walk use the common owner-speed helper at the exact call
positions above while retaining their own locomotor fraction producers.
Aircraft are excluded by the crate pickup handler even though AircraftType can
select `SpeedAircraftMult`. The Jumpjet/Teleport and remaining active
locomotor ownership census above proves whether each eligible non-Aircraft
Foot consumes the stored multiplier or merely persists it; every shared
setter call still routes mechanically to the authoritative owner field.

There are no other direct `Foot+0x578` stores: constructor zero and the
setter's three low-then-high arms are the complete storage-writer census.

Required Rust touchpoints are explicit:

- `src/rules/ruleset.rs`: all three `CountryRules.SpeedInfantryMult`,
  `SpeedUnitsMult`, and `SpeedAircraftMult` values as raw f32 bits, one
  ObjectCategory selector with Building/unknown exact-one fallback,
  `GeneralRules.VeteranSpeed` as binary32-promoted f64 bits, native terrain
  row presence/defaults and f32 values, and binary64 slope factors;
- `src/rules/object_type.rs`: Veteran/Elite `FASTER` bytes and exact native
  scaled-speed helper input plus raw binary64 constructor and widened-f32 INI
  acceleration/deceleration values, and `Accelerates`, `Passive`, SpeedType,
  and SlowdownDistance state;
- `src/sim/game_entity.rs` and snapshot/hash/checksum: persistent per-Foot
  crate/current qwords, exact-coordinate/bridge/sinking/crushing state, real
  convoy next-link identity, and the already-owned raw veterancy rank;
- `src/sim/components.rs`: convert Drive/Ship target fractions to raw qwords
  and remove their duplicate current-fraction authorities;
- `src/sim/movement/drive_locomotion.rs`: replace
  `owner_current_speed_from_fraction` with the three-stage helper and migrate
  current and target fraction, exact ProcessMovement producer, Track ramp,
  caller-specific z/y/x distance, scheduling, forced-track, and convoy
  propagation to authoritative qwords;
- `src/sim/movement/movement_commands.rs`: delete the path-install-time
  `update_drive_speed_fraction(..., target=1, ...)` call. Accepting a Move path
  installs path/turn/destination state only; it must not mutate target or
  current fraction before the scheduled native ProcessMovement/Track rung,
  including for `Accelerates=false` types;
- `src/sim/movement/mod.rs`: make ForceTrack target-only and move factory-half
  and bunker-one owner writes to their verified caller positions;
- `src/sim/movement/navcom.rs`: replace direct fixed-point idle, sinking, and
  abort writes with the ordered current/target gates above;
- `src/sim/movement/tube_movement.rs`: route Unit/Infantry blocked zero, Unit
  success one, Infantry ordinary-success one, and Infantry special preserve
  through category-specific owner-qword timing without changing target or
  falling through to a same-tick generic overwrite;
- `src/sim/world/techno_ai/mission_handlers.rs`, the nonmoving Move reissue,
  Wave splash, and Chrono placement owners: install the verified generic
  writer gates and order rather than ad hoc Drive/Ship field writes;
- `src/sim/world/world_commands.rs`: stop imposing the non-native command-time
  `max(25)` floor or treating `LocomotorState::speed_multiplier` as crate state;
- `src/sim/infantry.rs`: replace the positive-only saturating prone helper with
  exact signed division plus wrapping add/sub and make missing `Crawls=` retain
  the native true constructor/art fallback;
- `src/sim/movement/movement_tick.rs`: query the entity/type/House/rank state
  after fraction update and cache the exact i32 result for Drive and Ship;
  Walk writes fraction one immediately before its adjusted query and consumes
  that i32 directly, while Hover samples at its verified one-or-two call sites;
- `src/sim/movement/movement_step.rs` and ready/animation consumers: consume
  the cached i32 unchanged where native does; same-Process retry recomputes the
  full query but masks only its fresh contribution before adding residual.
  Keep Walk readiness and Walk animation independent of the integer query.

After stage 3 native conditionally applies signed division by two for a Unit
whose CTF flag-owner index is not `-1`. Stock `CaptureTheFlag=no`; that state
machine belongs to Phase 13. Phase 3 therefore keeps the sentinel absent and
tests the ordinary no-half branch, while this documented signed
`(value - sign)/2` continuation remains mandatory when the Phase-13 CTF owner
is introduced. It is not TS legacy and must not be misnamed as docking state.

## Presentation contract

Add explicit crate event variants rather than hiding native gates in the app:

- spatial sound event with resolved sound identity and original crate world
  position;
- EVA event carrying the exact affected-owner gate result and EVA kind;
- common crate animation request carrying type identity, crate-center ground
  coordinate plus 200, draw flags `0x600`, and creation order.

The common tail allocates the ordinary Anim object at the native `0x1C8` size
and invokes its equivalent constructor with `(0,1,0x600,0,0)`. Allocation
failure is silent.

Simulation decides all ownership/human/PlayerControl gates and ordering. The
app only resolves audio assets and local playback. Reuse the normal authoritative
AnimClass creation path, not the multiplayer move-feedback sentinel path: crate
animations are gameplay-created normal Anims and participate in ordinary
animation registration, lifecycle, draw ordering, save/hash, and sound.

## Trigger and object ingress

### Ordered per-Tag runtime

Replace the current trigger-ID aggregate with native ownership. The minimum
future-affecting records are:

```text
TagRuntime {
    tag_type_id,
    trigger_instances,        // native linked-list order
    attachment_count,         // signed wrapping i32 at native +0x2C
    attached_cell_sentinel,
    disabled_or_uninit,
    busy,
    registered,
    pending_finalization,
}
TriggerTypeRuntime {
    trigger_type_id,
    events,                   // shared nodes in native linked-list order
    actions,                  // shared immutable nodes in native list order
}
EventRuntime {
    definition,
    last_raising_owner,       // shared native TEvent+0x54
}
TriggerInstance {
    trigger_type_runtime_id,
    next,
    raising_house,
    pending_delete,
    timer_start_frame,
    opaque_timer_word,        // normalized zero; native stack residue is inert
    timer_duration,
    satisfied_mask,
    enabled,
}
```

Construct one independent `TriggerInstance` for every TriggerType in a Tag's
linked chain and push each at the head, producing reverse chain order. Reusing
one Tag ID on multiple attached objects/cells shares the first `TagRuntime`
and increments its signed wrapping attachment count. Two different Tag IDs
that point to the same TriggerType own independent instance-local enable,
pending, timer, mask, and raising-House state, but reference the same
`TriggerTypeRuntime`. Its `TEvent+0x54` owner memory is mutable shared state:
an Event owner write through one Tag is visible when another Tag's instance of
that TriggerType evaluates later. Event/Action definitions are never cloned
per instance.
Maintain separate stable registries: Tag master order is first materialization,
global TriggerInstance order is construction order, while the globally polled
category-`0x10` Tag list is appended independently in `[Tags]` source order.
Parser push-front construction makes Events run in reverse textual CSV-chunk
order. Actions use a separate append construction and run in textual
CSV-chunk order. Do not sort either list or trigger/tag identities.

Construction begins with `enabled=true`, appends the instance to the global
registry, and then runs the common timer reset before applying the final
field-three/Scenario-difficulty enable gate. Timer reset walks Events
head-to-tail, which is reverse textual chunk order. Event 13 writes
`start=binary_frame` and `duration=scalar.wrapping_mul(15)` and clears that
Event index's aliased satisfied-mask bit. Event 51 draws
`ScenarioRng::random_ranged(0, scalar)` even for an instance that the final
gate disables, computes `scalar / 2` with signed truncation toward zero, writes
`duration=(scalar / 2 + draw).wrapping_mul(15)` and the current start, and
clears its aliased bit. Every Event 51 spends a draw even when a later timer
Event overwrites the shared start/duration. The native `+0x38` word is
uninitialized stack residue that raw save happens to carry but no semantic or
quick-CRC consumer reads; typed Rust must keep it normalized to zero rather
than simulate nondeterministic garbage.

Condition evaluation rejects an instance whose `enabled` is false or whose
`pending_delete` is set. Repeat mode two bypasses its Event list and succeeds.
Every other mode visits every Event in native runtime order with no boolean
short circuit. Event index `i` owns bit `1 << (i & 31)`, so lists beyond 32
alias. A set bit is already satisfied and is not reevaluated. Every satisfied
or prelatched Event may supply `last_raising_owner`; the last non-null owner in
runtime order wins `raising_house`.

The evaluator's shared persistence byte starts true only for repeat mode two.
Event 1 sets it. A successful qualifying persistent Event sets its latch bit,
except the native nonlatching classifier excludes Event 1 and Event 8. Events
49 and 50 are latch-eligible. When every condition is true and persistence is
set, rearm the TriggerInstance timer. Events 1, 8, 49, and 50 consume no RNG.

Tag event delivery rejects editor mode, a busy Tag, a disabled/uninitialized
Tag, or a null TagType. It sets `busy`, visits every TriggerInstance without
stopping after a match, and springs matching Actions synchronously. Every
Action is invoked in native runtime order; an Action's returned boolean does
not stop the list.

- Repeat zero springs matching enabled instances, marks each sprung instance
  pending-delete and queues it, then after the full instance list clears busy,
  detaches the triggering object if it still points at this Tag, detaches the
  passed cell when its coordinate is not the sentinel, logically unregisters
  the Tag, queues physical finalization, and returns whether any instance
  sprang. Physical free is late in the main tick.
- Repeat one reads the exact signed attachment count. At count other than one,
  a satisfied instance does not Spring or queue; the supplied matching source
  object and/or non-sentinel cell detach, and the Tag stays live. At exact one,
  every satisfied instance Springs/queues, the Tag logically expires, pointer-
  expiration clears all remaining references, and physical destruction stays
  deferred. A detach that changes count to one does not reevaluate. Count zero
  is live/inert and no clamp exists. This is active for 16 campaign Tags.
- Repeat two bypasses conditions; every enabled, non-pending instance springs
  and remains registered without detaching or queueing. The active 184-map
  corpus has 584 repeat-two Tags (including two standard-skirmish Tags), 1,926
  repeat-zero Tags, and 16 campaign-only repeat-one Tags. The 13 ordinary
  standard-skirmish action-108 calls are repeat zero; four additional campaign
  action-108 owners use repeat two.

The polling iterator resets to index zero each tick, processes the live entry,
then unconditionally increments. Repeat-zero/repeat-one-last cleanup stable-
erases polling membership synchronously through pointer expiration. Therefore
`[A,B,C]` with A retiring becomes `[B,C]`, then the incremented index processes
C; B waits until next tick. Do not repair this ordinary cleanup skip. The
deferred finalizer performs physical Trigger/Tag release on the late main-tick
rung and stable-compacts master registries.

Fresh runtime materialization is not a `[Tags]`-only pass. Preserve ordered
definitions plus lookup indices, then ensure/reuse each Tag in this exact
sequence: valid unoccupied `[CellTags]` source rows, `[Units]`, `[Aircraft]`,
`[Infantry]`, `[Structures]`, then category-mask `4`, `0x10`, and `8` postpass
walks in `[Tags]` source order. Cell and object attachment setters increment
the shared signed count; repeated successful CellTags overwrite the stored
attached cell with the last such cell. Tag construction appends the Tag master,
then constructs its TriggerType chain head-to-tail, appending the global
TriggerInstance registry while push-fronting the Tag-local list. Event-13/51
timer construction, including Event-51 RNG, occurs in that global construction
order before the field-three/Scenario-difficulty enable gate is applied. An
unattached Tag with none of the
three category bits has no runtime and cannot be Force/Enable/Disable targeted.

Runtime Team construction is the one no-reuse path: each Team directly creates
a distinct Tag/Trigger group at Team creation time and member add/remove owns
its attachment count. Active retail's initial load has no Team interleave with
the sequence above. The 184-map TriggerType graph is a forest: 3,186 types,
2,526 Tag heads, 659 links, 3,185 owned nodes, no fan-in/cycle/self-link/
dangling reference, one inactive unowned node, and maximum chain length 30.
Those facts exclude malformed/cyclic initial graphs without weakening runtime
Team duplicates or native unguarded recursive Force behavior.

### Existing trigger mechanisms retained through the migration

Replace the sorted `TriggerGraph`/ID queue only as an execution owner;
diagnostics may remain. Use an immutable source-ordered `TriggerProgram` plus
serialized mutable Tag/Trigger/Event registries. `Simulation::advance_triggers`
must create one explicit `TriggerTransaction` that owns the temporarily moved
runtime and borrows the disjoint world authorities. Every action and object,
cell, capture, or crate callback receives that same transaction recursively.
No nested path may consult the placeholder `Simulation.trigger_runtime`, and
no synchronous native callback may be deferred into a later command queue.
Same-Tag recursion is rejected by `busy`; a different Tag remains reentrant.

`[Events]` is variable arity, not fixed triples. After the count, ParamType 0
consumes `kind,param_type,scalar`; ParamType 2 consumes
`kind,param_type,scalar,type_name`. The active 184-map corpus contains 3,540
type-zero and 78 type-two entries and no leftovers under that schema. Store a
signed lenient-atoi scalar separately from ParamType. The current runtime's
`params[0]` reads ParamType and is wrong. Event behavior retained/corrected is:

- 27/28: signed global index 0..49 is set/not set;
- 36/37: signed local index 0..99 is set/not set;
- 47: `event.scalar <= signed(current_frame / 15)` with C truncation;
- 60: case-sensitive TechnoType lookup, backward live Techno-vector scan,
  signed threshold/early `count >= threshold`, and no owner/alive/limbo filter;
- 61: the same lookup/scan and true iff no exact-type entry exists.

These seven Events are nonlatching. Reject invalid variable indices at typed
map compilation: native indexes adjacent stack bytes, which is invalid-domain,
and active retail values are in range. On the leading Logic rung, poll Tags in
the live category list. For each Tag the first successful delivery rung wins:
Event50, dirty global/local 27/28/36/37 deliveries, the remaining scenario
latches, timer passes 13/51, then mission timer 14. A variable value change is
synchronously visible to later Tags, marks its dirty delivery, and rearms every
Event-51 instance that references it, consuming any timer RNG immediately;
an unchanged write does none of those. Already-completed Tags are not revisited.

All counted Actions retain eight tokens and materialize typed fields at read
time; ParamType is not the operand. Continue executing the current supported
set synchronously in textual order:

- 22 scans the live global TriggerInstance registry forward and directly calls
  Spring on every matching TriggerType. It bypasses Events, Tag busy/repeat
  cleanup, and queue/dedupe; enabled/pending Spring gates still apply. Forced
  repeat-zero instances are not consumed. It returns true for a valid target
  and nonempty registry even when no instance matches. Native recursive Force
  is unguarded; the sole active retail Action22 is nonrecursive.
- 28/29 set/clear global; 56/57 set/clear local. Use the materialized signed
  scalar and range contract above, with synchronous dirty/rearm behavior.
- 40 retains the existing synchronous native visible-area mutation and full
  terrain/zone/radar/building refresh; later Actions see the new authority.
- 53 scans all exact TriggerType matches in the live global registry forward.
  It ignores current enabled state, pending-delete, and authored field three.
  For Scenario difficulties `0..=2`, only the matching Easy/Medium/Hard field
  admits the instance; an out-of-range raw value admits every match. An
  ineligible match is unchanged. For each admitted match it writes
  `enabled=true` first, then runs the same reverse-Event timer reset, including
  every Event-51 RNG draw and mask clear. 54 scans every exact match including
  pending instances and only writes `enabled=false`; it applies no difficulty
  or field-three filter and performs no timer, mask, or RNG work. Both actions
  return true for null/empty/no-match/all-ineligible inputs and neither
  evaluates conditions.
- 137/138 retain the already verified source TriggerType owner -> first
  registration-order House resolution and alternate-base-only writes.

Actions 48 and 112 are ordered camera commands, not the current deferred
boolean effect. Both resolve the full 702-slot waypoint table; a valid missing
slot is packed `(0,0)` and is not a no-op. Both build cell-center XYZ with
exact slope-aware ground Z plus 416 for Cell flags `0x100` or `0x400`, project
through `adjust_for_z_standard`, have no House/human/local gate, and return
true. Invalid decoded/out-of-range waypoint indices are native OOB and must be
rejected at typed map compilation.

Action 48 arms app-owned `TacticalCameraMotion` using signed Param3 selector
0..4 and exact f32 speeds `[0.0015,0.003,0.0075,0.03,0.06]` (completion
steps `[667,334,134,34,17]`); reject other selectors. It captures current
committed center, replaces target/speed, and resets f32 progress. Action 112
immediately writes committed/requested center without canceling a pending
glide. Therefore `48->112`, `112->48`, two 48s, and two 112s remain order-
sensitive. Tactical AI advances at most once per binary sim frame after trigger
commands and before follow-camera, gates replay/scenario-active, treats target
`(0,0)` as disabled, performs actual f32 progress add/cap, f64 axis lerp with
truncate-toward-zero and tactical clamp, and clears glide only on completion.
Serialize this local presentation state (including last processed frame) and
resume mid-glide without a same-frame double step; exclude it from Simulation
world hash and retail checksum, matching native Tactical CRC/replay boundaries.

Actions 67/68/69 are synchronous local-player House result operations. Pass
the app-pinned `MatchState.local_player_owner` into `TriggerTransaction` as
per-client context; validate it before runtime and never infer the first human,
PlayerControl House, trigger owner, or raising House. All three wrappers ignore
operands and return true. Remove `last_announcement`, its snapshot/hash state,
hardcoded trigger messages, and direct Action-69 result screens.

House result authority must expose independent pending/win/loss bytes plus one
shared `{start,duration}` timer; the existing optional outcome enum cannot
represent pending with neither terminal byte. Fresh House state is all bytes
zero, `start=creation_frame`, `duration=0`. Action67 calls Win with skip-timer:
it accepts only when all three bytes are zero, sets win, preserves timer, and
emits the localized immediate notice only on transition. Action68 clears win
unconditionally, then if pending or already lost does nothing else; otherwise
it sets loss, preserves timer, and emits one accepted notice. Action69 calls
Win with normal timer only when neither terminal byte is set; pending may reject
it. With win/loss set it writes only `start=current_frame` when start is `-1`.
It emits no direct result.

House update later in the same Logic rung owns timer expiry and result routing;
pending expiry clears pending and scatters units. Preserve textual same-stack
outcomes: 67->68 loss; 68->67 loss; terminal 67/68 then 69 only repairs a `-1`
start; 69->68 changes armed win to loss while retaining its timer; 69->67
retains the armed win. Persist/hash all future-affecting result fields. Active
network quick checksum `0x64DAB0` folds only `House+0x241 map_is_clear`; it
omits the result bytes and TimerClass remaining time. The latter belongs to the
distinct full House CRCEngine `0x502D60` and must not enter the Rust network
quick-checksum oracle.

The active corpus has two Action68 rows, in `all01umd` and `all04dmd`, and zero
Action67/69. Both must target the pinned local player despite authored
`Americans` Trigger owners. Action12 remains outside this promoted crate
prerequisite: its 27 active campaign-only calls do not co-occur with Action108
and are owned by later full campaign GSI-10.12 closure. Its no-op status is
unchanged by this migration. The sole active Action22 edge is non-self/acyclic;
typed compilation rejects synthetic Force self/cycles rather than adding a
non-native runtime recursion guard.

### Action 108 and retail Events 1/8

Materialize each counted eight-token Action chunk as
`ActionID,ParamType,Param3,Param4,Param5,Param6,Param7,WaypointCode`. Add a
signed `i32 materialized_operand` representing native `Action+0x90`; do not
narrow it to the eventual crate data byte or reparse `params` during
execution. Constructor/read starts at zero, then applies:

| ParamType | `materialized_operand` |
|---:|---|
| 0 or 11 | lenient native `atoi(Param3)` |
| 5 or 9 | lenient native `atoi(WaypointCode)` when present, otherwise zero |
| 6 | dialog-registry index, including `-1` unknown |
| 7 | sound-registry index, including `-1` unknown |
| 8 | theme-registry index, including `-1` unknown |
| other | zero |

The current sim parser does not own dialog/sound/theme registries. Preserve
those raw tokens as an explicit unresolved-registry variant and reject an
Action 108 requiring ParamType 6/7/8 when compiling the map runtime, before
`ScenarioSession` exists. Do not silently substitute zero or `-1`. This is an
evidence-backed inactive boundary: all 13 installed Action-108 chunks use
ParamType zero. Other Action kinds retain their raw unresolved representation
for their owning phase rather than making the whole map unloadable.

Native mandatory tokens ActionID through Param7 are unconditional
`strtok -> atoi`; missing tails and comma-collapsed empties are invalid-domain
inputs in retail. Rust must reject malformed counted chunks safely and
deterministically. Optional missing/trailing-empty WaypointCode retains the
constructor zero. Native `atoi` accepts leading whitespace/sign and decimal
prefix, stops at junk, returns zero with no digits, and wraps to i32; reuse the
project's exact lenient parser. Waypoint materialization retains constructor
zero for ParamTypes 5/9/11; every other type decodes a present alphabetic token
through the two-letter waypoint decoder, where nonalphabetic is the signed
`-1` sentinel. Store the exact signed projection or its safe `Option` identity,
not a separately guessed action-108 waypoint.

Add an explicit `NativeActionResult::{False,True}`. Action 108 resolves its
signed `Action+0x44` through the scenario waypoint table, passes the full
signed `materialized_operand` to `place_specific`, and returns that helper
boolean—including true for an accepted ghost. A Spring-equivalent executes
every Action in textual CSV order without short-circuiting and returns the OR
of all Action results. Tag delivery ignores that OR: its own return and
repeat-zero retirement depend on condition/repeat state, so a satisfied
all-false Action list still unregisters and queues its one-shot Tag. Every
other native Spring caller also discards the OR; it is preserved as an exact
API boundary, not promoted into cleanup policy.

Installed action-108 execution is fixed:

| Map | Textual order | Waypoint/cell | Data/effect |
|---|---:|---|---|
| `xxmas.map` | 1..11 | `CA..CK` / `(63,67)`, `(64,62)`, `(71,64)`, `(74,73)`, `(69,78)`, `(82,77)`, `(79,85)`, `(71,86)`, `(80,94)`, `(91,82)`, `(88,72)` | `0,10,9,0,14,11,0,10,9,0,14` / Money, Speed, Armor, Money, Veteran, Firepower, Money, Speed, Armor, Money, Veteran |
| `xarena.map` left | single | `CB` / `(69,114)` | `2` / HealBase |
| `xarena.map` right | single | `CA` / `(99,52)` | `8` / Reveal |

The full dword is load-bearing: exact `20` suppresses the placement post-write,
while `276` is not that sentinel and writes low byte `0x14`.

Event 8 evaluates true regardless of the raised event ID, but it has no native
`Process(8)` caller. `xxmas.map` is first delivered unconditional Event 13 on
the first pre-object Logic trigger rung, after earlier optional trigger latches
are cleared. Evaluation of that delivery satisfies Event 8, springs the
repeat-zero CreatePresents Tag synchronously, and executes its eleven action-108
placements in textual `CA..CK` order exactly once before logical unregister
and late physical finalization.

Event 1 (`Entered By`) is synchronous object-raised, records the raising owner,
sets evaluator persistence, and is explicitly nonlatching. Its owner/country
filter is disabled only by exact `-1`; otherwise the raising House must match.
Map object tags connect the raised object to its shared Tag runtime. In retail
`xarena.map`, after the strict under-128-lepton arrival gate, engineer capture
reads the target Building's AttachedTag and raises Event 1 with the engineer as
the object. The callback writes the engineer owner's identity and completes
action 108 before target Guard mission, target Limbo, capture EVA/detach,
ownership transfer, Building tag replacement, and engineer destruction. No
polling surrogate, Building-as-raising-object substitution, or post-transfer
delivery is allowed. Both event rows are `1,1,0,-1`, so either player's
engineer passes; each repeat-zero Tag places its fixed HealBase or Reveal crate
once and then follows the exact repeat-zero cleanup order above.

### Events 49 and 50

Crate pickup delivers Event 49 only to the collector's AttachedTag, synchronously
before the collector liveness reread, global event-50 latch, any RNG, or crate
removal. It requires exact raised ID 49 and non-editor mode, with no House/data
gate and no persistence or raising-owner write. If its Actions kill the
collector, pickup returns before latch/RNG/removal and the crate remains.

A surviving pickup sets the global event-50 latch. On the next leading trigger
rung, attempt Event 50 first for every globally registered Tag in source order.
If one Tag fires, skip only that Tag's later per-tick event ladder; continue the
global walk. Event 50 has the same exact-ID/non-editor/no-House/no-data/no-owner
contract as Event 49. Always clear the global latch after the complete walk.

### CrateBeneath and CarriesCrate

`CrateBeneath` is ordinary fatal-building behavior, not fresh placement and
not the stale “Iron-Curtain carryover only” claim in older Building reports.
The misleadingly named `BuildingClass__Place_OccupyMap @ 0x00441F60` has
exactly two callers: fatal `ReceiveDamage @0x004426A2` and the zero-health
`BuildingClass__Update @0x004400E5` fallback. All stock CrateBeneath types have
`Explodes=no`, so ordinary result-4 death's duration-8 path synchronously calls
`ObjectClass::UnInit` and then this body. Duration-zero/already-expired death
defers to Update, whose order is Limbo, SpawnSurvivors, UnInit, then this body.
A lethal hit during Selling eventually reaches it; voluntary sale does not.
Damage result 5, construction, Unlimbo, capture, generic despawn, and normal
placement do not call it.

Inside the body, native first refreshes every available foundation delta in
order: obtain foundation data, obtain render coordinates, convert to cells,
merge/dirty radar and screen rectangles, write `Cell+0x44=0xEF` and
`Cell+0x40=0`, recalculate attributes, assign orphaned zone, incrementally
rebuild the zone graph, detach target references/restore missions, restore the
origin cell's BuildingType pointer, then dirty/queue redraw. Missing foundation
data skips that refresh but not the crate tail.

The crate tail runs after UnInit and foundation refresh while the Building
tombstone is still resolvable. It reads `CrateBeneath`; false returns. True
re-reads vslot `+0xAC`, the exact `BuildingClass__GetRenderCoords` result:

```text
render = (LocationX - 128, LocationY - 128, LocationZ)
cell_axis = (render + ((render >> 31) & 0xFF)) >> 8
```

Arithmetic is wrapping i32; the low signed i16 is passed to specific placement.
This is signed `/256` toward zero and selects the northwest foundation anchor,
not the building center. Foundation size and odd/even dimensions never shift
it. `CrateBeneathIsMoney` passes data zero; otherwise exact `0x14`. The helper
return is ignored. There is no Crates, mode, owner, House, alive, foundation-
size, or player gate.

The Rust hook is exactly after `uninit_with_context` returns in the
`AfterDeathEffects` fatal-Structure branch in `src/sim/world/mod.rs`, before
deferred physical deletion. Snapshot type flags and stable ID before UnInit,
then re-fetch the tombstone's post-UnInit Location/type and invoke the shared
specific-placement helper once. Do not hook generic UnInit, occupancy removal,
Limbo, or deletion: those would double-drop. A future native duration-zero
deferred-building owner must call the same one-shot post-UnInit adapter.

The unit fatal-damage tail checks CarriesCrate, session Crates, and the resolved
TruckCrate/TrainCrate flag, performs the outer FNPC/invalid check, then calls
the helper that snaps again with full `0x14`. Retail tests must show all four
installed TRUCKB objects remain no-op because both flags are false; synthetic
tests activate each exact branch.

## Pickup callsite integration

Route every verified call through `pickup_crate_at`; do not create a generic
"cell changed" approximation that fires at extra times:

- Hover movement helper;
- Jumpjet movement helper and state-4 descend;
- Drive ForceTrack, ProcessDriveTrack, and both ProcessMovement callsites;
- Ship ForceTrack, ProcessDriveTrack, and both ProcessMovement callsites;
- Teleport arrival;
- Walk FindSubCellDest.

Each adapter snapshots its requested native inputs, releases the entity borrow,
dispatches pickup, re-fetches by stable ID/tombstone, then follows that caller's
verified continuation below. `One`/`Zero` name the native pickup return, not a
consumed bit. Native member offsets are retained here where a semantic Rust
field split could otherwise hide a write. No adapter may normalize a dead or
limboed tombstone before the specified same-stack raw operations finish.

### Drive/Ship ForceTrack

Drive `0x004B0C40/@0x004B0D1B` and Ship
`0x006A0310/@0x006A03EB` first write locomotor selector `+0x54` and track index
`+0x58=0`. A non-null request clears the prior destination/validity, installs
the original requested XYZ/validity, then dispatches at the resolved cell.

- `Zero` or limbo: if alive, clear destination/validity; if dead, write
  nothing and retain installed/callback state.
- `One` and unlimbo: do not test alive. Raw-apply the original requested step,
  copy original XYZ to head-to `+0x30..+0x38`, and write the low/high dwords of
  the locomotor target qword `+0x4C/+0x50=1.0`. Do not write the owner's
  current fraction. Do not reinstall destination, so a callback retarget
  survives. Explosion death with `One`/unlimbo still performs these writes.

Ship ForceTrack is mandatory before resuming SQD. `ParasiteClass::Attach`
then writes victim backlink and manager victim even when pickup killed or
uninitialized the limboed SQD.

### Drive/Ship ProcessDriveTrack

Drive `0x004B0F20/@0x004B1DBE` and Ship
`0x006A05F0/@0x006A1401` first capture the collector's exact raw
`Foot.CurrentSpeedFraction` low/high dwords before candidate classification,
CanEnter, accepted-track setup, destination installation, or pickup dispatch.
Retain `{stable collector ID/tombstone, immutable original candidate,
saved NativeF64Bits}` across callbacks. Accepted-track setup writes `+0x60=0`, the new
selector to `+0x58`, and `step_count-1` to `+0x5C`, then performs the native
temporary-validity and alive/unlimbo/status gate. It installs/resolves the
original candidate destination before dispatch.

- `Zero` or limbo: alive clears destination/validity; dead retains it.
- `One` and unlimbo: no alive recheck. Raw-apply the original candidate,
  write the saved pre-dispatch fraction through the exact speed setter, copy
  exactly 23 dwords `Foot+0x5E4..0x63C` to `+0x5E0..0x638`, then set
  `Foot+0x63C=-1`, advance the point cursor, and continue the paid-point
  loop/residual tail. Callback movement does not change the applied candidate;
  callback retarget survives; dead/unlimbo still receives raw writes.

The success setter never rereads current/target fraction, crate multiplier,
integer speed, or Rules. Event-49/action/effect mutation of the live current
fraction is overwritten by the saved qword on `One`/unlimbo, but remains on
`Zero` or limbo because the setter is skipped. Speed's separate `Foot+0x580`
crate-multiplier mutation always persists. Preserve every special bit through
the integer snapshot; the setter alone canonicalizes it under its verified
table.

### Drive/Ship ProcessMovement first/candidate pickup

Drive `0x004B2630/@0x004B405D` and Ship
`0x006A1C80/@0x006A3689` compute the first candidate before pickup. Occupancy
may replace the second direction and write locomotor `+0x64`; setup always
writes `+0x60=0` and selector `+0x58=(u20+8*u19)`, or `9*u19` for an empty
descriptor. Dispatch occurs only when descriptor flag bit 3 is set, against
that already-computed candidate; this call does not clear or install the live
destination.

- `Zero` and unlimbo: force CanEnter result 7. Dead returns zero immediately;
  alive enters the existing result handler. Its rejection writes
  `Foot+0x5E0=-1`, locomotor `+0x58=-1`, nulls the local endpoint, then common
  finalization clears any live callback destination, writes
  `Foot+0x63C=-1`, packed-null endpoint `Foot+0x558`, `Foot+0x68A=0`,
  locomotor `+0x5C=0`, and stops speed.
- `One` or limbo: dead returns zero. Alive computes the second curve cell from
  the original endpoint plus `u20` (callback movement is ignored), calls
  CanEnter and native coercions, then returns zero if a second alive check
  fails.
- Result 0 shifts 22 dwords `Foot+0x5E8..` to `+0x5E0..`, writes
  `Foot+0x638=-1` and `+0x68B=1`, then finalizes. Result 2 recurses with the
  force flag and returns. Results 4/5 clear `Foot+0x5E0` and locomotor
  `+0x58` to `-1`, null only the local endpoint, recurse with the force flag,
  and return; these direct recursive outcomes retain a callback destination
  into recursion. Results 1/7/other rejection perform those two clears and
  null only the local endpoint before common finalization, which clears or
  replaces live destination. Results 3 and 6 remain the already-existing
  native crush/block handling; crate integration must enter them with the
  computed result without adding destination writes.

Thus `Zero`/unlimbo/alive has no invented stop/recurse path: it becomes result
7, executes rejection state, and reaches common finalization with a null local
endpoint.

### Drive/Ship ProcessMovement second/final pickup

Drive `@0x004B46E6` and Ship `@0x006A3D15` first write
`Foot+0x63C=-1`, packed endpoint `Foot+0x558`, `Foot+0x68A=0`, and locomotor
`+0x5C=0`. They clear a non-null live destination/validity; a non-null local
endpoint is then installed, resolved, and dispatched.

- `One` and unlimbo: no alive test. Raw-apply the original local endpoint and
  return zero immediately; do not reset `+0x58/+0x5E0`, stop speed, or rewrite
  destination. Callback retarget survives and callback movement does not
  change the applied endpoint. Death still raw-applies.
- `Zero` or limbo: alive clears destination/validity; dead retains installed
  or callback state. Regardless of alive, write locomotor `+0x58=-1`,
  `Foot+0x5E0=-1`, call the speed vcall with `0.0`, and return zero.
- A null local endpoint skips dispatch and performs the same cleanup as the
  prior bullet.

### Hover movement helper

`0x00514F70/@0x005153E9` clears a prior non-null Hover destination only after
the native reservation-clear vcall. It writes `Foot+0x5E0=-1`, installs the
next-cell center and ground/bridge Z in Hover `+0x24..+0x2C`, resolves it, and
dispatches.

- `Zero` and unlimbo: if alive and status `+0x8D==0`, clear
  `Foot+0x5E0`, set the destination to Hover Null, call the stop vcall, zero
  Hover `+0x50/+0x48/+0x54/+0x4C`, call the speed vcall with `0.0`, and return
  status 7. Dead or nonzero `+0x8D` returns 7 while retaining candidate or
  callback destination and state.
- `One` or limbo: re-read alive, limbo, and `+0x8D` in that order; any failed
  gate returns 7 retaining live destination/state. Otherwise recompute the
  next coordinate from the collector's current post-callback XYZ, set
  `Foot+0x68B=1` on bridge-layer mismatch, and enter existing CanEnter result
  handling. Never reinstall the original candidate; callback retarget remains
  live.

### Jumpjet movement helper and state-4 descend

`0x005B17B0/@0x005B1894` adjusts a local requested Z for ground/deck, clears
the prior stored-destination reservation without clearing its fields, resolves
the adjusted local even for Null, and dispatches.

- `One` and unlimbo overwrites the stored destination with the original
  adjusted local, discarding callback retarget, with no alive check.
- `Zero` or limbo overwrites stored destination with Null, then dead returns
  zero. The common tail reservation-installs the live non-null destination and
  returns one; with Null it re-reads current post-callback XYZ, installs that,
  and returns zero. `One`/unlimbo death therefore still raw-writes/vcalls.

State-4 descend `0x0054C550/@0x0054C9F6` dispatches only after its successful
zero-altitude landing sequence has stopped motion, cleared the Jumpjet target,
updated bridge/fog/cell state, and resolved the current cell. It ignores
return, alive, and limbo. Immediately afterward it raw-writes collector
`+0x6AE=1`, `+0x427=0`, `+0x425=0`, locomotor `+0x90=0`, and for the native
special UnitType gate collector `+0x134=0`. Callback retarget survives the
pre-call target clear; callback kill/move/limbo suppresses nothing.

### Walk FindSubCellDest

`0x0075C240/@0x0075C56C` reservation-clears the stored destination (or current
XYZ), performs native subcell/deck placement, stores the exact returned
coordinate in Walk `+0x28..+0x30`, resolves it, and dispatches. Null input
stores Null and skips dispatch.

- Only `Zero` and unlimbo clears the live destination; dead then returns zero,
  while alive continues.
- `Zero`/limbo and every `One` preserve the live destination and do not test
  alive. A live non-null destination is reservation-installed and returns one;
  Null re-reads current post-callback XYZ, installs it, and returns zero.
  Callback retarget is discarded only by `Zero`/unlimbo; dead callers in all
  other cases may still receive the raw reservation vcall.

### Teleport arrival

Arrival `@0x0071972E` dispatches after bridge/unlimbo/state/sound and locomotor
arrival work. It ignores return, alive, and limbo, then raw-calls collector
stop, allocates the `0x1C8` arrival Anim, reads current post-callback collector
XYZ for the animation, raw-writes collector `+0x280=0`, and returns false.
Callback movement changes the Anim location, callback retarget survives, and
kill/limbo suppresses no postwrite.

These functions and callsites are the exact live xref set: `0x5153E9`,
`0x5B1894`, `0x4B405D`, `0x4B46E6`, `0x4B0D1B`, `0x6A3689`, `0x6A3D15`,
`0x6A03EB`, `0x71972E`, `0x75C56C`, `0x6A1401`, `0x4B1DBE`, and `0x54C9F6`.

## Persistence and deterministic identity

- Serialize all 256 slots verbatim, including ghosts, paused timers, aux, and
  duplicate coordinates.
- Serialize pickup latch, resolved scenario Train/Truck flags, object/cell Tag
  attachments, `trigger_difficulty_raw`, global Tag source order, and native
  TriggerInstance list order.
  Preserve every Tag field named in the ordered-runtime contract and every
  TriggerInstance field including `raising_house`, pending-delete, semantic
  timer start/duration, the satisfied mask, and enabled. Serialize each
  TriggerType Event node's shared `last_raising_owner` exactly once in
  TriggerType/Event source order; busy is normally false at save but is not
  discardable.
  Load must restore/swizzle TagType, TriggerType, next-instance, raising-House,
  object, cell, shared TriggerTypeRuntime, and shared Event identities without
  coalescing distinct Tags or cloning one TriggerType's Event state per
  instance. Preserve all raw multiplier bits.
- Fold all future-affecting crate state into `Simulation::state_hash` in stable
  slot/display/logic order.
- Preserve the native checksum boundary for trigger-owned objects: Tag checksum
  identity includes TagType, first TriggerInstance, and disabled/uninitialized;
  TriggerInstance identity includes TriggerType, next, pending-delete, enabled,
  remaining timer, and satisfied mask. Raising-House and Event owner memory are
  still serialized and included in the deterministic Rust world hash even
  where the native quick checksum omits them. Fold
  `ScenarioSession.trigger_difficulty_raw` into the Rust world hash and its
  retail Scenario quick-checksum projection. Normalize the semantically inert
  native `TriggerInstance+0x38` stack-residue word to zero and omit it from
  future-state identity.
- Do not add direct slot folding to `compute_retail_multiplayer_checksum`.
- Bump `SNAPSHOT_VERSION` exactly once for the completed crate slice and update
  all literal version fixtures/migration diagnostics.
- Preserve the existing nonzero-mode OverlayPack behavior: reject authored
  `Crate=yes` identities while independently applying OverlayData; never
  synthesize slots from authored overlays.

## Implementation sequence for the single builder

The assigned crate builder owns the mechanism end-to-end and commits coherent,
focused slices:

1. rules, type/map parsing, signed data, and parser fixtures;
2. persistent state, exact placement/ghost/timer/clear/regen, bootstrap
   replacement, save/hash;
3. synchronous pickup selection/remaps/removal/replacement;
4. eight effects, native multipliers/consumers, presentation;
5. source-ordered typed TriggerProgram, native load materialization, reentrant
   per-Tag runtime, migrated Events 27/28/36/37/47/60/61 and Actions
   22/28/29/40/48/53/54/56/57/67/68/69/112/137/138, action 108,
   Events 1/8/49/50, camera state, and CrateBeneath/CarriesCrate ingress;
6. all movement/teleport callsites and SQD-safe stable-ID continuations;
7. focused integration matrix and cleanup of stale assertions/comments.

Before every Cargo invocation, check `Get-Process cargo,rustc`; every test uses
`cargo test -p vera20k --lib <filter>`. The Phase-wide full
`cargo test -p vera20k --lib` remains reserved for the single final Phase 3
certification run, not this mechanism loop.

## Acceptance matrix

### Parser and installed data

- all 19 Powerups canonical keys parse independent of file order from the
  executable-image baseline; absent section and a loader-discarded empty-only
  section preserve all, while a present partial section gives every omitted
  row the exact `0,NONE,0` mixed fallback;
- Powerups fixtures distinguish an empty canonical row in a section kept live
  by an unknown/nonempty row, prove a later partial pass zeros omitted weights
  and replaces their animation through live `NONE` lookup while retaining
  water/data, and allocate literal animation `NONE` before the pass to prevent
  an incorrect hardcoded `-1`; a later animation allocation does not repair an
  earlier unknown lookup;
- Powerups `strtok` fixtures cover `,,,`, leading/consecutive/trailing commas,
  whitespace-only shifted fields, a shifted fifth token, and ignored sixth
  token; signed decimal `atoi` covers `0x10`, `$10`, `10h`, signed junk, no
  digits, and i32 overflow wrapping with no CrateRules hex reader reuse;
- Powerups animation fixtures cover `<none>`, bare `NONE`, empty, known,
  unknown, and case-insensitive first-match lookup; water covers exact yes/no
  and invalid/absent retention; data asserts exact bits for direct-binary64
  `1.2`, `50%`, `1x%`, `-0`, overflow, underflow, invalid/nan/inf, percent in
  junk, and `%` immediately inside/outside the 127-byte copy boundary;
- sequential typed Powerups fixtures apply base, LANGRULE, mode, and map
  passes and distinguish stacks whose flattened key/value projection is equal;
  installed fixtures lock the complete weights/animation/water/data arrays,
  retail total 110, eight positive-weight effects, and legacy `SOV07S`/
  `SOV08U` arrays; installed Mission/TMC/mode absence assertions protect the
  explicit no-op layer boundary;
- CrateRules constructor fixtures assert signed `1/255`, radius 640, regen bits
  `0x4024000000000000`, solo 2000, fixed `2/0/0`, null images/type, Heal
  sound `-1`, and FreeMCV false before any INI; section-absent and per-key-
  absent later layers preserve every current value/bit/identity;
- ReadRange fixtures assert `2.5->640`, retail `3->768`, `-0.5->-128`, exact
  `-1` and NaN retain prior, `-1%->-2`, `-100%` retains, f32 widening,
  nonfinite/out-of-i64 low-dword zero, and finite i64-to-i32 wrap with no
  clamp; CrateRegen fixtures distinguish constructor exact 10.0 from retail
  exact 3.0 and percent/widened-f32 inputs;
- signed minimum/maximum/solo fixtures include negative, hex, junk-zero, and
  inverted values, trailing decimal junk, hex-conversion failure retention,
  and signed wrap; bool/image/sound/UnitType/fixed-effect fixtures prove
  authored empty is omitted/retained, none/null, known resolution, unknown
  allocation or retain/Money fallback as appropriate, and all 19 fixed names.
  CrateRules layer fixtures prove constructor -> RULESMD -> LANGRULE -> mode ->
  map retention and that pre-reset MISSIONMD cannot win for RulesClass-owned
  CrateRules fields. Installed retail resolves exact `1/255`,
  radius 768, regen 3.0, solo 5000, HealBase/Money/Money,
  CRATE/CRATE/WCRATE, HealCrate, null UnitCrateType, and FreeMCV true;
- crate sound fixtures construct seven `-1` slots and resolve installed
  `CrateMoney/CrateReveal/CrateFirePower/CrateArmor/CrateSpeed/CrateFreeUnit/
  CratePromoted`; C4Warhead constructs null and resolves installed `Super`;
- all remaining sound, type, overlay, Basic flags, and tag/object columns parse;
- Event parser fixtures consume ParamType-0 triples and ParamType-2 quads
  without desynchronizing a following condition, materialize scalar/type name
  separately, and reproduce the active 3,540/78 census; Action fixtures retain
  all eight tokens and prove ParamType-2 Trigger targets plus ParamType-0
  variable scalars do not read the ParamType slot;
- 184-map fixture proves no authored CRATE/WCRATE, no ordinary Events 49/50,
  no ordinary TeamType tags, 13 action-108 calls, 58 ordinary CrateBeneath
  structures, and no active CarriesCrate scenario gate;
- trigger field-three nonzero-enabled parsing and Tag field-zero repeat parsing
  correctly activate xxmas/xarena; field seven never controls repetition;
- Campaign Easy/Medium/Hard and OfflineSkirmish raw-zero bootstrap snapshot the
  Scenario trigger difficulty independently of per-House AI difficulty;
  out-of-range raw values persist and take the constructor field-three-only
  fallthrough;
- parser fixtures preserve `[Tags]` source order, linked TriggerType order, and
  textual Event/Action chunks needed for reverse-Event/append-Action
  construction;
- Action parser materializes signed `+0x90` for ParamTypes 0/11, 5/9,
  registry-dependent 6/7/8, and zero-default others; invalid mandatory tails
  are rejected, while optional missing WaypointCode retains constructor zero;
- all installed Action-108 chunks prove ParamType zero, exact signed data, and
  the `xxmas`/`xarena` waypoint/cell/content table above.

### Slots, placement, and timer

- 256 exact fresh slots and serde round trip;
- one human plus seven AI requests one before min/max; negative/inverted signed
  rules preserve native branch order;
- full slots spend zero RNG;
- hard rejects spend X/Y only and retry; accepted visible/ghost spends timer;
- stock flat empty Water/WCRATE is visible with data `0xff`; Water with
  `Float=0` is an accepted ghost; Land with nonzero/zero Track is respectively
  visible/ghost; slope 4 remains eligible while slope 5 ghosts;
- active ground TerrainClass skips Unlimbo and ghosts, while ordinary Cell
  occupation follows the zero-only Mark gate; bridge/deck selects alternate
  occupation and bypasses zero underlying terrain speed; Water/common pointer
  aliasing proves Water/Float precedence; every post-precheck failure is timed
  and stops retry;
- origin water/land selects Float/Track snapping while the re-fetched snapped
  destination `LandType == 2` independently selects Water/Wood image; cross-
  surface snaps prove `CrateImg` is never selected;
- accepted visible and every accepted ghost emit DirtyScreenRect(force zero)
  then gated cell redraw before slot/timer writes; hard reject emits neither,
  no path emits radar dirty, and specific-data post-write emits no second
  invalidation;
- cell-redraw fixtures cover suppression, same-frame stamp, explored/forced,
  widened viewport, 798/799 queue boundary, success stamp/forced-byte clear,
  and tactical-flag set;
- existing overlay is hard rejection;
- duplicate ghost coordinates and cell-byte preservation;
- specific data 0, 18, 19, 20, and 0x114 for visible and ghost;
- timer goldens at draw 0, max, and an interior value;
- clear pause/wrap/start-minus-one behavior plus nonzero matched/ghost/mismatch
  removal; matched removal unions exact rectangles and dirties before writing
  only identity/data, while ghost/mismatch preserve the Cell and emit nothing;
- mode-zero removal accepts arbitrary OverlayType `Crate=yes`; both modes
  preserve all other Cell state, never CellRedraw/radar-dirty on removal, and
  never use the generic whole-OverlayCell clear;
- ascending regeneration including zero-duration same-pass cascade;
- exact scheduler position and pre-increment frame.

### Pickup, guards, and RNG

- fixed content skips selection; weighted inclusive boundaries;
- exact selection/replacement/timer/effect cursor order;
- checked raw-mode construction accepts 0/5 and rejects 3/4 before Simulation;
  supported modes never execute WOL counter mutation and no
  `game_mode_nonzero` approximation is accepted;
- event-49 kill returns Zero with crate/latch/RNG untouched;
- event-49 move, unlimbo, retarget, and limbo continuations;
- event-50 next-rung source-ordered scan, per-Tag ladder skip, continued global
  walk, and unconditional latch clear;
- passive-owner, no-overlay, non-crate prefixes return One;
- first matching slot removal, ghost removal, mismatch still replacement;
- strict thresholds 50/51, 100/101, 1500/1501, BaseUnit ownership, session
  Bases off/on, Water, Beach, and water-byte remaps;
- solo CRATE image collision and fixed 5000 without amount draw.

### Effects

- Money inclusive 2000/2900 endpoints, solo local-player-versus-picker credit
  target, wrapping credits, mutation/sound/anim order;
- Unit forced/random candidate draw order, BaseUnit gates, exact/nearby success,
  movement type/facing, creation failure, pre-remap 2000..2900 versus
  double-failure 0..900 Money fallback, return and anim suppression;
- HealBase live-count re-read, Logic order, owner match, zero receiver call,
  negative healing, sound before mutation, and mandatory
  `ResolvedRuleHandles.c4` dispatch; stock `[CombatDamage] C4Warhead=` resolves
  `Super`, while a synthetic key change plus alternate target WarheadType
  proves the receiver observes the changed identity and semantics at distance
  zero with flags `(0,1,1)` and no duplicate crate authority;
- Reveal sets remote/local `MapIsClear` first; remote and pre-Visionary local
  skip the exact map-work tail but still sound/animate;
- local mode-5 Reveal persists Visionary, performs the two Paranoid brackets,
  visits all 12,720 cells for `N=M=80` in anti-diagonal order, zeros both
  counters, preserves unrelated OR-field bits while applying `0x18/0x03`, then
  refreshes radar and requests redraw mode 2/cache-generation increment;
- mode-5 direct writes include all three native network-exclusion coordinates;
  offline Cooperative ID 3 and Unholy ID 4 remain raw mode 5, while raw LAN 3
  and WOL 4 fail launch construction before any Reveal runtime exists;
- radius 3D/Z and strict 767/768 boundary, live-capacity re-read, display order,
  enemy modification, and no Armor/Speed/Firepower alive filter;
- Armor/Speed/Firepower exact-one gate, raw double product, EVA latch gates,
  picker sound gates, and Firepower no-ROF/no-flag regression;
- Speed consumer exact stock AMCV 10, rookie/veteran/elite MTNK 17/20/20,
  stock crate rookie/veteran 20/24, raw multiplier-bit persistence, and
  staged-rounding discriminator `18.9 -> 18 -> 21` versus fused 22;
- Speed consumer parser/precision fixtures for raw Speed `-1/-2/99/100`,
  all three f32 House multiplier categories plus other-category one fallback,
  missing-layer raw-bit retention, `1.15 == 115% == 0x3F933333`, signed zero,
  subnormal/nonfinite exact loads, no clamp, invalid/overflowing FISTP low
  dword zero, stock VeteranSpeed/Speed-crate bits, FASTER inheritance,
  fraction zero/one/interior values, and no command floor;
- Infantry/Walk fixtures prove stock `Speed=4 -> 10`, upright 10, prone Crawls
  7, synthetic non-Crawls 15, crate-only 12/8, FASTER-only 12/8,
  crate-plus-FASTER 12->14->10, and fraction-half 5->4. Signed wrapper cases
  include `-17 -> -12/-25`, `i32::MAX -> 1431655765/-1073741826`, and
  `i32::MIN -> -1431655766/1073741824`; `WEEDGUY` missing Crawls stays true;
- Walk call-order fixtures prove fraction one is written immediately before
  one adjusted displacement query, stop exits write zero and do not query,
  terrain/health are not reapplied, and the ready/animation truth tables remain
  independent of the returned integer. Hover proves its ordinary one-query and
  aligned-zero startup two-query paths with the common Unit result sampled
  before the locomotor multiplier;
- locomotor-eligibility fixtures prove Speed-crated Jumpjet Infantry/Units and
  Teleport Units persist/hash the exact multiplier and emit the normal effect
  while displacement/timing stays bit-identical, yet real ApparentSpeed and
  moving-Unit archive-target projection observe the changed common helper;
  parachute descent is unchanged but landed Walk consumes the retained value;
  Aircraft Fly/Rocket cannot acquire it, a modded non-Aircraft Foot on those
  locomotors stores it without changing flight, and an Aircraft on Hover is
  still pickup-rejected; DropPod/Mech/Tunnel have no retail binding. An operand
  guard fails if any path other than the common helper begins reading the
  multiplier;
- Speed fraction setter proves exact bits for `0`, `-0`, `1`, below/above
  bounds, positive/negative subnormals, multiple NaN payloads, `+infinity`, and
  `-infinity`; only ordered strict interior preserves the input qword;
- target-producer fixtures cover f32 70% widening to
  `0x3FE6666660000000`, signed-level Road override, OnBridge +4, exact-ground
  slope with stock widened-f32 1.2, zero/unordered fallback before damaged 0.75,
  negative survival, inclusive ConditionYellow 50/100 versus 51/100, and the
  selector `<64` target versus `>=64` current branches;
- terrain-loader fixtures distinguish absent-section zero from present-row
  missing-key one, enforce its upper-only cap and f32 store, and keep Rules
  slope plus explicit ObjectType acceleration/deceleration overrides at the
  `%f` binary32-to-binary64 `ReadDouble` boundary while missing keys retain
  exact constructor/default qwords;
- installed-type fixtures assert the exact DNOA/DNOB/V3/SQD, DRON/CAOS, and
  SMIN override qwords above rather than projecting them through SimFixed;
- ramp fixtures prove stock `0.03` acceleration and overshoot cap, strict
  distance 499/500 split, destination and alternate promoted-f32 floors/rate,
  downward target floor, crush target writeback/exact 0.2, nonaccelerated
  direct target, passive/forced no-write gates, equality no owner setter, and
  NaN ordered/unordered choices at every arm;
- the caller-specific z/y/x distance fixture proves structural destination
  `(100,0,416) -> 427`, while Drive's virtual speed source and Ship's direct
  type-speed source produce bit-identical active-retail Unit results;
- Drive/Ship trace proves owner write, Unit linked-member virtual-setter writes
  in exact native link/self-link order, then current-speed query; accelerated
  equality still propagates, while nonaccelerated/passive/forced paths do not;
- scheduling fixtures prove Track(0)-Movement-Track(1),
  Movement-before-Track(0), retry `0.03 -> 0.06` while masking only fresh
  budget, sinking target-only clear, ordered-positive idle clear, and terminal
  abort current-zero with target retained;
- a command-admission negative fixture snapshots both raw qwords, installs an
  ordinary Drive path for accelerating and nonaccelerating types, and proves
  neither changes until the native ProcessMovement/Track rung owns the first
  update;
- writer fixtures prove ForceTrack changes target but not current, factory
  half, bunker selectors one and no invented refinery release, arrival queue
  gate, Move-reissue/NavCom gate, visible Wave zero, Chrono one, and Tube
  category branches; starting from raw `0.375`, Unit success and ordinary
  Infantry success become one, Infantry `+0x5A4`-equal success preserves the
  exact qword, and blocked Unit/Infantry become zero; target never changes and
  no same-tick generic writer follows. Raw snapshot/hash distinguishes
  interior qwords below I16F16 resolution;
- crate-continuation fixtures distinguish the saved pre-dispatch current qword
  from target, crate multiplier, integer speed, and callback-mutated current;
  One/unlimbo restores saved bits before the path shift, while Zero/limbo
  retains callback current and Speed-crate multiplier always persists;
- same-Process retry repeats fraction/query state but contributes zero fresh
  speed; uncrated movement and ready/animation consumers retain their exact
  cached-i32 behavior;
- Veteran Rookie/Veteran/Elite, positive-fractional ceil iteration, and
  non-trainable cases, no EVA;
- common animation allocation size, coordinate Z+200, constructor flags,
  ordering, none/missing allocation.

### Ingress and callers

- same Tag ID on two attachments shares one runtime, while two Tag IDs pointing
  to one TriggerType retain independent instance enable/pending/timer/mask/
  raising-House state but reference one shared TriggerType/Event runtime; a
  first-Tag `TEvent+0x54` owner write is observed by the later Tag and survives
  save/load/hash as one shared cell;
- deliberately nonlexical IDs prove Tag master first-materialization order
  CellTags -> Units -> Aircraft -> Infantry -> Structures -> mask 4/0x10/8
  postpass; Trigger master groups are chain head-to-tail, Tag-local instances
  tail-to-head, and polling registration is independent `[Tags]` source order;
- same Tag on cell/unit/building reuses one runtime with signed attachment
  count three and last successful CellTag coordinate; an unattached category-
  ineligible Tag has no runtime; Team construction creates a no-reuse group;
- Events are reverse textual chunk order and Actions remain textual chunk
  order; constructor timer reset proves Event 13's wrapping `scalar*15`, Event
  51's signed-half plus ranged-draw wrapping formula, aliased-mask clears, and
  every Event-51 RNG spend in exact materialization order even when the final
  field-three/difficulty gate disables the instance or a later timer overwrites
  it; the inert native `+0x38` residue remains normalized and hash-neutral;
- a false early Event does not short-circuit later Event side effects; latch
  aliases at index 32, Event-1 persistence/nonlatching, persistent-event latch,
  and last-non-null raising-owner selection match native order;
- xxmas first Event-13 delivery satisfies Event 8, produces eleven accepted
  calls in textual `CA..CK` Action order, follows repeat-zero logical
  cleanup, and never repeats after late physical finalization;
- xarena tagged CATECH01 event-1 producer uses the engineer as raising object,
  accepts exact owner filter `-1`, writes the engineer owner, runs before the
  capture mutations, and places its fixed type exactly once;
- action108 returns false full/invalid and true visible/ghost without gates;
  `[false,true]` and `[true,false]` both execute in textual order and Spring OR
  is true; `[false,false]` returns false from Spring but still retires a
  satisfied repeat-zero Tag;
- Action 108 distinguishes full data dword `20` from `276`, exercises optional
  WaypointCode defaults for ParamTypes 0/5/9/11, and rejects unresolved
  ParamTypes 6/7/8 at runtime compilation rather than guessing a registry index;
- Event-49 Actions that kill the collector preserve crate/latch/RNG; Event-50
  visits Tags in source order, a firing Tag skips only its own later ladder,
  and the latch clears;
- Event 27/28/36/37 ranges, Event47 signed frame/15, and case-sensitive reverse-
  vector Event60/61 counting run under per-instance ownership; changed versus
  unchanged variable writes prove same-pass later-Tag visibility and timer/RNG
  rearm only on change;
- Action22 bypasses conditions/cleanup and synchronously scans every matching
  construction-ordered instance; Action53 ignores field three/current enabled/
  pending, leaves difficulty-ineligible matches bit-identical, and writes then
  rearms every admitted match in registry/reverse-Event RNG order; Action54
  only disables all matches, including pending-state boundaries;
  direct regression fixtures cover Actions29/54/57 as well as existing
  28/40/53/56/137/138 paths;
- Action53 null/empty/no-match/all-ineligible and Action54 null/no-match return
  true; a field-three-zero or pending instance can be enabled/rearmed by 53,
  while 54 spends no RNG and changes no timer or mask state;
- an installed-corpus fixture locks all 3,186 Trigger rows and their thirteen
  field3/Easy/Medium/Hard combinations: initially enabled totals are
  Easy=1,293, Medium=1,318, Hard=1,330; the 1,807 Action53 occurrences admit
  1,668/1,701/1,710 targets and reset Event51 RNG 57/60/60 times respectively,
  including the 98 occurrences whose target has field three zero; Action54
  count is 1,502, Event13 count 1,693, and Event51 count 73;
- repeat-one signed counts 2->1 detach then fire/finalize, zero stays inert;
  repeat-two bypasses conditions and stays registered; polling `[A,B,C]` with
  A retiring processes C now and defers compacted B to next tick; physical
  Trigger/Tag free occurs only on the late finalizer rung;
- Actions48/112 cover exact f32 speed bits/completion frames, flat/slope/bridge
  target Z, valid missing `(0,0)`, all four ordered stack combinations,
  follow-camera ordering, one step per binary frame, scenario/replay gates,
  f32-progress/f64-truncating lerp, mid-glide save/load, and no same-frame
  double step; differing camera state leaves Simulation hash/checksum equal;
- Actions67/68/69 target a pinned local owner distinct from Trigger/raising
  Houses, ignore operands, and return true; fixtures cover accepted/repeated/
  pending transitions, all ordered pairs, start `-1` repair, trigger-before-
  House same-frame expiry, localized notice counts, and no direct result screen;
- the two active Action68 campaign rows execute against local player; active
  Action67/69 counts remain zero; the sole Action22 edge executes synchronously
  and typed compilation rejects synthetic self/cycles; existing trigger-runtime
  Action40, camera, alternate-base, variable, save/load, hash, and master-frame
  tests are migrated to real Tag fixtures rather than deleted;
- CrateBeneath stock CAMIAM07 money and CAWASH17 random destruction at the
  northwest anchor, with Crates off and mode zero/nonzero;
- CrateBeneath render-coordinate fixtures: `Location=256*A+128 -> A`, and
  locations `0,-1,-128,-129 -> cells 0,0,-1,-1`; 2x2/odd foundations do not
  shift the requested cell;
- CrateBeneath foundation-data failure still reaches its tail; ordinary fatal
  immediate/future-deferred paths place at most once after UnInit, while
  result-5/nonlethal/construction/Unlimbo/capture/direct-despawn/voluntary-sale
  paths place none;
- CrateBeneath accepted/full/pre-existing-overlay/out-of-playfield results are
  attempted once, ignore the boolean, and never retry;
- CarriesCrate Train/Truck/default/global-gate/double-snap matrix;
- every one of the 13 callsites is observed once at its native committed cell;
- per-site `Zero`/`One` x alive/dead x limbo/unlimbo matrices assert the exact
  clear, retain, original-coordinate apply, head-to/speed, path-window, and
  return/status writes specified above;
- each return-sensitive adapter runs all eight combinations
  `{Zero,One} x {alive,dead} x {unlimbo,limbo}`, with retarget on both Zero
  cases and independent move/retarget scripts on One/unlimbo;
- callback move and retarget fixtures prove which adapters use post-callback
  XYZ, which raw-apply the original endpoint, and which preserve/discard the
  retarget;
- dead/unlimbo `One` fixtures prove the native no-alive-check raw writes for
  ForceTrack, ProcessDriveTrack, ProcessMovement final, and Jumpjet;
- ignored-return Jumpjet-descend and Teleport fixtures prove every postwrite,
  including Teleport's post-callback animation coordinate;
- SQD event49 death, Explosion death, Unit success, moved/unlimboed callback,
  and reciprocal link continuation.

### Persistence and regression

- snapshot preserves exact admitted native mode, local Visionary, ghosts, all
  slot words, pickup latch, content byte,
  object/cell Tag attachments, source/list order, signed attachment counts,
  Tag flags/state, master/polling registries and pending-finalization order,
  TriggerInstance links/owner/pending/timers/mask/enabled, shared-per-
  TriggerType Event owner memory, and multiplier bits; a load round trip does
  not coalesce distinct Tag
  IDs and snapshot version literals agree;
- snapshot/hash preserves independent House pending/win/loss/start/duration,
  including pending-with-no-terminal and mid-Savour states; `last_announcement`
  is absent, and the active network quick-checksum oracle omits these result
  fields and TimerClass remaining time;
- app/presentation save adjunct preserves Tactical camera glide/committed/
  requested/progress/last-frame state while world hash and retail checksum omit
  it;
- broad hash changes for each new future-affecting field;
- retail checksum unchanged by slot-only differences;
- no ordinary Phase 3 House/WOL statistic is introduced; checked launch
  construction accepts raw 0/5, rejects raw 3/4 before Simulation, and never
  confuses MPModes roster IDs 3/4 with raw network modes;
- existing OverlayPack identity filtering/data pass stays green;
- existing Armor damage and uncrated movement/combat remain bit-identical;
- no duplicate pickup, deferred crate event, or extra scenario RNG consumer.

## Evidence-backed exclusions

These do not keep the ordinary-retail row open once their absence and boundary
tests pass:

- weight-zero handlers Cloak, Explosion, Napalm, Darkness, ICBM, Gas,
  Tiberium, and inert/TS slots are not randomly selected and no ordinary
  action 108 supplies them;
- authored crate identities do not occur in the 184 installed maps, and native
  rejects them in nonzero mode;
- ordinary maps define no Event-49/50 conditions and no TeamType tag producers;
- every installed TruckCrate/TrainCrate value is false;
- native mode-4 crate counters are WOL postgame protocol state; Phase 3's
  ordinary offline Skirmish mode is exact 5, and Phase 13 owns WOL/session
  protocol support; raw LAN/WOL 3/4 and all other unsupported raw modes fail
  the launch boundary before Simulation, while MPModes roster IDs are not
  interpreted as raw modes;
- native network Reveal's exact three-cell predicate/formulas are documented,
  but no network Reveal runtime is reachable under that construction boundary;
- native failed-Mark orphan Overlay object registration/UniqueID is outside
  crate gameplay state and absent from the quick checksum;
- allocator OOM graph identity, corrupt pointers, memory corruption, and
  malformed zero-total Powerups are invalid-domain behavior.

These exclusions do not authorize invented behavior. Canonical indices,
parsers, no-op/latch boundaries, and synthetic exact-gate tests remain.

## Critic handoff

Every design critic receives the user requirement, the native authority report,
this complete file/diff, current Rust paths, and literal validation output.
After any finding, fix the largest one first and submit the full revised design
to a new critic who rechecks prior fixes. Implementation begins only on an
explicit zero-finding verdict.
