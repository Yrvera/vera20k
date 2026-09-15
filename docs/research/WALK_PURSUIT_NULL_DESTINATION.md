# Walk pursuit and null-destination lifetime

Original active-retail `gamemd.exe`, x86 little-endian, image base `0x400000`,
SHA-256 `1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
The caller, virtual-table bindings and coordinate leaves below were inspected
and independently reviewed during the Phase 3 bridge integration. They establish
bounded native semantics; they do not certify whole Walk, Attack or bridge parity.

The production owner is [walk_host.rs](../../src/sim/movement/walk_host.rs).
The executable comparison is [walk_percell_stop.py](../../tools/spatial_oracle/walk_percell_stop.py),
with [results](../../tools/spatial_oracle/walk_percell_stop.json) and
[coverage and substitutions](../../tools/spatial_oracle/walk_percell_stop.meta.json).
Weapon selection and the final range boolean are supplied seams. The original
coordinate conversion, range-source wrapper, mission/queue gates and null setter
execute. Read the metadata before extending a claim to another caller or state.

## Why this is a spatial integration prerequisite

The new Walk owner retains an immediate step head and raw cell reservation while
the body travels to its subcell destination. Removing only the Rust movement
adapter prevents subsequent movement enumeration from retiring that head. A new
order can then inherit stale head and occupancy state. Both explicit attack orders
and pursuit completion can stop navigation, at different points in the native loop.
Their shared physical lifetime must survive integration of bridge movement.

This dependency does not establish complete Attack mission, UI command or
locomotion parity. In particular, completion-stop evidence does not establish the
first path-search/queue-publication timing or justify unrelated replay changes.

## Explicit synchronized Attack assigns a null destination

Foot object-click action 5 reaches `4D769F`; after the owner gate it emits mission
1, target, null destination and null fourth argument at `4D76BA..4D76C1` through
slot `+378`. Cell-click action 5 reaches `4D80B1`; its eligible overlay-object and
Cell alternatives emit the same null-destination shape at `4D80CD..4D80FF`.
Actual Infantry slot `7EB3D0` binds `+378` to `6FFBE0`.

In its synchronized branch (`637AA0` true), `6FFBE0` serializes fourth argument,
destination and target through `6E6AB0`, constructs Event 4 with `4C6860`, and
queues it through `637DD0`. Account for the successive eight-byte stack pushes
when interpreting its parameter loads. A null reference serializes as ID 0/type 0;
the constructor places the destination token at Event `+13`.

Execution assigns the target through `+3C8` at `4C7467`, then resolves Event
`+13` through `6E6E20` and calls `+480(NULL, true)` at `4C747C`.
The preceding `4DA1C0` clears a separate vector at Foot `+5AC`; it is not evidence
for clearing NavQueue entries `+58C` or count `+598`.

This trace covers ordinary non-remapped synchronized Attack. The input chord
which remaps mission 1 to 29 and complete command admission remain separate.

## Completion range check has a different caller and query order

Walk `75BE3C` dispatches Infantry PerCell `519630`. After the step-head/raw
retirement work, the Infantry tail at `51A9EB..51A9FC` reaches Foot PerCell
`4D85D0` mode 2 only while the actor's alive byte `+90` is true. The interior block
`4D882F..4D8978` first selects the weapon against the original attack target.
Its target must be non-null and have the Foot bit (`+14 & 4`).

For actor type `+5E4 == false`, target `+4F0` supplies current XYZ; actor `+164`
then checks range using that coordinate and the already selected weapon index.
Only after this query does the caller check effective missions 21, 11, 1 or 15,
the range result, and empty NavQueue (`+598 == 0`). When admitted it calls
`+480(NULL, true)` at `4D8968`, then writes one DWORD `-1` at Foot `+5E0`.
The outer DWORD write also occurs when the Infantry setter refuses the request.
Moving mission or queue checks before the range query loses observable Map/Dummy
lookup effects.

The live path head at `+5E0`, its suffix/reference state, and the ordered target
NavQueue are distinct. In `4D5A4D..4D5A9F`, positive count and a false stack
argument admit consumption by setting the first destination, decrementing `+598`,
and shifting the entries addressed by `+58C`. The true-argument branch at
`4D5A62` skips those operations.
Neither the single path-head write nor the command's `+5AC` operation proves
blanket queue exhaustion.

## Target coordinate and range-source bindings

Constructor-bound Infantry table `7EB058`, Unit table `7F5C70`, and Aircraft
table `7E22A4` bind `+4F0` to `4D9FF0`, `+A4` to `41BDD0`, and `+48` to
`5F65A0`. The chain copies current Object XYZ at `+9C/+A0/+A4` verbatim.
It does not substitute the target's step head, destination, cell center or a new
terrain-height sample. These bindings do not establish Building or Cell behavior.

Actual Infantry `+164` at `7EB1BC` is `6F7970`: it converts the coordinate to a
Cell through `565730`, then dispatches actual `+3A8` (`7EB400 -> 6F77B0`).
The wrapper obtains shooter XYZ and the same indexed weapon; CellRangefinding
can recenter the shooter through its Cell, including bridge elevation. The
high-flying branch subsequently replaces shooter Z with the target Cell's Z.
It calls `6F7220` with the converted Cell and selected weapon.

Thus the Rust caller must preserve original-target weapon selection while using
the converted Cell for range. Reselecting a weapon against the Cell, checking
object-target range instead, or using MinimumRange pursuit hold changes this
caller contract. Signed coordinate division and shared Dummy identity matter.

## Ordinary type gate and stop lifetime

Actor type `+5E4` is OpenTopped: constructor `710FE4` initializes it false and
`7143B6..7143CA` reads the `OpenTopped` key. Actual Infantry `+84 -> 6F3270`
forwards `+88 -> 51FAF0`, returning the actor's own type at `+6C0`.
This does not read the containing transport's type.

The surveyed winning rules contain 65 base Infantry types with no OpenTopped
assignment; the only base-rules true assignment is BFRT. Nine surveyed mode files
and 184 retail maps contain no OpenTopped override (one map adds DNOAA).
This bounds the ordinary branch to those inputs, not every language override or
runtime type mutation. The true branch's strict XYZ-distance comparison remains
distinct.

Actual Infantry `51AA40` can refuse for human-controlled Doing 27..30. Accepted
null requests reach Foot `4D94B0` and Walk `75ADA0`. Walk clears its destination
at full-object `+1C`, while retaining a nonempty immediate head at `+28` and
moving flags. Only the no-head branch clears full-object `+34` and `+36` and
calls the stopped callback (`521B40` for Infantry). Continuing the paid step
after an accepted stop is required; clearing the physical head immediately is
not equivalent.

The corpus declares its supplied object state, head-retirement seam and callback
limits. It does not prove complete reciprocal/radio/bunker handling, the
`Infantry+6E4=true` stopped action, or whole WalkProcess timing. These limitations
must remain visible in production review.
