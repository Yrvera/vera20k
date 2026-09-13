# Aircraft receivers during live bridge repair

## Evidence and scope

Original active-retail `gamemd.exe`, x86 little-endian, image base `0x400000`,
SHA-256 `1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Instructions, virtual slots, initialization and the retail inputs below were
inspected on 2026-09-13. A fresh independent read-only critic agreed with the
bounded semantic reduction and composed stock witness. This is not a native
whole-flight recording, a Rust test result, or acceptance of the whole repair host.

The production consumer is
[bridge_repair_admission.rs](../../../../src/sim/world/bridge_repair_admission.rs).
Its Aircraft reduction must preserve the conditions below. The ordinary repair
controller may encounter a live Aircraft ground-list receiver; rejecting that
category after partial terrain writes loses required continuation.

## Why an ordinary carrier child reaches this receiver

Winning `expandmd01.mix/rulesmd.ini` SHA-256:
`3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`.

- CARRIER is buildable, naval, Float/Water and spawns HORNET.
- HORNET is Spawned, Landable and Fly. Carryall is absent, hence false.
  Its apparent `Selectable=no` is a comment; the default is true.
- HornetBomb has no Camera assignment; WeaponType Camera defaults false.

The Aircraft layer call is `41ADC0 -> Fly 4CFCF0`: height above zero selects
layer 4; height at or below zero selects layer 2. Foot Mark `4D3780` admits
layer 2 before `5683C0 -> 47E8A0` inserts the ground list. Carryall-false
Aircraft floor query `41B6A0` returns zero. Fly descent `4CEC7F..4CED0C`
lands at that floor; its wrapper executes REMOVE at `4CD324`, descent at
`4CD341`, then PUT at `4CD4F3`.

The Spawned landing receiver `4DDC60` accepts a parent with SpawnManager
`+2D0` at `4DDD2F..4DDD41`. Parent SpawnManager update `6B7230` has a
ten-frame timer; the child enters Limbo at `6B77FA/6B7803` on a later parent
update. Child landing does not synchronously invoke that parent update.
A parent created before its child, followed by a later-created engineer, can
therefore execute parent, landing child, repairing engineer in one frame.
Choose a healthy child inside the map and outside the environmental-damage
cadence (`frame mod 64 == 63`). This is a composed instruction/order witness.

The persistent Aircraft `+3D4` suppression flag is not the Spawned type flag.
Constructor `6F2B4B/6F2F55` clears it. Ordinary SpawnManager allocation
`6B6D4D/6B78E4` uses AircraftType slot `7E28F4 -> 41CB20`, then
`413D20 -> 4D31E0 -> 6F2B40`. The manager sets SpawnOwner `+2D4`;
it does not thereby set `+3D4`. Successful unlimbo `414310` sets suppression
only for nonselectable, nonlandable, or primary Camera aircraft. Fresh ordinary
stock HORNET thus retains zero. Trigger, superweapon, airstrike and later
mutated histories are separate; this statement does not cover them.

## Concrete stock geometry

`c1a01md.map` SHA-256
`51aa58f2de7e03217660bf090ae8adcab4dc17bcfe1675493db9c95c9be7e31f`
is a multiplayer map. Its ordinary bridge cell (99,67) has overlay 83
(destroyed identity 101), tile 314, subtile 3 and level 2. The approach (98,67)
has the same level; huts are at (103,64) and (97,71).

TEMPERATE WaterSet 21 starts at tile 314. `Water01.tem`, from
`ra2.mix -> isotemp.mix`, has SHA-256
`7995ceb43404d81096220e2d7b3474a527a3717df8e8db4e7068e5e39b04e083`.
All four entries have TMP terrain byte 9, height 0 and ramp 0.
Native `544BE0` maps terrain 9 through `8288E4` to land 2 (Water).
Theater startup `5349F2/5349FD -> 534BD0 -> 5B3C20` mounts ISOTEMP.MIX;
a generic asset-browser catalog-only warning does not model that dynamic mount.

Overlay identity 101 is LOBRDG28 (numeric INI key 104). It is not Tiberium and
does not override tile land. Recalc `47D87A..47D8AA` therefore retains Water.
Its cliff checks at `47DB59..47DD34` inspect (99,66), (98,67), (101,69),
(100,68), (98,68), (100,66). All are level 2, below the required current+4
threshold. The Water/Float speed cost is nonzero, and `4D9C60` admits the
equal-level nonstructural approach. No structural Tube premise is used.

## Exact permitted reduction

The first ground-list pass at `487A3B` invokes the Aircraft admission receiver
`4196B0`, but kind 2 forces the damage continuation regardless of its answer.
The second neighboring-head pass uses the locomotor `+A0` predicate before
admission at `487B9F/487BA5`. Active Fly slot `7E8A94 -> 4B6630` is
`32 C0 C2 10 00 90` (clear AL, return popping 16, padding); Fly cannot enter
that neighboring-head admission branch.

For authoritative TeamNone, the remaining local-viewer gate can call
`586360`. Its real-cell path performs reads; missing lookups at `5863ED`
or `586470` mutate shared dummy coordinates and cannot be discarded.
Odd projection quotients can additionally call `481810(3)` if the first
alternate flag is clear. Every potentially selected lookup must resolve a
real identity when using this reduction, including branches selected by a
viewer state the simulation does not otherwise need.

Ground-height `47B3A0` initializes deterministic derived slope/height caches
(`89E770/89E720/89E740`), not per-cell state. The quotient proof must still use
the established native ground calculation. Its divisor `89E7C0` equals
projection divisor `ABDE88`: basis routines `47B150/561710` use identical
inputs and calls; angle constants at `7E4EB0/7ED3C8` have identical bytes
`39 9D 52 A2 46 DF 91 3F`; final routines `47B220/5617E0` use the same
subtraction, table tangent, multiplication and integer conversion.

For the flat level-2 witness, ground height is twice that shared divisor.
The projection quotient is 2 and its real alternate cell is (98,66):
tile 91, subtile 2, level 2. This argument does not assume a hand-chosen
numerical divisor. It establishes a concrete stock case within the reduction.

## Team query effects

Nonnull Team membership does not by itself require broader Team AI in this
receiver. `4196B0` calls `6EC300` through Aircraft `+5D4`. That query reaches
the waypoint lookup only when Team byte `+7F` is nonzero, the current Script
cursor is unsigned-less-than its action count (`6915D0`), and the current
action is 3. `691500` copies the action/argument into caller-local storage;
`68BCC0` copies the Scenario waypoint at `+632 + argument*4` into that storage.
Neither advances the Script or writes gameplay state.

`578460(waypoint, true)` performs a fixed Map lookup. A missing or out-of-range
slot writes the shared dummy coordinate at `ABDC74` (`578498`). Its real-cell
path only reads. The subsequent `578540(selectedCell, true)` also only reads
and performs no lookup. Early admission failure still cannot prevent the
first repair pass's kind-2 damage continuation.

The reduction can therefore also cover a Team with an authoritative existing
Script whose raw cursor is out of range or whose current action is not 3.
Those conditions exclude the waypoint lookup regardless of unrepresented
Team `+7F`. For action 3, an authoritative real waypoint lookup would likewise
prove that branch has no gameplay writes. Missing Script information is unknown;
completion, refusal and pending-advance flags cannot replace the native cursor
test. Do not assume a missing waypoint is `(0,0)` or assume `+7F` is false.

This instruction-level extension was independently reviewed. It does not prove
a stock landed-Team aircraft repair scene, supply authoritative waypoint inputs,
or certify a Rust implementation. Missing/dummy projected identities, unresolved
waypoint effects and other locomotor histories remain outside the reduction.
Unresolved reachable cases keep the repair mechanism open. Full Rust integration,
regression checks and final independent source review are separate requirements.

## Nonzero session mode excludes the downstream shroud lookup

The original `419764` reads the session mode at `A8B238`. `419769` tests that
value and `41976B` jumps to the return-0 path at `4197AA` when it is nonzero.
This skips both the selected Cell's `+48` coordinate query and the shroud query
`586360` at `419793`. The subsequent local-owner `+41A` and suppression `+3D4`
tests are reached only in mode zero; they are not needed for this mode bound.

Therefore an authoritative nonzero session mode discharges the downstream
projected-cell/dummy requirements described above. It does not discharge the
earlier Team query at `4196C6`, including its possible waypoint lookup, or
establish the other locomotor histories. The repair caller's kind-2 damage
continuation remains independent of this predicate's result.

Rust already retains the native zero/nonzero classification in
`ScenarioDescriptor::game_mode_nonzero`, copies it into `ScenarioSession`, and
includes it in session persistence and identity hashing. Both current skirmish
loading paths supply true. The receiver can use this existing authority; no
viewer-local flag or additional session state is required. The native gate and
state ownership were independently reviewed. This is a bounded effects proof,
not acceptance of the implementing Rust change or the complete repair mechanism.

## The same reduction covers four explicit active locomotor kinds

The Fly-only bound above can be extended to active Rocket, Jumpjet, and Teleport
interfaces. Their `+A0` slots at `7F0BBC`, `7ECE08`, and `7F50A0`, together with
Fly's `7E8A94`, all resolve to `4B6630`: `XOR AL,AL; RET 0x10`. This leaf does
not inspect the Foot, coordinates, or movement state and has no gameplay writes.
Rocket's constructor installs its interface table `7F0B1C` at `661F1F`.

The first repair pass still invokes Aircraft `+1AC` at `487A3B`. The kind check
at `487A70..78` forces damage at `487A9E` independently of that predicate's
answer. The neighbor pass tests
`+A0` at `487B88` and its false branch skips admission at `487BD6`. Aircraft
`4196B0` itself does not inspect the locomotor. The same effects proof therefore
applies to these four explicit kinds, retaining every Team, mode, and projected
lookup requirement already described. A missing locomotor or another kind is
not established by this extension.

The committed `tools/spatial_oracle/locomotor_at_coord.py` reads all four original
slots and executes the unchanged leaf with a null Foot and two coordinate probes
per kind; all eight native rows return false. The two-instruction body establishes
the input-independent result; the samples also check the harness invocation.
Winning RULESMD declares V3ROCKET, DMISL, and CMISL with the Rocket locomotor,
`Landable=yes`, and `Spawned=yes`. These declarations do not prove a complete
stock rocket arrival or repair scene.

This caller/leaf extension was independently reviewed. Do not infer its kind
set from a generic absent coordinate query, which can also represent unsupported
or dormant TS classes. Rust implementation and production regression acceptance
remain separate requirements.
