# Ground Movement Lifecycle and Effect Ownership - Ghidra Research Report

**Address(es):** `0x004B0500`, `0x004B0F20`, `0x00739EC0`, `0x007416A0`, `0x00481670`, `0x00743A50`, `0x004D3710`, `0x0070D990`, `0x0070D1D0`, `0x0070CC90`, `0x0070CCC0`, `0x0070CCF0`, `0x0065FA70`, `0x005F4EC0`, `0x005F65F0`, `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x005F4D30`, `0x0055AFB0`, `0x0055BAE0`, `0x00725C70`, `0x004DF0D0`, `0x004D94B0`, `0x004D82B0`, `0x00452540`, `0x0044E440`
**Investigation Mode:** coverage-map
**Claimed Scope:** an owner-or-explicit-blocker inventory for every gameplay-bearing mutation currently performed inside or around the clean-Rust ground movement bulk phase: NavCom refresh, Tube/forced work, blocker/cache state, accepted-cell occupancy, per-cell contact, scatter/crush and both stock Infantry crush branches, sound creation, Reveal and Techno BREAK where movement/lifecycle causes them, UnInit/conceal/live-Logic removal, pending physical deletion, track completion, arrival/finalization, gate request/progression, war-factory contact breakup, wall crush, and the Unit/Infantry Tube completion/blocked-exit speed and post-leaf effect suffix
**Non-Scope:** exhaustive behavior of every `Detach_From_All_Lists` listener; exhaustive bodies of every virtual dispatched by the Tube `+0x4A0` effect callback; exhaustive pathfinder/zone cache internals beyond proved `CellClass` recalculation; non-ground death families; save/load lifecycle reconstruction; executable retail capture and mixer presentation timing; production Rust implementation
**Confidence:** High for each verified owner/order slice and the current-clean-HEAD disparity map; Medium or explicitly deferred for the receiver/listener/cache/arrival/overlay dependencies named below
**Active in YR:** Yes

## 1. Overview

Checkpoint D is **PASS for the bounded owner-or-explicit-blocker coverage map**
and **NO-GO for production activation**. It does not claim exhaustive closure of
the gameplay dependencies that are explicitly blocked below.

The load-bearing result is simple: none of the scoped movement effects is a
native global movement tail. Native Drive refresh, accepted-cell marking,
per-cell contact, wall damage, scatter/crush, sound creation, track completion,
and arrival work belong to the current object. Gate progression belongs to the
gate's own ordered object turn. `ObjectClass::UnInit` removes gameplay-bearing
membership synchronously; only physical destruction/free is placed in the one
late pending-delete drain.

Current clean Rust now has substantial ordered lifecycle infrastructure, but
movement still reaches it through a snapshot bulk pass and a post-movement
`LifecycleRequest::Uninit` drain. That changes which state later movers see in
the same native pass. It also changes encounter order, sound order, victim
teardown timing, gate-vs-mover order, and whether a shifted live-vector successor
runs at all. These are mechanism DRIFT, not implementation details.

### 1.1 Verdict at a glance

| Question | Verdict |
|---|---|
| Accepted-cell remove/write/add/PerCell owner | verified: current Drive object |
| Unit wall/contact/crush ordering | verified: synchronous Unit per-cell chain |
| Scatter population and Scenario RNG owner | verified; clean Rust differs |
| Crush victim lifecycle and sound order | verified; clean Rust differs materially |
| Reveal coordinate/Mark/registration order | verified; current Rust bounded order matches, admission/effects do not |
| Techno Limbo BREAK before Conceal | verified; current Rust bounded core matches, receiver coverage does not |
| Conceal, occupancy, and live-vector removal | verified: synchronous before the next object |
| Physical delete/free | verified: one same-`Main_Tick` late drain for common UnInit objects |
| Dynamic NavCom refresh | verified: Unit-following-Infantry current Drive branch, not a pre-mover sweep |
| Track completion/finalization | verified: current Drive invocation |
| Queued/empty arrival | verified: that object's next eligible no-active-track Drive invocation |
| Gate request/progression | request is mover-time; progression is the gate object's own turn |
| War-factory contact breakup | verified: current Unit per-cell call |
| Wall crush | verified: current Unit per-cell call, before generic crush |
| Unit/Infantry Tube call scheduling | verified: branch-specific scatter/completion/lifecycle calls still reach ungated `+0x4A0` |
| Unit/Infantry Tube common speed state | verified: exact shared clamp and blocked-zero/Unit-one/Infantry-one-or-preserve order |
| Tube post-leaf effects | bounded direct `+0x4A0` sequencing and small tracker/dirty bodies verified; full `+0x324` visibility and downstream tracker/pixel equivalence remain blockers |
| Native global movement-effect tail | none proved |
| Exact every-listener detach/cache subsystem | touched, not exhausted; explicit external blocker |
| Executable retail oracle | deferred to Checkpoint E |
| Production activation | **NO-GO** |

### 1.2 Evidence boundary

All binary checks targeted the active retail `gamemd.exe` identified by the
read-only `get_current_program_info(program="gamemd.exe")` call as x86
little-endian 32-bit with image base `0x00400000`, loaded from
`<ra2-install>/gamemd.exe`.

Rust conclusions are frozen to clean HEAD
`cbf4d8711d6c136964a2e9210c442e1c79542d69`. Its simulation tree differs from
the older Checkpoint-C baseline only by committed lifecycle change
`95bef99dc2c121d37b9e45298b32926d5667dd6e` (`Implement ordered lifecycle
authority`). The shared working tree contains one companion-owned change in
`src/sim/world/techno_ai.rs`; that dirty blob was excluded from every durable
comparison. No Rust source, Ghidra metadata, INI, asset, Cargo state, stage, or
commit was changed by this investigation.

Checkpoint C is consumed at exact reviewed SHA-256
`CBE8307F6AF27760A151D0A599C5D7400727840E3C6C2195FFA1598E82ADE37D`.
Its authority here is binary/INI population and precedence only; its older
`cacc073f...` Rust snapshot is superseded by this report's clean HEAD.
Checkpoint B's speed and RawTrack reports remain authoritative for numeric
stepping. This report assigns the effects around those steps; it does not
rederive their formulas.

## 2. Native Authority Model

### 2.1 The live object pass is the visibility boundary

The native logic loop at `0x0055AFB0` reads the current vector member, calls its
object update through vtable `+0x5C`, increments the loop index, and re-reads the
live vector count. Fresh `decompile_function(0x0055AFB0)` and the loop assembly
at `0x0055B5FF..0x0055B61B` establish that this is not a frozen snapshot.
`decompile_function(0x0055BAE0)` shows the remover compacting the vector left and
clearing the object's registration field.

Consequences are exact:

1. If the current object removes a later object, that later object is absent
   before its scheduled slot and does not run.
2. If the current object removes itself or an already-earlier member, the next
   member shifts into the already-consumed index; the unconditional increment
   skips that shifted member in this pass.
3. If an object is appended at the tail, the re-read count can admit it later in
   the same pass.

Those rules apply to movement-caused UnInit because conceal/live-vector removal
finishes inside the current object's call. A separate Rust mover snapshot cannot
preserve these semantics merely by sorting IDs.

### 2.2 Owner classification

| Native owner class | Scoped actions |
|---|---|
| Current mover's Drive Process | dynamic target refresh, point/cell movement, track completion, empty/queued arrival entry |
| Current Unit per-cell hook | factory contact breakup, wall crush, generic scatter/crush dispatch, Foot per-cell tail |
| Current Unit/Infantry Tube leaf and class suffix | branch-specific `+0x174`; clamped `+0x544` speed writes; completion per-cell/lifecycle effects; then ungated substantive `+0x4A0` discovery/visibility/radar/tracker/cache work; full `+0x324` family remains an explicit blocker |
| Current victim's lifecycle calls | mark removal, Limbo/conceal, UnInit, live-Logic removal, pending-delete enqueue |
| Current lifecycle caller | Reveal coordinate commit/Mark and eligible tail registration; Techno Limbo BREAK broadcast before Conceal |
| Affected gate's own object turn | gate transition finalization, opening/hold/close mission progression and sound |
| One late `Main_Tick` service | physical destruction/free of objects already made non-live/non-occupying |
| Proven native global movement-effect tail | none |

This distinction matters. A Rust-native event buffer is acceptable only if it
commits the complete gameplay-bearing effect before the next native-equivalent
object slot. A buffer that survives to the end of all movers is a different
owner.

## 3. Key Fields, Lists, and Vtable Identities

Local Ghidra labels are navigation aids only. Identities below were rebuilt from
COL/TypeDescriptor bytes, slot reads, bodies, receiver flow, and callsites.

### 3.1 Unit and Infantry identities

The Unit vtable is `0x007F5C70`. Fresh
`read_memory(program="gamemd.exe", address="0x007F5C6C", length=8)` reads COL
`0x0080CC68` at vtable-minus-four;
`read_memory(program="gamemd.exe", address="0x0080CC68", length=24)` reads
TypeDescriptor `0x00842D80` from COL `+0x0C`; and
`inspect_memory_content(program="gamemd.exe", address="0x00842D80", length=64)` decodes
`.?AVUnitClass@@`. The following slot reads and fresh body/caller reads bind the
load-bearing virtuals used by this report:

| Unit slot | Slot address/read | Body | Verified use |
|---:|---|---:|---|
| `+0x2C` | `0x007F5C9C -> 0x00746E20` | `0x00746E20` | returns Unit type value `1` for `+0x4A0` coordinate branch |
| `+0x124` | `0x007F5D94 -> 0x004D3780` | `0x004D3780` | multi-cell Mark remove/add |
| `+0x174` | `0x007F5DE4 -> 0x00743A50` | `0x00743A50` | Unit scatter callback |
| `+0x18C` | `0x007F5DFC -> 0x00739EC0` | `0x00739EC0` | Unit per-cell hook |
| `+0x1C4` | `0x007F5E34 -> 0x005F6A10` | `0x005F6A10` | occupied-cell helper in outside-radar-rectangle correction |
| `+0x280` | `0x007F5EF0 -> 0x0065ACE0` | `0x0065ACE0` | ordered Techno BREAK broadcast for Unit |
| `+0x324` | `0x007F5F94 -> 0x0070D1D0` | `0x0070D1D0` | visibility/out-code body used by Tube suffix |
| `+0x494` | `0x007F6104 -> 0x0070CC90` | `0x0070CC90` | add radar tracker using new cached coordinate |
| `+0x498` | `0x007F6108 -> 0x0070CCC0` | `0x0070CCC0` | remove radar tracker using old cached coordinate |
| `+0x49C` | `0x007F610C -> 0x0070CCF0` | `0x0070CCF0` | mark cached radar cell dirty |
| `+0x4A0` | `0x007F6110 -> 0x0070D990` | `0x0070D990` | post-Tube suffix callback |
| `+0x534` | `0x007F61A4 -> 0x007416A0` | `0x007416A0` | crush/scatter helper |
| `+0x544` | `0x007F61B4 -> 0x004D3710` | `0x004D3710` | clamped Techno speed-fraction setter |

The `+0x2C`, `+0x124`, `+0x174`, `+0x1C4`, `+0x280`, `+0x324`, `+0x494` through
`+0x4A0`, and `+0x544` values above were re-read with
`read_memory(program="gamemd.exe", ...)`; the other two were independently
rechecked by the native cold-review pass. This supplies COL walk, slot read,
and body/caller evidence rather than trusting local Ghidra labels.

The Infantry vtable is `0x007EB058`. Fresh
`read_memory(program="gamemd.exe", address="0x007EB054", length=8)` yields COL
`0x008033B8` at vtable-minus-four;
`read_memory(program="gamemd.exe", address="0x008033B8", length=24)` yields
TypeDescriptor `0x00825508`; and
`inspect_memory_content(program="gamemd.exe", address="0x00825508", length=64)`
decodes `.?AVInfantryClass@@`. Fresh raw slot reads bind:

| Infantry slot | Slot address/read | Body | Verified use |
|---:|---|---:|---|
| `+0x2C` | `0x007EB084 -> 0x00523340` | `0x00523340` | returns Infantry type value `0xF` for `+0x4A0` coordinate branch |
| `+0xD4` | `0x007EB12C -> 0x0051DF10` | `0x0051DF10` | Infantry/Foot Limbo chain |
| `+0xF0` | `0x007EB148 -> 0x005217C0` | `0x005217C0` | Infantry completion cell-occupancy side work |
| `+0xF8` | `0x007EB150 -> 0x004DE5D0` | `0x004DE5D0` | Infantry UnInit wrapper |
| `+0x124` | `0x007EB17C -> 0x004D3780` | `0x004D3780` | multi-cell Mark remove/add |
| `+0x174` | `0x007EB1CC -> 0x0051D0D0` | `0x0051D0D0` | Infantry scatter callback |
| `+0x18C` | `0x007EB1E4 -> 0x00519630` | `0x00519630` | Infantry per-cell callback |
| `+0x1C4` | `0x007EB21C -> 0x005F6A10` | `0x005F6A10` | occupied-cell helper in outside-radar-rectangle correction |
| `+0x280` | `0x007EB2D8 -> 0x0065ACE0` | `0x0065ACE0` | ordered Techno BREAK broadcast for Infantry |
| `+0x324` | `0x007EB37C -> 0x0070D1D0` | `0x0070D1D0` | visibility/out-code body used by Tube suffix |
| `+0x494` | `0x007EB4EC -> 0x0070CC90` | `0x0070CC90` | add radar tracker using new cached coordinate |
| `+0x498` | `0x007EB4F0 -> 0x0070CCC0` | `0x0070CCC0` | remove radar tracker using old cached coordinate |
| `+0x49C` | `0x007EB4F4 -> 0x0070CCF0` | `0x0070CCF0` | mark cached radar cell dirty |
| `+0x4A0` | `0x007EB4F8 -> 0x0070D990` | `0x0070D990` | post-Tube discovery/visibility/display callback |
| `+0x544` | `0x007EB59C -> 0x004D3710` | `0x004D3710` | clamped Techno speed-fraction setter |

Fresh `read_memory(program="gamemd.exe", address="0x007EB148", length=4)`,
`read_memory(..., address="0x007F5E34", length=4)`, and
`read_memory(..., address="0x007EB21C", length=4)` bind the added `+0xF0` and
shared `+0x1C4` slots; fresh `get_function_by_address` plus
`disassemble_bytes` reads cover bodies `0x005217C0` and `0x005F6A10`.

The bodies route an Infantry crush victim through the shared multi-cell Mark
helper, Foot Limbo, ordered Techno BREAK, and Foot/Object UnInit chain. They also
prove that Infantry Tube completion/blocked paths use concrete Infantry
per-cell/scatter bindings before the shared `+0x4A0` and `+0x544` Techno bodies.

The investigation-plan address `0x00741700` is not a function entry.
`get_function_by_address(0x00741700)` places it inside the body beginning at
`0x007416A0`. All conclusions and handoff tests must use `0x007416A0`.

### 3.2 Fields and structures used by this slice

| Receiver | Offset | Width | Verified role | Evidence |
|---|---:|---:|---|---|
| Object | `+0x14` bit 2 | bit | when set, enables signed Tube `+0x684 >= 0` suppression of the type-5 radar-event call | `0x70DA79..0x70DA93` |
| Cell-list object | `+0x30` | pointer | next member in selected CellClass object list; saved before victim mutation | `0x007416A0`; `0x0047E8A0`; `0x0047EA90` |
| Object | `+0x81` | byte | InLimbo state set by Conceal | `0x005F4D30` |
| Object | `+0x90` | byte | alive byte cleared by UnInit and checked by Drive after per-cell effects | `0x005F65F0`; `0x004B1D12..0x004B1D2E` |
| Object | `+0x98` | byte/flag field | live-Logic registration cleared by the vector remover | `0x0055BAE0` |
| Techno | `+0x208/+0x20C` | two dwords | cached display coordinates read/written by the Tube post-leaf `+0x4A0` body | `0x70D9E0..0x70D9F0`; `0x70DB80..0x70DBB6` |
| Techno | `+0x41B` | byte | discovery state initialized from inverse shroud when global `0x00A8B238` is zero | `0x70D997..0x70D9C6` |
| Techno | `+0x423` | byte | radar-tracker membership set by concrete `+0x494` add and cleared by concrete `+0x498` remove | `0x70CC90`; `0x70CCC0`; `0x70DB76..0x70DBC6` |
| Techno | `+0x174/+0x17C` | dword/dword | start-frame sentinel and duration used by the local-owner periodic `+0x49C` branch | `0x70DBE9..0x70DC22` |
| Techno | `+0x21C` | pointer | owning House compared with current-player global for the periodic dirty branch | `0x70DBCC..0x70DBE5` |
| Techno | `+0x578/+0x57C` | 64-bit double | applied/current speed fraction clamped to `[0.0,1.0]` by shared `+0x544` body; distinct from Drive target fraction | `0x004D3710..0x004D3774` |
| Foot | `+0x5A0` | pointer | auxiliary navigation target cleared by Stop_Moving | `0x004DF0D0` |
| Foot | `+0x5A4` | pointer | NavCom target read by Drive refresh and cleared by Stop_Moving | `0x004B05E4..0x004B0638`; `0x004DF0D0` |
| Foot | `+0x684` | signed byte | Tube selector/state; only when object `+0x14` bit 2 is set does a nonnegative value suppress the body’s type-5 radar-event call | `0x70DA79..0x70DA93`; Tube callers |
| Foot | `+0x6B3` | byte | OnArrival re-entry/guard state | `0x004D82B0`; Foot AI reset near `0x004DA54E` |
| CellClass | `+0xE4` | pointer | selected ordinary/ground content-list head | `0x0047E8A0`; `0x0047EA90` |
| CellClass | `+0xE8` | pointer | selected alternate/bridge content-list head | `0x0047E8A0`; `0x0047EA90` |

Offsets are stated in their native receiver frames. The Rust handoff must model
the behavior, not recreate these C++ byte layouts.

## 4. Core Native Logic

### 4.1 Accepted Drive cell transition

Fresh `disassemble_bytes` of `DriveLocomotionClass::Process_Drive_Track @
0x004B0F20` establishes this current-object sequence for an accepted cell
transition:

1. Owner vtable `+0x124(0)` removes the old occupation mark at `0x004B17C8`.
2. Owner vtable `+0x1B4` writes the new coordinate.
3. `OnBridge` state is updated at `0x004B182D..0x004B184A`.
4. A destination-cell object/callback walk runs at `0x004B1851..0x004B1978`.
   Fresh
   `disassemble_bytes(program="gamemd.exe", start_address="0x004B1840",
   end_address="0x004B19A5")` shows that callbacks can mutate the mover.
5. The owner-alive test at `0x004B197E..0x004B1989` branches directly to the
   epilogue at `0x004B25F9` when those callbacks killed the mover. This path
   **does not** add the new Mark and does not call per-cell `+0x18C`.
6. Survivor-only: owner vtable `+0x124(1)` adds the new mark at `0x004B1993`.
7. Survivor-only: Unit per-cell vtable `+0x18C(2)` runs at `0x004B1CFD`; the
   second completion site is `0x004B220F`.
8. Drive immediately checks owner alive/limbo/state gates at
   `0x004B1D12`, `0x004B1D20`, and `0x004B1D2E` before any survivor path resumes.

The shared `+0x124` body is `0x004D3780`. Fresh decompile/disassembly shows arg
`0` selecting `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0` and
args `1`/`3` selecting `EnterCell_AddToMultiCells @ 0x005683C0` after the common
coordinate gate. Both helpers update each footprint cell and immediately call
`CellClass` recalculation before advancing to the next footprint cell.

Therefore the accepted-cell contract has two observable outcomes. A survivor
finishes the destination Mark and per-cell effects inside the current Drive
call. A mover killed by the destination-cell callback walk exits with the old
Mark already removed but without the new Mark or per-cell suffix. Both outcomes
are settled before the next live object. The clean Rust local move helper
resembles part of the remove/add order, but its bulk host and deferred effects
do not preserve this callback/lifecycle gate.

### 4.2 Unit per-cell ordering

Fresh `decompile_function(0x00739EC0)` and disassembly of its tail establish the
following ordering inside Unit vtable `+0x18C`:

1. Dock/refinery/building contact cases and stop/arrival-related branches run.
2. War-factory contact may be broken synchronously through the radio path.
3. Eligible wall overlay crush plays sound, calls `DestroyOverlay(-1)`, and
   writes rocking state at `0x73AFD4..0x73B06E`.
4. The Unit calls its `+0x534` helper at `0x73B089`, resolving to
   `0x007416A0`.
5. Only if owner alive byte `+0x90` remains nonzero does the Foot per-cell tail
   run. A concealed/absent-from-live-Logic but still-alive owner passes this gate.

This fixes two common documentation traps: `0x00739EC0` is the Unit per-cell
hook, while `0x007416A0` is the separate crush/scatter helper; and wall effects
precede generic crush/scatter in this tail.

### 4.3 Crush victim enumeration and teardown

Fresh `decompile_function`/`disassemble_function(0x007416A0)` shows two modes.
In the lethal mode, the helper walks the selected CellClass linked list in its
existing list order and saves `next = victim+0x30` before callbacks can unlink
the victim. It applies the ability, alliance/train, distance (`<= 0x3FFF`), and
victim-state gates. Fresh
`disassemble_bytes(program="gamemd.exe", start_address="0x00741840",
end_address="0x00741935")` proves two distinct active Infantry branches.

For normal crush, the verified order is:

1. Choose the victim's crush sound and call `VocClass::PlayAt @ 0x007509E0`.
2. Run victim `+0x170` record/kill and `+0xE0(crusher)` callbacks.
3. Call victim vtable `+0x124(0)` at `0x007418FC` to remove occupation.
4. Call victim vtable `+0xD4` at `0x00741906` for Limbo/conceal.
5. Call victim vtable `+0xF8` at `0x00741910` for UnInit.
6. Restore the saved `next` at `0x00741916`, test it at `0x0074191D`, and loop
   directly to `0x007417BE` when non-null.

The normal `0x007416A0` crush body contains no RNG call. Sound creation is still
ordering-sensitive: it occurs before victim cleanup and before the next victim
in native cell-list order.

There is **no per-victim crusher alive/limbo recheck** at `0x0074191D`. If the
first victim's sound, record, radio, Limbo, or detach callbacks kill or conceal
the crusher, a saved non-null second victim is still visited. The crusher
`+0x90` gate occurs only after `+0x534` returns to Unit per-cell/Drive. Any Rust
loop that aborts the victim chain as soon as the crusher changes lifecycle state
is therefore DRIFT.

For a stock Infantry victim on the normal branch, Section 3 binds the final
three virtuals to `0x004D3780`, `0x0051DF10`, and `0x004DE5D0`. The exact
concrete sequence is important:

1. explicit `+0x124(0)` unmarks the victim;
2. explicit `+0xD4` runs `0x0051DF10 -> Foot Limbo -> Techno Limbo -> BREAK ->
   Conceal`; Conceal calls Mark-remove again, removes live-Logic membership, and
   sets InLimbo;
3. `+0xF8 -> 0x004DE5D0 -> ObjectClass::UnInit @ 0x005F65F0` performs its
   detach/listener step while the victim is still alive but already unmarked,
   concealed, absent from live Logic, and InLimbo;
4. Object UnInit then invokes derived `+0xD4` a second time. Fresh
   `decompile_function(0x0051DF10)` proves this call is not a no-op: it repeats
   locomotor `+0xAC` and writes `+0x6E8`, `+0x6DB`, and `+0x6C4` before the
   already-limbo gates suppress later tails;
5. Object UnInit clears alive and appends pending delete.

The distinct branch at `0x00741853..0x0074188D` saves `next`, copies victim
`+0x41A` to crusher `+0x41A`, calls crusher `+0xDC(0)`, calls crusher
`+0x3D4(victim+0x21C,1)`, and calls victim `+0xF8` directly. It bypasses the
normal crush sound, victim `+0x170`, victim `+0xE0`, explicit `+0x124(0)`, and
explicit pre-UnInit `+0xD4`. On this branch Object UnInit's detach/listener step
can run while the Infantry is still marked and not yet concealed; its own
derived `+0xD4` then performs Limbo/conceal before alive clear. The two branches
must not share a fabricated pre-UnInit lifecycle prefix.

These concrete chains establish synchronous occupancy, conceal, live-Logic
removal, dead-state, and delete-queue effects for the required stock Infantry
fixtures. They do not exhaust unrelated hooks of every derived class.

### 4.4 Scatter population and RNG

Fresh `decompile_function(0x00481670)` shows
`CellClass::Scatter_Objects` snapshotting the selected ground/bridge linked list
into a DynamicVector in list order, then dispatching each saved occupant through
vtable `+0x174`. The CellClass helper itself consumes no RNG.

For Unit, fresh slot/body reads resolve `+0x174` to `0x00743A50`. Its branch
logic conditionally consumes Scenario RNG: an inclusive `RandomRanged(1,4)`
gate, and when a direction choice is needed, `RandomRanged(0,2)-1`. Draw count
therefore depends on each occupant's branches and native list order.

Clean Rust's `scatter_blocker` instead handles one classified blocker, always
draws one eight-way value on its accepted path, searches directions, and uses a
movement-pass-wide `already_scattered` set. Population, eligibility, draw range,
draw count, and ordering are all DRIFT. A Rust-vs-Rust deterministic hash cannot
certify that RNG mechanism.

### 4.5 UnInit, Conceal, and physical deletion

Fresh `decompile_function(0x005F65F0)` establishes common Object UnInit order:

1. Defuse an attached bomb.
2. Run the Foot passenger/EMP helper where applicable.
3. Call `Detach_From_All_Lists @ 0x007258D0` while the object's `+0x90`
   alive byte is still nonzero. Registration, Mark, and limbo state are
   caller-dependent: normal Infantry crush reaches
   this point already unmarked and InLimbo; the direct-`+0xF8` Infantry branch
   can reach it still marked and not yet concealed.
4. Call derived vtable `+0xD4` for Limbo.
5. Clear alive byte `+0x90`.
6. Append the object to the pending-delete vector.

Fresh `decompile_function(0x005F4D30)` establishes Conceal's gameplay teardown:
deselect, derived conceal hook, occupation mark removal, display-layer removal,
animation detach, sound stop, conditional live-Logic removal at
`0x0055BAE0`, alpha/visual cleanup, and setting `InLimbo +0x81`. The exact
derived chain can add class work, but occupancy and live membership are not
deferred to physical deletion.

Fresh `decompile_function(0x00725C70)` shows the late pending-delete drain. Its
common virtual `+0x44` call reaches the focused, independently rechecked body at
`ObjectClass::IsDead @ 0x005F6690`; this report deliberately does not infer a
concrete receiver RTTI identity from that common slot alone. `IsDead` returns
dead after `+0x90` was cleared. The drain compacts the queue and destructs/frees
common UnInit objects. Main Tick places one drain at
`0x0055DE9F`, after the logic pass at `0x0055DC9E`, Network service, the normal
frame increment, and `0x0055E160`, but before the following `0x00637270` tail
call.

The correct lifetime wording is: the object remains physically allocated for
the interval from synchronous UnInit until the one late drain in the same
`Main_Tick`; it does **not** remain allocated for the remainder of that tick.
It is already non-live, non-occupying, and concealed before the current object
call returns. Older reports saying a common UnInit object necessarily survives
one or more complete ticks are stale.

#### 4.5.1 Reveal is Mark-before-register and can append same-pass work

A fresh read-only `decompile_function(0x005F4EC0)` and assembly at
`0x005F4F4A..0x005F5040` establish the active Reveal order. After its sentinel,
game-active, limbo, placement, and mode gates, Reveal clears InLimbo for the
attempt, computes/commits the adjusted coordinate through vtable `+0x1B4`, then
calls Mark through vtable `+0x124(1)`. If Mark fails, it restores InLimbo but does
not restore the old coordinate. Only after Mark succeeds does the type/mode
eligibility chain reach Logic registration at `0x005F5038..0x005F5040`.

The registration helper tail-appends to the same live Logic vector whose count
is re-read by Section 2.1. A movement/lifecycle action that successfully reveals
an eligible object can therefore make that object runnable later in the same
pass. Current clean Rust now has a result-bearing `try_reveal_entity` transaction
that clears limbo, commits coordinates, models Mark failure, marks occupancy,
emits output, then registers Logic membership. The order is a bounded MATCH;
caller-supplied placement/type eligibility and no-op output consumers remain
DRIFT or UNCHECKED as classified in Section 5.

#### 4.5.2 Techno Limbo broadcasts ordered BREAK before Conceal

Fresh `decompile_function(0x0065AA80)` shows the Techno Limbo tail checking
InLimbo, calling virtual `+0x280(3)`, then calling common Object Conceal. The
Unit COL and fresh `read_memory(0x007F5EF0,8)` in Section 3 bind Unit `+0x280`
to `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0`. Fresh
`decompile_function(0x0065ACE0)` shows an
ascending `0..capacity` walk that re-reads each sparse contact slot and sends
BREAK to every non-null contact.

Fresh `decompile_function(0x0065A970)` establishes sender-side order for BREAK:
clear every sender slot matching the target first, then synchronously invoke the
target receiver. Receiver/subclass handling can mutate further contact, dock,
building, or mission state before broadcast proceeds to the next slot. Current
Rust now preserves the ascending slot re-read, sender-clear, and synchronous
dispatch skeleton. Its receiver-class coverage is incomplete, so the bounded
core is MATCH while complete BREAK behavior remains DRIFT.

For the normal Infantry movement-crush branch, the resulting lifecycle spine is
therefore: sound and victim callbacks, Mark removal, Limbo's ordered BREAK
broadcast, common Conceal and live-vector compaction, alive clear, and
pending-delete enqueue, all before the next native object. Section 4.3's direct
`+0xF8` branch reaches detach before its derived Limbo/Conceal call instead. The
exhaustive meaning of every possible receiver BREAK
subclass remains outside this bounded D slice, but the broadcast owner and
ordering are closed.

### 4.6 Cell lists and immediate recalculation

Fresh body/callee reads of `0x005683C0`, `0x005687F0`, `0x0047E8A0`,
`0x0047EA90`, and `0x0047D2B0` establish:

- the selected content head is `CellClass+0xE4` or `+0xE8`;
- ordinary mobile objects prepend, while `WhatAmI == 6` buildings append;
- removal unlinks in place and clears the removed object's `+0x30` link;
- every affected footprint cell is recalculated synchronously;
- building-specific occupation counters change in the same add/remove call.

Later objects therefore observe the mutated native list and recomputed cell
attributes. This evidence does not prove that every Rust route, zone, blocker,
or path cache has a one-to-one native counterpart. A Rust-native cache is
allowed only if every consumer sees data equivalent to the already-mutated
native cell state at the same object boundary.

### 4.7 Dynamic NavCom refresh

Fresh
`disassemble_bytes(program="gamemd.exe", start_address="0x004B05D0",
end_address="0x004B0640")` proves this is not a generic moving-target branch.
The owner `WhatAmI` call at `0x004B05D9` must return `1` (Unit), the live NavCom
target at owner `+0x5A4` must exist, and the target `WhatAmI` call at
`0x004B05F0` must return `0xF` (Infantry). Only then does Drive call target
vtable `+0x4C`, compare the returned coordinate with the Drive head coordinate,
and invoke Drive vtable `+0x44` if it changed. The second refresh at
`0x004B0971..0x004B09A3` inherits the same Unit-owner/Infantry-target branch
established earlier in the Process body.

This happens in the current Unit's `DriveLocomotionClass::Process @ 0x004B0500`.
If Infantry A moves before following Unit B, B's later slot can refresh to A's
new coordinate. Clean Rust's broader global `drive_reaims` snapshot/commit
occurs before all movers and cannot reproduce that later-object read; this D
fixture and claim are limited to the proved Unit-following-Infantry case.

### 4.8 Track completion, finalization, and arrival

Fresh disassembly of `Process_Drive_Track @ 0x004B0F20` shows track completion
clearing track `+0x58` to `-1` at `0x004B210E`, point state `+0x5C` at
`0x004B2115`, invoking owner per-cell `+0x18C(2)` at `0x004B220F`, applying
conditional Stop/OnArrival work, and calling owner `+0x504` at `0x004B228B`.
These completion fields and effects belong to the current Drive invocation.

On the next eligible no-active-track invocation of that same object:

- the empty-queue path calls owner `+0x480(NULL,1)` to clear the destination;
- the queued path calls `Stop_Moving @ 0x004DF0D0` and then owner `+0x484`, whose
  common body reaches `FootClass::OnArrival @ 0x004D82B0` and installs at most
  one queued NavCom synchronously.

`Stop_Moving` clears owner `+0x5A0/+0x5A4`. Fresh slot reads also correct a stale
binding claim: normal Unit `+0x480` resolves to wrapper `0x00741970`, not
directly to `Set_Destination_Internal @ 0x004D94B0`; Foot base and Infantry have
their own bindings. `OnArrival` sets guard `+0x6B3`, runs common Techno work,
optional scatter/piggyback/queue branches, and eventually sets speed fraction
zero on the empty completion path; Foot AI resets the guard later.

Clean Rust front-loads all pending arrivals in stable-key order before movers and
defers all finished movement finalization until after movers. Both owners and
visibility are DRIFT.

### 4.9 Gate request versus gate progression

The mover-side live obstacle check in Map code at `0x00578AD0` walks cell
objects and calls the allied gate helper `0x00452540`. That helper assigns and
commences gate mission `0x18` but still reports the current entry as blocked.
The request mutation is therefore synchronous with the mover.

Gate transition completion and mission progression belong to the gate's own
Techno/object turn: the common update reaches transition helpers near
`0x006FA5C6`, then Mission Dispatch near `0x006FA646`; mission `0x18` body
`0x0044E440` owns opening, hold, live-footprint obstruction scan, closing, and
sound.

Object order is observable. For `mover A -> gate -> mover B`, B may see a gate
state advanced during the gate's slot. For `gate -> mover A`, A's request waits
until the gate's next turn. Clean Rust requests during movement but advances all
gates after all movers, erasing both cases.

### 4.10 War-factory contact

The successful factory exit/radio chain creates the reciprocal building-unit
contact before drive-out. Unit cell-entry logic reads that live relation for the
`NumberImpassableRows` exception. The frozen supporting proof is
`docs/research/WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md` at SHA-256
`E86C7A0C6AC9805FDAD53B8C63E87382521B66318912640680F607D889F41FE7`;
the current-object breakup order below was independently re-read from the live
binary.

Fresh Unit per-cell disassembly shows the contact breakup in the moving Unit's
current call: at `0x73A93D` it sends radio `0x08`; `TechnoClass::Receive_Radio @
0x006F4AB0` case 8 synchronously sends `0x19` then `0x03`, clearing the relation
and associated dock state. A multi-cell-budget Unit therefore loses the
privilege as soon as it fully enters the first non-building cell; a later entry
in the same Drive invocation and all later objects see it gone.

Clean Rust builds a fixed privilege map separately for each mover before that
mover's inner Drive budget, then clears contacts in a global post-movement sweep.
That per-mover frozen map can extend the privilege across additional
same-invocation entries and later objects.

### 4.11 Wall crush

Fresh Unit per-cell assembly at `0x73B04D..0x73B06E` shows eligible wall crush
playing its sound, pushing `-1`, calling `DestroyOverlay @ 0x00480CB0`, and
writing rocking state before generic `+0x534` crush/scatter. The overlay and
connectivity consequences are therefore visible before the next live object.

Clean Rust scans final standing cells after all movers and applies shared wall
teardown without the native sound/rocking order. It can miss a wall fully
entered and left within one multi-step Drive budget. Stock BFRT reachability of
this drive-over wall branch is closed by
`docs/research/WALL_CRUSH_ON_DRIVEOVER_GHIDRA_REPORT.md` at SHA-256
`A6C68D29F55819883DE3411ED2C5E0AC03B3687DB30FD9A0CC8E301F9A78F778`.
What remains separate is alternate/custom weapon-ability predicate reachability
and exhaustive `DestroyOverlay(-1)` cleanup equivalence; neither uncertainty
changes the verified stock owner/timing result.

### 4.12 Unit/Infantry Tube completion lifecycle suffix

Checkpoint C deferred OQ-24 because neither class Tube suffix has an explicit
owner-alive/limbo guard. Fresh read-only disassembly closes the scheduling
question, but the suffix is not a generic marker: it performs speed and
discovery/visibility/display work that must remain in the same object turn.

Fresh `disassemble_bytes(program="gamemd.exe", start_address="0x004D3710",
end_address="0x004D3778")` establishes the shared `+0x544` body. It writes the owner-wide
applied/current speed fraction, not Drive's separate target fraction. Input
`>= 1.0` writes exact `1.0`; input `<= 0.0` writes canonical positive zero;
otherwise it copies the double's low/high dwords to owner `+0x578/+0x57C`.
The `TEST AH,0x41` branch also sends unordered/NaN to zero, so `+inf` clamps to
one, `-inf` to zero, and `-0.0` canonicalizes to `+0.0`. Unit slot
`0x007F61B4` and Infantry slot `0x007EB59C` both raw-read to this body.

For Unit, accepted completion first commits coordinates, clears Tube `+0x684`,
and calls owner `+0x124`. If `+0x5A4` equals the exit/current Cell, it then calls
Unit `+0x174 -> 0x00743A50` at `0x00736028`; otherwise that callback is skipped.
Both arms set `+0x68B`, call Unit `+0x18C(2) -> 0x00739EC0` at `0x0073603F`, and
only then call shared `+0x544(1.0)` at `0x00736047..0x0073604F`. The outer Unit
wrapper calls `+0x4A0(0)` at `0x007363BB`. Lifecycle mutation inside `+0x18C`
does not suppress either later call.

Infantry accepted completion inside `0x0051B350` is deliberately asymmetric.
After coordinate, owner `+0x124`, and `+0xF0` work, the `+0x5A4`-equal arm calls
Infantry `+0x174 -> 0x0051D0D0` at `0x0051BA6F` and does **not** directly set
the speed fraction. The unequal ordinary arm instead calls `+0x544(1.0)` at
`0x0051BA79..0x0051BA81`. Both arms then clear Tube `+0x684`, set `+0x68B`, and
call Infantry `+0x18C(2) -> 0x00519630` at `0x0051BA9B`; the class wrapper later
calls `+0x4A0(0)` at `0x0051BACD`. Thus the equal/`+0x174` arm preserves the
prior applied/current fraction, while the ordinary arm writes one before the
lifecycle-capable per-cell call.

A concrete active `+0x18C` callee chain exists: Infantry PerCell `0x00519630`
can call the eligible destination building's AddGarrisonOccupant body at
`0x0051972D -> 0x00522910`; fresh
`disassemble_bytes(0x00522910..0x00522945)` shows current Infantry virtual
`+0xD4` at `0x00522931`, which reaches `0x0051DF10 -> Foot Limbo 0x004DB260 ->
Techno Limbo 0x006F6AC0 -> Limbo tail 0x0065AA99 -> Object Conceal
0x005F4D30`. The Tube owner is synchronously concealed and removed from live
Logic inside `+0x18C`.

Control nevertheless returns to the Infantry wrapper; there is no intervening
alive-byte `+0x90` or InLimbo guard. Section 3 binds the call to `0x0070D990`.

Fresh `decompile_function(0x0051B350)` and
`decompile_function(0x007359F0)` establish the blocked-exit alternative. Each
class snapshots/walks selected-cell occupants, invokes eligible concrete
`+0x174` scatter callbacks, then passes `0.0` to owner `+0x544` at
`0x0051B8F8..0x0051B8FC` or `0x00735F66..0x00735F6A`. The helper returns without
completion `+0x18C` and retains Tube `+0x684`, but the class wrapper still
invokes `+0x4A0(0)`. Its trace is therefore:

`blocked exit -> occupant +0x174 callbacks -> owner +0x544(0.0) -> helper
return -> ungated owner +0x4A0(0) -> class return`.

The shared `+0x4A0` body at `0x0070D990` is substantive. Fresh
`disassemble_bytes(program="gamemd.exe", start_address="0x0070D990",
end_address="0x0070DC40")`, direct disassembly of `0x0070D1D0..0x0070D410`,
and the Section 3 raw slot reads establish this bounded Tube-call sequence:

1. At `0x70D997..0x70D9C6`, if discovery byte `+0x41B` is clear and global
   `0x00A8B238` is zero, query position/shroud and write inverse-shrouded state
   to `+0x41B`.
2. Unit and Infantry `+0x2C` return `1` and `0xF`, respectively, so their Tube
   `+0x4A0(0)` calls bypass the type-6/caller-zero cached-coordinate shortcut and
   transform `+0x9C/+0xA0/+0xA4` through `0x006557F0` mode zero.
3. Initialize an out-code to zero and call shared virtual `+0x324 -> 0x0070D1D0`.
   That concrete body can write out-code `1` or `2` and returns visibility; it
   already returns false for InLimbo `+0x81`.
4. Unconditionally at `0x70DA48..0x70DA69`, load global `0x00880A04`, invoke
   its `[vtable +0x78]` call, and copy the returned four-dword radar-surface
   rectangle. This occurs even when `+0x324` returned false or the owner is
   InLimbo; no further side effect of that virtual is inferred here.
5. Only after the rectangle query, the type-5 radar-event call requires both nonzero out-code and true
   visibility. It is allowed when object `+0x14` bit 2 is clear or signed Tube
   `+0x684 < 0`; only bit 2 set together with nonnegative `+0x684` suppresses it.
   Coordinates use signed truncation toward zero by 256,
   `(v + ((v >> 31) & 0xFF)) >> 8`, are packed as signed 16-bit X/Y, and call
   `CreateRadarEvent @ 0x0065FA70` with type `5`; that callee may deduplicate or
   decline allocation, so the proved action is the call, not unconditional event
   creation.
6. If the coordinate lies outside the radar-surface half-open rectangle and
   visibility is true, call `+0x1C4(1)` and `0x00578540`; on success recompute
   coordinates through `0x006557F0` mode one.
7. At `0x70DB66..0x70DB74`, InLimbo explicitly forces visibility false a second
   time before tracker membership changes.
8. If `+0x423 != 0` and either cached coordinate changed or visibility is false,
   call `+0x498 -> 0x0070CCC0` before cache writes. That body removes the tracker
   entry using old `+0x208/+0x20C` and clears `+0x423`.
9. Always write `+0x208` then `+0x20C`. If `+0x423 == 0` and visibility is true,
   call `+0x494 -> 0x0070CC90` afterward; that body adds the tracker entry using
   the new coordinates and sets `+0x423`.
10. Independently, when owner `+0x21C` is the current player and positive
   remaining time derived from `+0x17C/+0x174` is divisible by Rules `+0x88`,
   call `+0x49C -> 0x0070CCF0`, which marks the radar cell at `this+0x208` dirty.

The Unit and Infantry `+0x4A0` slots both bind `0x0070D990`. Thus an Infantry
concealed inside `+0x18C` still executes this body. Its concrete `+0x324` returns
false, but the radar-surface rectangle virtual/query still runs; only the later
type-5 and on-screen branches are skipped. The Limbo helper has
already removed ordinary tracker membership through `+0x498`, so the suffix
normally performs no second removal; it still writes both cache coordinates, no
tracker add follows, and the player/timer `+0x49C` branch remains independently
reachable. The direct order,
bindings, and small `+0x494/+0x498/+0x49C` bodies are verified. The full
`+0x324` visibility predicate and tracker downstream/capacity/clamp/pixel
equivalence remain touched-not-exhausted production blockers.

Current clean Rust's global Tube helper moves occupancy/position, refreshes its
generic floating-point screen-coordinate cache, and clears
`low_bridge_tube_state`; it does not perform the branch-specific `+0x544` writes
or this post-leaf discovery, visibility, type-5 radar, and tracker transaction.
It has no per-object `+0x41B/+0x423` equivalents, and `EnemyObjectSensed` has no
producer. The inspected app minimap path derives directly from EntityStore/cell
visibility and does not consult `in_limbo`; ordered lifecycle display outputs
are also drained as no-ops. After Tube state clears, current bulk routing can also run the same
entity through forced/ordinary movement; Drive-only fraction logic is not this
native zero/one/preserve mechanism and may overwrite it. Those differences are
DRIFT, not a partial cache match.

Therefore C OQ-24 resolves **yes only for scheduling**: a Tube-invoked virtual
can conceal/remove the owner from live Logic, and the class-specific
post-leaf `+0x4A0` call still executes. This is not permission to invent an
early return after `+0x18C`, nor does it certify the full `+0x4A0` effect body.
Malformed Tube data and first-post-load mixed state remain the separate C
deferrals.

## 5. Clean-Rust Baseline Comparison

### 5.1 Current phase order at `cbf4d871...`

Direct current-HEAD reads establish this exact scoped order. The companion-owned
working-copy delta in `src/sim/world/techno_ai.rs` is excluded, but its clean
HEAD blob is PRIMARY evidence: production `object_ai_stage` already walks live
Logic order through `for_each_live_object` and hosts partial Unit mission work.
Phase-1 ground movement immediately afterward still snapshots live order and
builds a second mover vector.

| Order | Clean-Rust owner at `cbf4d871...` | Native disposition |
|---:|---|---|
| 1 | production `object_ai_stage` uses `for_each_live_object` for partial Unit mission/object-AI work | bounded live-host mechanism already exists and must be extended, not duplicated |
| 2 | `live_object_order_snapshot` for Phase-1 movement | native ground locomotion belongs inside that live object host, not a later snapshot |
| 3 | one-time `blocker_neighbor_counts` and global `drive_reaims` snapshot/commit | later consumers must see earlier current-object mutations; proved reaim is Unit-following-Infantry in current Drive |
| 4 | `tick_low_bridge_tube_movement` over sorted entity keys | class Tube leaf in the current Unit/Infantry turn |
| 5 | `tick_forced_drive_tracks` | exact caller initializes; later ordered Drive consumes |
| 6 | initial mover/block-set collection and global pending-arrival sweep | each object's current/next eligible Drive slot |
| 7 | rebuilt snapshot mover vector and bulk mover loop | one live ordered object slot at a time |
| 8 | mover-local bridge update and deferred chain/cell commits | acceptable only where fully committed before leaving this mover; resulting lifecycle currently is not |
| 9 | population-wide formation sync | no equivalent population-wide native owner |
| 10 | `crush_kills` stable-ID sort/dedup, sound, health-zero, and `LifecycleRequest::Uninit` emission | encounter-order sound and synchronous victim lifecycle inside current Unit per-cell call |
| 11 | global `finished_entities` finalization | current Drive completion |
| 12 | global locomotor phase update | each class Process/leaf in its object slot |
| 13 | global Hover vertical/wake scan | Hover Process in its object slot |
| 14 | return to `Simulation::advance_tick`, then drain all pending lifecycle requests through canonical `Simulation::uninit` | each movement-caused UnInit already completed before next object |
| 15 | all gate runtimes | each gate's own object turn |
| 16 | all war-factory contact breaks | current Unit per-cell call |
| 17 | all wall drive-over crushes | current Unit per-cell call |
| 18 | later ground Teleport pass and locomotor piggyback restore; miner batch pipeline and Ship wake/global phase work remain separate owners | their Checkpoint-C class/mission/locomotor object slots |
| 19 | later normal phases, projected `binary_frame` assignment, then one pending-delete drain | assignment-before-drain call placement matches bounded relative order; authoritative frame cadence/value is DRIFT |

Primary current source is clean HEAD `src/sim/world/techno_ai.rs:282..320`,
`src/sim/movement/movement_tick.rs:872..2040`, and
`src/sim/world/mod.rs:2029..2240`. Current blob IDs are recorded in Section 14.2
so later line drift cannot silently change this comparison.

### 5.2 Deferred vector/list classification

| Clean-Rust state/list | Current purpose | Native owner classification | Required disposition |
|---|---|---|---|
| `drive_reaims` | target coordinate updates | proved Unit-following-Infantry current Drive Process | remove global authority; perform live branch-gated read/commit in Unit slot |
| movement/live-order snapshots | bulk iteration | live Logic vector | replace as gameplay scheduler; a pure read-only precompute may survive only with full ordered-commit proof |
| `blocker_neighbor_counts` | shared path heuristic counts | later object consumes already-mutated live state | remove one-time authority or recompute/invalidate before each affected consumer |
| `entity_block_sets` | per-owner blocker candidates | later object consumes live CellClass/object state | generation refresh is insufficient while an early-unmarked victim is still alive; rebuild from one authority |
| pending Drive arrivals | next-step arrival work | that object's no-active-track Drive Process | remove global sweep |
| `pending_bridge_update` | borrow-local OnBridge/occupancy state | current mover accepted-cell body | non-vehicle-deferred paths apply before leaving the mover at `movement_tick.rs:1494..1503` or `1617..1625`; `DeferredCellCheck::Vehicle` explicitly suppresses it and its handler receives no update, so that branch is UNCHECKED |
| `deferred_drive_track_chain` | borrow-local forced/track cell work | current mover point/per-cell body | local commit boundary MATCH at `1705..1727`; resulting crush lifecycle is DRIFT because it survives the object slot |
| `deferred_cell_check` | borrow-local obstacle/cell work | current mover per-cell body | local commit boundary MATCH at `1729..1757`; resulting crush lifecycle is DRIFT because it survives the object slot |
| `already_scattered` | whole-pass scatter dedup | no proved native whole-pass owner | remove unless a narrower native branch independently proves identical suppression |
| `crush_kills` | delayed/sorted victim effects | current Unit per-cell call | remove global queue; use canonical UnInit in encounter order inside current slot |
| `pending_lifecycle_requests` | post-movement central lifecycle requests | current victim's synchronous lifecycle | retain request type only if applied before next object; current post-movement drain is DRIFT |
| `contains_crush_victim` | handled-ID skip across selected later loops | no native handled-ID bridge | remove at atomic cutover; it cannot repair live-vector compaction/order |
| `finished_entities` | delayed track/facing/arrival state | current Drive completion | remove global finalizer |
| formation sync | group minimum speed | explicit Drive Unit links only | remove population-wide authority |
| global locomotor phases / Hover tail / stable-ID Ship wake scan | class state updates | class Process | move complete class work into object slot atomically |
| global Tube / forced / Teleport / piggyback / miner owners | special ground movement | exact Checkpoint-C class/caller/mission owner | remove all old production callers with the atomic population flip |
| gate runtime list | gate progression | each gate's object turn | route through ordered Techno/Mission owner |
| `live_building_entry_skips` | per-mover gate/repair/bunker/bib/C4 plus contact-derived entry exceptions | native factory privilege reads the live contact and loses it at current Unit per-cell breakup | preserve unrelated entries; make only the contact-derived branch observe inline breakup before a later same-budget entry |
| factory contact cleanup | relationship teardown | current Unit per-cell call | move inline before possible further cell entries |
| wall crush scan | final-cell overlay damage | each fully entered cell's Unit per-cell call | move inline with sound/rocking/order |
| pending physical delete | storage reclamation | late Main Tick service | retain one queue/drain; Rust's assignment-before-drain relative placement matches, but authoritative frame cadence/value is DRIFT |
| sound/lifecycle output queues | presentation handoff | event creation is inline; presentation may be later | enqueue in exact encounter order and implement consumers; do not sort by ID or discard effects |

### 5.3 Lifecycle authority split

Commit `95bef99d` added a useful bounded lifecycle authority. It must be retained
and completed, not replaced. Current Reveal, sparse-slot BREAK, Conceal, Techno
Limbo, common UnInit, Logic-vector append/compact, duplicate pending-delete, and
one ordinary drain all have significant native-order structure.

Movement crush now reaches canonical `Simulation::uninit`; the old report claim
that it directly removes victims from EntityStore is false. The remaining split
is timing and authority:

1. During the current mover, `movement_occupancy.rs:675..689` (and the forced
   chain at `movement_tick.rs:774..785`) removes victim occupancy and clears
   `lifecycle.cell_marked`.
2. The victim remains alive, non-limbo, and non-dying through the rest of the
   mover loop. `crush_kills` accumulates it; `contains_crush_victim` suppresses
   only selected later mover/finalizer/phase/Hover paths.
3. After every mover and formation sync, `movement_tick.rs:1775..1800` sorts and
   deduplicates victims by stable ID, emits crush/Die sounds, sets health to zero,
   and pushes `LifecycleRequest::Uninit`.
4. Only after `tick_movement_with_grids` returns does
   `world/mod.rs:2139..2144` apply every request through central
   `Simulation::uninit`, before the global gate/factory/wall postludes.

This is still major DRIFT. BREAK, Conceal, Logic-vector compaction, alive clear,
and pending-delete enqueue occur after all movers rather than inside the crusher's
current per-cell call. A victim after the live cursor can run when native would
remove it; removal of a victim before the cursor cannot produce native's shifted-
successor skip. Stable-ID sound/death order replaces CellClass encounter order.

The split can also reintroduce a crushed victim into derived blocker state.
`OccupancyGrid::rebuild` respects `cell_marked=false`, but blocker and neighbor
builders filter only `dying`; during the split window they can scan the still-
alive victim from EntityStore. `blocker_neighbor_counts` is built only once,
while generation-triggered `entity_block_sets` can rebuild from that inconsistent
authority. Broader native route/zone/cache correspondence remains UNCHECKED.

The physical drain has a bounded relative-order match but not a frame-value
match. Current Rust assigns `binary_frame` at `world/mod.rs:2029..2039` and runs
ordinary `process_pending_delete` at `2041..2044`. Clean HEAD
`src/util/fixed_math.rs` and `src/app_types.rs` set production simulation to
45 Hz (`SIM_TICK_HZ = 45`, integer `SIM_TICK_MS = 22`), while `world/mod.rs` assigns
`binary_frame = (total_sim_ms * 15) / 1000`, so most drains occur without an
`N -> N+1` change. Native increments `g_CurrentFrameCounter` once per normally
reached `Main_Tick` before its late drain. Assignment-before-drain call placement
is therefore a bounded MATCH, while authoritative frame cadence/value is DRIFT.
Exact correspondence to native `0x0055E160`, intervening tails, concrete
destructors, and late-skip flags remains UNCHECKED.

One separate current drift remains: app animated-death completion calls
`sim.uninit` at `src/app_sim_tick.rs:599..615` after that tick's ordinary drain,
so physical removal waits for a later drain. In addition, the app drains every
`LifecycleOutput` variant with an empty arm at `src/app_sim_tick.rs:619..630`;
ordered emission exists, but display/audio/animation effects are no-ops.

### 5.4 Current-global/deferred owner-or-blocker matrix

This is the zero-add owner-or-explicit-blocker inventory for every family named
by the approved Checkpoint-D plan and the lifecycle authority input. `MATCH` is
used only for a positively proved bounded mechanism; partial resemblance is
`DRIFT` or `UNCHECKED`, never an implicit pass. It does not claim exhaustive
closure of dependencies explicitly assigned to another system context.

| Surface/family | Native owner and visibility | Frozen clean-Rust owner | Verdict | Production disposition |
|---|---|---|---|---|
| Dynamic NavCom target refresh | Unit following Infantry in current Drive Process; sees earlier Infantry's new coordinate | broader global pre-mover `drive_reaims` snapshot | DRIFT | implement exact class gates and live read in Unit slot |
| Normal path/head-to and first track | Drive Process/Process_Movement in object slot | command/global movement preparation | DRIFT (Checkpoint B/C) | route through per-object class owner |
| Active track point/cursor work | current Drive Process | snapshot mover loop plus separate cadence | DRIFT (Checkpoint B) | replace with exact per-object Drive owner |
| Tube producer and Unit/Infantry leaf | producer state at current caller; Tube leaf preempts ordinary locomotor next object turn | `tick_low_bridge_tube_movement` global pass | DRIFT (Checkpoint C) | migrate producer/leaf together |
| Tube completion/blocked call scheduling | branch-specific `+0x174`; completion `+0x18C`; ungated class wrapper `+0x4A0`; exact Unit/Infantry asymmetry | global Tube helper has no corresponding class call transaction | DRIFT | preserve the verified branch order in one object turn |
| Tube applied/current speed fraction | shared `+0x544` writes blocked zero; Unit completion one after `+0x18C`; Infantry ordinary completion one before `+0x18C`; Infantry `+0x174` completion preserves prior value | no owner-wide cross-Foot field or Tube-specific zero/one/preserve write; later bulk routing may overwrite via Drive-only state | DRIFT | add common Foot/Techno state and prohibit same-tick generic overwrite after Tube wrapper |
| Tube `+0x4A0` direct effect order | discovery update; visibility/out-code; unconditional radar-surface rectangle query; gated type-5 call; coordinate correction; tracker remove; unconditional cache writes; tracker add; periodic dirty | global Tube helper only refreshes generic floating-point screen cache; no discovery/tracker state or radar-event producer | DRIFT | implement the bounded direct order after the class leaf, including concealed-owner result |
| Full Tube `+0x324` visibility and tracker/pixel equivalence | live Unit/Infantry visibility/out-code predicate and downstream tracker/radar effects | app-side visibility/minimap path is a different mechanism and does not consult the proved per-object state | UNCHECKED | production blocker: exhaust `0x70D1D0` family and downstream tracker/pixel consumers |
| Forced-track initialization | exact mission/caller-time initialization; later Drive consumes | global forced prepass and incomplete caller set | DRIFT (Checkpoint C) | keep at caller; remove global authority |
| `blocker_neighbor_counts` | later object reads live mutated cell/object state | one-time pre-movement build | DRIFT | invalidate/rebuild before affected later consumer |
| Per-owner `entity_block_sets` | later object reads live mutated CellClass/object state | prebuilt sets can generation-refresh from split store/occupancy authorities | DRIFT | derive from one authoritative live state |
| Exact broader route/zone cache graph | immediate CellClass recalculation proved; full graph separate | `OccupancyGrid::generation` plus path/blocker caches | UNCHECKED | block parity claim pending system-context audit |
| Pending Drive arrivals | that object's next eligible no-track Drive slot | global sorted-key pre-mover sweep | DRIFT | remove sweep |
| Point coordinate/cell transition | current locomotor point body | current mover helper inside bulk pass | DRIFT overall | retain local arithmetic only under new host/order |
| Mark/occupancy add/remove | current point/victim lifecycle; immediate per-cell recalc | local grid mutation, sometimes separated from entity lifecycle | DRIFT | commit one transaction before next slot |
| `pending_bridge_update`, non-vehicle-deferred | current accepted-cell body | applied before mover borrow/slot ends | MATCH for local commit boundary | preserve; broader bridge predicate parity remains Checkpoint C |
| `pending_bridge_update` with `DeferredCellCheck::Vehicle` | depends on whether deferred occupancy accepts or blocks this crossing | application is suppressed and handler receives no update | UNCHECKED | trace the deferred clear/blocked outcomes before keeping or deleting the update |
| `deferred_drive_track_chain` | current point/per-cell body | consumed before next mover, but produced crush lifecycle survives | MATCH local commit / DRIFT lifecycle | retain local staging only; apply full effects before next slot |
| `deferred_cell_check` | current obstacle/per-cell body | consumed before next mover, but produced crush lifecycle survives | MATCH local commit / DRIFT lifecycle | retain local staging only; apply full effects before next slot |
| Nonlethal contention | current mover against live cell list | snapshot/block-set classification | DRIFT | read native-equivalent live state |
| Scatter dispatch/RNG | current Unit per-cell helper, occupant list order and conditional draws | one-blocker helper plus pass-wide dedup/eight-way draw | DRIFT | replace population/order/RNG mechanism |
| Normal crush victim selection/death | current Unit per-cell helper, cell-list order, saved-next loop without crusher recheck | early unmark then stable-ID tail sound/health/request | DRIFT | canonical UnInit in encounter order; continue saved-next chain |
| Direct-`+0xF8` Infantry crush branch | field copy and crusher callbacks, then direct victim UnInit without normal prefix | no separate proved branch | DRIFT | implement as distinct sequence; do not synthesize sound/Mark/Limbo prefix |
| `pending_lifecycle_requests` | current victim lifecycle inside current mover | drained through canonical UnInit only after all movement | DRIFT | apply before next live object |
| `contains_crush_victim` | no native handled-ID bridge | ad-hoc skip in selected mover/finalizer/phase/Hover loops | DRIFT | delete with atomic scheduler cutover |
| Formation speed | explicit linked Unit relation inside Drive | global group minimum sync | DRIFT (Checkpoint C) | remove population-wide formation authority |
| Finished movement finalization | current Drive completion | global `finished_entities` tail | DRIFT | commit inside current Drive call |
| Common/class locomotor phases | each active class Process/leaf post-work | global phase update | DRIFT (Checkpoint C) | move with complete class population |
| Hover vertical/wake state | Hover Process in object slot | global Hover tail | DRIFT (Checkpoint C) | move into Hover owner |
| Ship wake state | Ship Process in live object slot | stable-ID/global phase scan | DRIFT (Checkpoint C) | move into Ship owner |
| Ground Teleport/restore | Teleport Process and verified Foot restore points | population-wide phase/tail | DRIFT (Checkpoint C) | include in atomic population |
| Locomotor piggyback restore | verified Foot restore point in current object flow | global `tick_locomotor_piggyback_restore` | DRIFT (Checkpoint C) | migrate with Teleport/miner population |
| Miner mission/movement | miner mission then same miner locomotor in one slot | `tick_miners_with_overlay_registry` snapshot/process/writeback plus global movement | DRIFT (Checkpoint C) | migrate one live miner transaction |
| Gate request | mover obstacle check | mover check | MATCH for request placement only | preserve while changing surrounding host |
| Gate progression | gate's ordered object turn | all gates after all movers | DRIFT | route through gate Techno/Mission slot |
| War-factory contact creation/use | producer/radio caller and live relation | spawn-time relation plus a per-mover map frozen across that mover's budget | UNCHECKED locally; DRIFT for frozen lifetime | retain proven creation; live-read and inline breakup |
| War-factory contact breakup | current Unit per-cell call | global post-movement scan | DRIFT | move inline before further entries |
| Stock BFRT wall crush | each fully entered cell in Unit per-cell call | final-cell post-movement scan | DRIFT | move sound/DestroyOverlay/rock inline |
| Rust wall backing-entity teardown | destroyed wall entity follows canonical lifecycle after Rust wall scan | routes through `Simulation::uninit` | MATCH bounded cleanup / DRIFT owner timing | retain lifecycle routing under inline native wall owner |
| Alternate wall predicate / full `DestroyOverlay(-1)` equivalence | separate wall/overlay system | shared Rust wall teardown | UNCHECKED | audit alternate ability callers and every overlay side effect |
| Crush/wall sound-event creation | current object, native encounter order | crush delayed/sorted; wall sound absent | DRIFT | enqueue at verified effect point without ID sort |
| Crush sound contents | bounded native point proves the crush cue | Rust may emit both CrushSound and DieSound | DRIFT | reproduce exact event set and order |
| Mixer/device presentation | downstream service after event creation | upper-layer consumption | UNCHECKED | Checkpoint E/runtime audio scope |
| Independent alive/limbo/Mark/Logic/storage axes | native lifecycle stages remain independently observable | `ObjectLifecycle` and Logic/storage are independently represented | MATCH bounded | save/load reconstruction remains UNCHECKED |
| Reveal coordinate/Mark/register order | current Reveal caller; Mark success before eligible tail append | `try_reveal_entity` has the bounded ordered transaction | MATCH bounded | retain order; complete admission/eligibility and effects |
| Reveal admission/type/placement oracle | native caller/type/mode gates | caller-supplied result; convenience unlimbo assumes Mark success and eligibility | DRIFT | implement exact gates; Mark/cell full equivalence remains UNCHECKED |
| Techno BREAK core | ascending sparse-slot reread, sender clear, synchronous dispatch before Conceal | same represented core in `radio/mod.rs` | MATCH bounded | retain |
| BREAK receiver/subclass effects | class effect before common receiver clear | incomplete Building/refinery/other receiver coverage | DRIFT | complete receiver census and exact effects |
| Conceal state/output order | unmark/display/anim/Voc/live removal/limbo/redraw | ordered state and output emission in lifecycle authority | MATCH bounded | retain; dirty/type exactness UNCHECKED |
| LifecycleOutput consumers | apply ordered display/audio/animation effects | app drains every variant as no-op | DRIFT | implement consumers without changing sim order |
| UnInit represented core | class prework/passengers, Limbo/BREAK/Conceal, alive clear, duplicate queue | central authority represents this bounded tail | MATCH bounded | retain |
| UnInit detach/listeners/class hooks | exact derived prework and `Detach_From_All_Lists` listener census | partial hooks plus test-only notification boundary | DRIFT / UNCHECKED | complete named ground listener/manager coverage |
| Carried-passenger UnInit | passenger recursion before carrier tail | represented cargo recursion | MATCH represented subset | audit Capture/Temporal/other managers separately |
| Physical pending-delete drain | one late Main-Tick service after one authoritative-frame increment | one ordinary drain after Rust's projected `binary_frame` assignment; most 45-Hz ticks retain the same value | MATCH bounded relative placement / DRIFT frame cadence | retain one drain, replace/align authoritative frame cadence, and keep concrete destructor/intervening-tail blockers explicit |
| App animated-death UnInit | lifecycle call must reach current tick's native placement | call occurs after ordinary drain | DRIFT | route before the one drain or prove native class-specific timing |
| Live scheduler compaction/append | current pass sees first-match removal/tail append immediately | LogicVector and `for_each_live_object` already serve object-AI/Anim; Phase-1 movement still uses snapshots | MATCH mechanism / DRIFT movement integration | make the existing live scheduler the production movement authority |
| `0x004D94B0` and class arrival wrappers | normal Foot internal destination plus class wrappers/current object arrival | global pending-arrival sweep | DRIFT; wrapper family UNCHECKED | migrate proved normal owner; block class-specific claims pending wrapper audit |

No row is omitted because it appears data-structural or currently has no visible
fixture. Frequency affects priority, not parity verdict.

## 6. Required Ownership Contract

The eventual Rust-native implementation must retain the current bounded
lifecycle authority and satisfy all of these constraints at one atomic
production boundary:

1. One live ordered object scheduler remains the gameplay authority.
2. Each ground locomotor and its class-owned phase runs inside that object's
   verified slot and precedence from Checkpoint C.
3. Every current-object movement event commits occupancy, cell attributes,
   contact, sound-event creation, wall state, victim lifecycle, NavCom, and
   completion state before the next native-equivalent object slot.
4. Movement-caused lethal teardown calls the canonical lifecycle authority;
   direct entity-store removal is forbidden.
5. Conceal and removal from live Logic occur synchronously; physical free alone may be
   queued for the verified late drain.
6. Cell/list/cache views consumed by a later object are rebuilt or invalidated
   before that object. `OccupancyGrid::generation` is a possible Rust-native
   mechanism, not parity evidence by itself.
7. Native encounter/list order controls victims, sound events, callbacks, and
   branch-dependent Scenario RNG. No stable-ID sorting may replace it.
8. Gate request remains mover-time, while gate progression runs only in the
   gate's own ordered object slot.
9. War-factory contact breaks at the first verified Unit per-cell point, before
   any later same-invocation cell entry.
10. Wall crush fires at each verified fully entered cell, even when the mover
    leaves that cell again in the same Drive budget.
11. Arrival/finalization is not front-loaded or delayed globally; it belongs to
    the verified current/next Drive invocation.
12. The old bulk/postlude callers are removed for every migrated category in the
    same production change so no effect can run twice.
13. Unit/Infantry Tube handling preserves branch-specific `+0x174`, `+0x18C`,
    and `+0x544` order plus the ungated class suffix through `+0x4A0`; no
    invented alive/limbo early return may suppress the suffix.
14. The normal and direct-`+0xF8` Infantry crush branches remain distinct, and
    the saved-next victim walk does not add a per-victim crusher-state abort.
15. The common owner applied/current speed fraction is not a Drive-only field:
    blocked Tube exits write exact positive zero, Unit completion writes one
    after `+0x18C`, Infantry ordinary completion writes one before `+0x18C`, and
    the Infantry `+0x174` completion arm preserves its prior value. No generic
    movement pass may overwrite that result later in the same object turn.
16. The bounded `+0x4A0` discovery, visibility, unconditional radar-surface
    rectangle query, radar-event, tracker remove/cache-write/add, and
    periodic-dirty order remains after the Tube leaf. Production activation
    also requires closure of the full `+0x324` visibility family and downstream
    tracker/pixel equivalence named as blockers below.

This contract is semantic. It does not require C++ inheritance, COM plumbing,
raw pointers, or literal native vector storage.

### 6.1 Population-by-population atomic boundary

| Population/branch | New sole production owner | Old authority that must be absent in the same build |
|---|---|---|
| Unit Drive | Unit -> Foot -> Drive in live object slot, including D effects | snapshot generic vehicle mover; global `drive_reaims`; pending-arrival sweep; formation/finalizer/phases; sorted crush/postludes |
| Infantry Drive/custom binding | Infantry -> Foot -> active locomotor in live slot | target-based generic translation, snapshot mover authority, or skipped Infantry host |
| Infantry Walk | Infantry -> Foot -> Walk, unless active Tube preempts | generic walking translation and global phase owners |
| Unit Hover | Unit -> Foot -> Hover, including vertical/wake work | generic XY movement plus global Hover vertical tail |
| Unit Ship | Unit -> Foot -> Ship, including wake scan | generic target translation, global phase owner, stable-ID Ship wake scan |
| Ground Teleport | Unit/Infantry -> Foot -> Teleport plus verified restore points | `tick_teleport_movement` and `tick_locomotor_piggyback_restore` global callers |
| CMIN/other miners | current miner mission then its active locomotor in the same slot | `tick_miners_with_overlay_registry` snapshot/process/writeback and later global movement bridge |
| Active Unit/Infantry Tube | class Tube leaf; exact completion/blocked `+0x174/+0x18C/+0x544` branches; ungated `+0x4A0` direct effects for the whole preempted turn | `tick_low_bridge_tube_movement`, missing common speed/effect state, and forced/ordinary locomotor double-run |
| Forced Drive states | exact caller initializes; later ordered Drive consumes | `tick_forced_drive_tracks` initializer/consumer bridge |
| Movement-caused crush lifecycle | current Unit per-cell encounter order through canonical UnInit | sorted/deduped `crush_kills`, `pending_lifecycle_requests`, post-movement request drain, and `contains_crush_victim` skip bridge |
| Gameplay cache views | current object mutates; each later consumer reads equivalent live state | one-time `blocker_neighbor_counts` and stale/split `entity_block_sets` prepasses |
| Gate objects affected by movers | their own Techno/Mission slots | after-all-movers gate sweep |

The cutover cannot be staged as “new Drive plus handled-ID skip.” That would
leave Walk/Hover/Ship/Teleport/miner/Tube/forced objects and cross-object effects
under different schedulers, exactly the ordering defect this checkpoint closes.

### 6.2 Full crush-to-drain frame trace

The required normal-Infantry frame-visible mechanism is:

1. At native frame value N, crusher's object slot reaches Unit per-cell work.
2. In CellClass encounter order it saves victim `next`, creates crush sound,
   runs record/kill and `+0xE0` callbacks, and explicitly Mark-removes the victim.
3. The first Infantry `+0xD4` broadcasts ordered BREAK, conceals/removes from live Logic,
   and sets InLimbo. `+0xF8` then reaches generic UnInit: detach/listeners observe
   an alive but already-unmarked/already-limbo victim; generic UnInit invokes the
   second derived `+0xD4`, clears alive, and enqueues pending delete.
4. The crush helper continues from saved `next` without a per-victim crusher
   alive/limbo test. The live Logic vector is already compacted before the
   crusher returns. A later surviving object observes no victim occupancy,
   contact, or live membership while the global frame is still N.
5. Network service and the normal Main-Tick increment later commit frame N+1;
   `0x0055E160` then runs.
6. The pending-delete drain at `0x0055DE9F` physically destructs/frees the
   already-dead common victim before the following `0x00637270` tail call.

The direct-`+0xF8` Infantry branch has a different prefix: field copy and crusher
callbacks lead directly to victim UnInit. Its detach/listener boundary may see
the still-marked, not-yet-limbo victim; UnInit's derived `+0xD4` performs the
only Limbo/conceal call before alive clear. Both traces converge only after the
branch-specific lifecycle prefix.

Two scheduler placements must be distinguished. If the victim's Logic slot is
after the current cursor, removal compacts it out before its turn and the next
survivor remains reachable. If the victim's slot is before the current cursor,
removal shifts the current/following members left; the outer unconditional
increment can skip the immediate successor. A frozen movers vector cannot model
either outcome by delaying the kill to the end of movement.

## 7. Tiny-Detail Ledger

| Detail | Result |
|---|---|
| Plan's `0x741700` crush entry | corrected to body entry `0x7416A0` |
| Unit per-cell versus crush helper | distinct slots `+0x18C` and `+0x534` |
| Crush traversal | native cell-list order, saves next before victim mutation |
| Crusher lifecycle during victim chain | saved-next loop has no per-victim crusher alive/limbo recheck |
| Normal Infantry crush | explicit Mark-remove and first Limbo precede UnInit detach; generic UnInit invokes derived Limbo again |
| Direct-`+0xF8` Infantry branch | copies `+0x41A`, runs crusher `+0xDC/+0x3D4`, then direct victim UnInit; no normal sound/record/Mark/Limbo prefix |
| Normal crush RNG | none in `0x7416A0` normal path |
| Scatter RNG | Cell helper none; occupant callback consumes conditional Scenario draws |
| Sound order | crush sound before victim cleanup; wall sound before overlay destruction/rocking |
| Cell-list insertion | mobiles prepend; buildings append |
| Multi-cell recalculation | immediate per footprint cell |
| Accepted-cell callback death | owner `+0x90` failure at `0x4B1989` exits before destination Mark and per-cell suffix |
| Current-object death after per-cell | Drive performs immediate post-hook alive/limbo/state gates |
| Later-object removal | removed before slot; no turn |
| Current/earlier removal | shifted successor can be skipped by live loop increment |
| Tail append | can run in the same pass because count is re-read |
| UnInit storage lifetime | physical allocation remains only until the one late drain within the current Main Tick |
| Unit `+0x480` | wrapper `0x741970`, not direct `0x4D94B0` |
| Dynamic NavCom refresh | proved branch is Unit owner following Infantry target, not generic mover-to-mover |
| Infantry Tube completion | equal arm calls `+0x174` and preserves prior applied fraction; ordinary arm calls `+0x544(1.0)`; both clear Tube state, call `+0x18C`, then ungated `+0x4A0` even after live-Logic removal |
| Unit Tube completion | optional `+0x174`, then `+0x18C`, exact `+0x544(1.0)`, then wrapper's ungated `+0x4A0` |
| Unit/Infantry Tube blocked exit | occupant `+0x174` callbacks -> owner `+0x544(+0.0)` while Tube state remains -> ungated wrapper `+0x4A0`; no completion `+0x18C` |
| Tube common speed state | owner-wide Techno `+0x578/+0x57C`, not Drive target/current storage; shared body clamps NaN/negative/zero/positive/infinity exactly |
| Tube `+0x4A0` event gate | type-5 call requires nonzero visibility out-code and visible; suppressed by Tube sign only when `+0x14` bit 2 is set |
| Tube outer effect order | unconditional radar-surface rectangle query occurs before event eligibility; optional `+0x498` removes old coordinate -> unconditional `+0x208/+0x20C` writes -> optional `+0x494` adds new coordinate -> independent periodic `+0x49C` dirty |
| Concealed Tube owner | wrapper still calls `+0x4A0`; concrete `+0x324` is false, so no type-5 call or tracker add; cache writes and independently gated dirty remain |
| Gate request return | request can start mission while current entry remains blocked |
| Gate progression | gate object slot, not mover postlude |
| Factory row privilege | Rust reads live contact when building a per-mover map, but the contact-derived entry remains frozen through that mover's budget and cleanup is global |
| Wall traversal | fully entered-and-left wall must still be destroyed |
| Rust sound buffer | may remain a presentation handoff only if enqueue order is exact |

## 8. INI and Asset Integration

This investigation adds no new INI parser mapping. Existing type predicates,
gate definitions, factory row counts, overlays, sounds, locomotor selection, and
crushability remain data-driven from active YR rules/art/assets. The native
owner/order proved here must not be replaced with hardcoded stock object IDs,
sound names, wall frames, or footprint sizes.

Checkpoint C remains the authority for the ground population and locomotor
defaults. Exact supporting reports are
`docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`,
`docs/research/WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`,
`docs/research/BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`,
`docs/research/WALL_CRUSH_ON_DRIVEOVER_GHIDRA_REPORT.md`, and
`docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`; their frozen hashes are listed in
Section 14. This report closes where the scoped effects execute relative to the
ordered object pass, not every data predicate inside those subsystems.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Live Logic-vector iteration and count re-read | verified | `0x0055AFB0`; `0x55B5FF..0x55B61B` | native gameplay scheduler is live, not snapshotted |
| Logic-vector compaction/removal | verified | `0x0055BAE0` | exact later-object/shifted-successor effects established |
| Drive accepted-cell remove/coord/OnBridge/callback/survivor-Mark order | verified | fresh `disassemble_bytes(0x004B1840..0x004B19A5)` | runtime fixture for callback-caused owner death before new Mark |
| Unit `+0x18C` contact/wall then `+0x534` crush order | verified | Unit COL/slots; `0x00739EC0` | effects complete before Foot tail/next object |
| Unit crush helper boundary | verified | `get_function_by_address(0x741700)`; body `0x007416A0` | stale mid-body anchor corrected |
| Crush list order/gates/saved-next loop | verified | fresh `0x00741840..0x00741935` disassembly | executable two-victim fixture; no per-victim crusher-state abort |
| Normal crush sound/callback/Limbo/UnInit order | verified | `0x7418AA..0x741910`; fresh callees | receiver/listener census remains external |
| Direct-`+0xF8` Infantry crush branch | verified | `0x741853..0x74188D` | runtime fixture for still-marked detach boundary |
| Infantry concrete normal lifecycle chain | verified | Infantry COL/slots and `0x4D3780/0x51DF10/0x4DE5D0/0x5F65F0` | runtime fixture for double-`+0xD4` state trace |
| Scatter enumeration and RNG ownership | verified | `0x00481670`; Unit `+0x174 -> 0x00743A50` | per-occupant conditional Scenario draws in list order |
| Reveal coordinate/Mark/register order | verified | fresh `0x005F4EC0`; assembly `0x5F4F4A..0x5F5040` | Mark succeeds before eligible tail registration; failure restores limbo |
| Techno Limbo ordered BREAK before Conceal | verified | fresh `0x0065AA80/0x0065ACE0/0x0065A970` | ascending contact slots; sender clear before synchronous receiver |
| BREAK receiver/subclass effects | touched-not-exhausted | binary receiver bodies; current `radio/receive.rs` | complete Building/refinery/other receiver census and effects |
| Object UnInit generic ordering | verified | fresh `decompile_function(0x005F65F0)` | caller-specific marked/limbo state must remain explicit |
| Conceal occupancy/display/sound/live removal | verified | `0x005F4D30`; `0x0055BAE0` | gameplay removal synchronous |
| Pending-delete drain and Main-Tick placement | verified | `0x00725C70`; `0x005F6690`; `0x55DC9E/0x55DE9F` | common object physically freed at the same-Main-Tick late drain |
| Current Rust authoritative frame/drain cadence | verified | `world/mod.rs:2029..2044`; `fixed_math.rs`; `app_types.rs` | assignment-before-drain relative order matches, but 45-Hz projected 15-Hz value often does not increment; DRIFT versus native per-Main-Tick counter |
| Cell add/remove/list order/recalculation | verified | `0x5683C0/0x5687F0/0x47E8A0/0x47EA90/0x47D2B0` | immediate later-object visibility |
| Dynamic NavCom refresh | verified | fresh `0x4B05D0..0x4B0640`; `0x4B0971..0x4B09A3` | proved Unit-owner/Infantry-target branch only |
| Track completion/finalization | verified | `0x4B210E..0x4B228B` | current Drive invocation |
| Empty/queued arrival owner | verified | `0x004B0500`; `0x004DF0D0`; `0x004D82B0` | same object's next eligible Drive slot |
| Concrete Unit destination wrapper identity | verified | Unit vtable slot read `0x7F60F0 -> 0x741970` | stale direct-Foot binding rejected |
| Foot internal destination `0x004D94B0` | touched-not-exhausted | direct body and class-binding comparison | exhaustive derived wrapper/caller mapping remains |
| Class-specific arrival wrappers | deferred | normal Unit/Foot owner closed; wrapper family sampled | separate arrival-system audit before class-wide parity claim |
| Unit/Infantry Tube completion/blocked call schedule | verified | fresh `0x51BA60..0x51BAD8`, `0x736000..0x736065`, `0x7363A0..0x7363C5`; class COL/slots | executable exact branch-order and conceal-inside-`+0x18C` fixtures |
| Shared Tube `+0x544` clamp and branch arguments | verified | `0x004D3710`; Unit `0x736047..0x73604F`; Infantry `0x51BA6F..0x51BA81`; blocked callsites | implement common owner field and zero/one/preserve outcomes without same-tick overwrite |
| Tube `0x0070D990` bounded outer control flow/field writes | verified | fresh body disassembly including `0x70DA48..0x70DA69`; Unit/Infantry `+0x2C/+0x324/+0x494/+0x498/+0x49C/+0x4A0` slot reads | implement discovery, unconditional rectangle query, event-call gate, remove/cache/add, and periodic-dirty order; full nested visibility remains separate |
| Concrete Tube `+0x494/+0x498/+0x49C` small bodies | verified | `0x0070CC90/0x0070CCC0/0x0070CCF0` | tracker add/remove and dirty callbacks use new/old cache at exact points |
| Full `+0x324` visibility predicate and tracker/pixel equivalence | touched-not-exhausted | `0x0070D1D0..0x0070D410`; tracker helpers `0x655560/0x655740` sampled | exhaust discovery/owner/alliance/ability/shroud/height/cloak/sensor/out-code branches and downstream consumers |
| Current clean-Rust Tube suffix effects | verified | `tube_movement.rs`; `components.rs`; `radar.rs`; app lifecycle/visibility reads | DRIFT: no common fraction transaction, discovery/tracker fields, type-5 producer, or native cache order |
| Gate request ownership | verified | `0x00578AD0`; `0x00452540` | mover-time mission request, current call still blocked |
| Gate progression ownership | verified | `0x006FA5C6..0x006FA655`; `0x0044E440` | gate object's ordered turn |
| Factory contact creation/initial row use | verified | factory report SHA `E86C7A0C6AC9805FDAD53B8C63E87382521B66318912640680F607D889F41FE7`; live binary/source audit | Rust initially reads live relation when building its per-mover map |
| Factory contact breakup | verified | `0x00739EC0`; `0x006F4AB0` case 8 | current Unit per-cell call |
| Rust contact-derived factory-map lifetime | verified | `movement_tick.rs:1048..1049`; global contact cleanup source | DRIFT: entry remains frozen through the mover budget and contact cleanup remains global; unrelated gate/repair/bunker/bib/C4 map entries are outside this correction |
| Stock BFRT wall reachability and sound/destroy/rock ordering | verified | `WALL_CRUSH_ON_DRIVEOVER_GHIDRA_REPORT.md` SHA `A6C68D...F778`; `0x73B04D..0x73B06E` | current Unit per-cell call |
| Alternate weapon predicate / full `DestroyOverlay(-1)` equivalence | deferred | stock branch closed; `0x00480CB0` touched | separate wall/overlay-system audit |
| Current-HEAD bulk/postlude inventory | verified | clean `cbf4d871...` source blobs in Section 14.2 | global owners enumerated and classified |
| Current-HEAD crush/canonical-lifecycle split | verified | `movement_tick.rs`, `movement_occupancy.rs`, `world/mod.rs`, `lifecycle.rs` | major timing/order DRIFT; no raw-store-removal claim |
| Current lifecycle authority delta | verified | commit `95bef99d`; lifecycle/radio/logic/app source reads | retain bounded matches; complete named blockers |
| Existing production live object-AI host | verified | `world/mod.rs:2088..2107`; clean HEAD `techno_ai.rs:282..320`; `for_each_live_object` | bounded MATCH: retain and extend this host rather than add a second scheduler |
| Phase-1 ground-movement live-host integration | verified | movement follows object-AI but uses `live_object_order_snapshot` plus a second mover vector | DRIFT: absorb the complete C population/effects into the existing live host atomically |
| Current blocker/cache split | verified | `movement_tick.rs:872..1040`; `bump_crush.rs:114..267`; `src/sim/occupancy.rs` | one-time counts and alive early-unmarked victim rebuild are DRIFT; broader native graph deferred |
| Current non-vehicle bridge and chain/cell local commit boundaries | verified | `movement_tick.rs:1494..1757`; `movement_occupancy.rs:484..705` | local staging classified; resulting lifecycle crossing remains DRIFT |
| `DeferredCellCheck::Vehicle` pending bridge propagation | deferred | handler receives no `pending_bridge_update` | D-OQ-31 requires native/Rust clear-and-blocked crossing audit |
| Current lifecycle request and handled-ID skips | verified | `movement_tick.rs:1775..1996`; `world/mod.rs:2139..2144` | canonical authority destination matches; post-movement timing/skip bridge drift |
| Current LifecycleOutput consumers | verified | `src/app_sim_tick.rs:619..630` | every output variant currently drains as no-op |
| Current deferred/global owner-or-blocker matrix | verified | Section 5.4 plus approved D-plan actions | every named family is MATCH/DRIFT/UNCHECKED with disposition |
| Atomic full-population caller-removal boundary | verified | Section 6.1 plus Checkpoint C | mechanically enumerable; no handled-ID bridge |
| Every `Detach_From_All_Lists` listener side effect | touched-not-exhausted | `0x007258D0` body/callees | requires separate listener/system census |
| Every native pathfinder/zone/cache invalidation | touched-not-exhausted | immediate `0x0047D2B0` cell recalculation proved | broader cache graph not exhausted |
| Executable object-order/sound oracle | deferred | static mechanism only | Checkpoint E runtime work |
| Save/load lifecycle/Tube reconstruction | deferred | outside bounded D mechanism slice | separate persistence scope; gates cutover if serialized fields/reconstruction change or in-flight Tube save/load remains supported |
| Non-ground death families | deferred | outside bounded ground population | separate class-system scope while shared lifecycle behavior/callers remain unchanged; otherwise regression closure gates cutover |

### 9.1 Zero-add, adversarial, and cold-spot pass

The zero-add pass re-enumerated the approved Phase-4 list directly from the
investigation plan: NavCom refresh; Tube/forced work; blocker/cache build and
refresh; pending arrivals; point/cell/occupancy; deferred chain/occupancy;
scatter/crush; formation; finished finalization; class phases; Hover vertical;
gate; factory contact; and wall crush. Reveal, Techno BREAK, Conceal, UnInit, and
the physical drain were then added from the lifecycle-authority input because
crush reaches them. The adversarial pass added accepted-cell callback death, the
direct-`+0xF8` Infantry crush branch, Tube lifecycle/post-suffix behavior,
Tube common speed state and the substantive `+0x4A0` effect transaction,
LifecycleOutput consumers, save/load/non-ground scope dispositions, and the
exact current-HEAD request drain. Every
named item appears in Section 5.4 and has an explicit `MATCH`, `DRIFT`, or
`UNCHECKED` verdict or an explicit external blocker.

The adversarial pass tested the strongest alternative explanations:

- identical final state cannot excuse different later-object visibility;
- the physical-delete tail cannot own earlier conceal/occupancy/contact effects;
- a stable-ID order cannot substitute for CellClass or LogicVector order;
- a generation counter cannot certify cache equivalence without consumer proof;
- a synchronous radio-contact ID clear cannot substitute for ordered BREAK
  receiver dispatch;
- canonical UnInit called after all movers cannot substitute for canonical
  UnInit inside the current victim encounter;
- a concealed Tube owner does not authorize suppressing the ungated class
  suffix; and
- a handled-ID bridge cannot preserve one scheduler when categories interact.

Two independent read-only evidence passes separately covered
lifecycle/crush/occupancy and arrival/gate/factory/wall. The root pass cold-spotted
the Unit and Infantry COL/slots, accepted-cell lifecycle gates, multi-cell
add/remove/recalculation, live-vector compaction, pending-delete placement, and
fresh Reveal/BREAK bodies. The corrected exact snapshot is released only after
the mechanical/schema pass and independent exact-hash cold reviews described at
handoff time; this prose is not itself a parity certificate.

## 10. Open Questions — Final State of the Investigation Log

- `[RESOLVED] D-OQ-01 — Are accepted-cell occupation and per-cell callbacks delayed to a movement tail? → No. They occur inside current Process_Drive_Track before its
   survivor gates.` (evidence: `0x004B17C8..0x004B1D2E`)
- `[RESOLVED] D-OQ-02 — Is Unit wall/contact/crush work one helper? → No. Unit +0x18C @ 0x00739EC0 owns the per-cell ordering and calls distinct +0x534 @ 0x007416A0.` (evidence: Unit COL chain `0x007F5C6C -> 0x0080CC68 -> 0x00842D80`; slots `0x007F5DFC -> 0x00739EC0` and `0x007F61A4 -> 0x007416A0`; fresh bodies)
- `[RESOLVED] D-OQ-03 — Is 0x00741700 a safe crush-function anchor? → No; it is inside the
   function beginning at 0x007416A0.` (evidence:
   `get_function_by_address(0x00741700)`)
- `[RESOLVED] D-OQ-04 — May normal crush deaths be sorted by stable ID? → No. Native uses
   current CellClass list order and saves next before unlink.` (evidence:
   `0x007416A0`; `0x0047E8A0`; `0x0047EA90`)
- `[RESOLVED] D-OQ-05 — Does normal crush consume RNG? → No RNG call was found in the normal
   helper body; scatter RNG belongs to occupant callbacks.` (evidence:
   `0x007416A0`; `0x00481670`; `0x00743A50`)
- `[RESOLVED] D-OQ-06 — Is movement-caused victim gameplay state left live until the delete drain? → No. Occupancy, conceal, live-Logic removal, and alive state change
   synchronously; only physical free waits.` (evidence: `0x005F65F0`;
   `0x005F4D30`; `0x0055BAE0`; `0x00725C70`)
- `[RESOLVED] D-OQ-07 — Does common UnInit require an additional full simulation tick before physical free? → No. IsDead is true after alive clear and the one late
   drain can free it in the same Main Tick.` (evidence: `0x005F6690`;
   `0x00725C70`; `0x0055DE9F`)
- `[RESOLVED] D-OQ-08 — Does a later object see changed cell content and attributes? → Yes;
   add/remove and recalculation are synchronous per footprint cell.` (evidence:
   `0x005683C0`; `0x005687F0`; `0x0047D2B0`)
- `[RESOLVED] D-OQ-09 — Is dynamic NavCom refresh a global prepass for arbitrary movers? → No. The proved branch belongs to a Unit following an Infantry target in the Unit's current Drive Process.` (evidence: fresh `0x4B05D0..0x4B0640`; `0x4B0971..0x4B09A3`)
- `[RESOLVED] D-OQ-10 — Is track completion a global finished-entity tail? → No. Native
    clears track state and runs per-cell/finalization work in the current Drive
    invocation.` (evidence: `0x4B210E..0x4B228B`)
- `[RESOLVED] D-OQ-11 — Are all pending arrivals processed before movers? → No. Each object
    reaches its empty/queued arrival path in its own next eligible Drive slot.
   ` (evidence: `0x004B0500`; `0x004DF0D0`; `0x004D82B0`)
- `[RESOLVED] D-OQ-12 — Does a gate request advance every gate after movement? → No. The
    mover requests; the gate advances only in its own object turn.` (evidence:
    `0x00452540`; `0x006FA5C6..0x006FA655`; `0x0044E440`)
- `[RESOLVED] D-OQ-13 — Is war-factory contact cleanup a global postpass? → No. The current
    Unit breaks it during per-cell processing.` (evidence: `0x73A93D`;
    `0x006F4AB0`)
- `[RESOLVED] D-OQ-14 — Is wall crush based only on the mover's final cell? → No. It fires
    at the verified fully-entered-cell per-cell point.` (evidence:
    `0x73B04D..0x73B06E`)
- `[RESOLVED] D-OQ-15 — Is any scoped gameplay effect proved to belong to a global movement tail? → No. The only proved late service is physical delete/free
    after synchronous gameplay removal.` (evidence: current-object anchors
    `0x004B0F20`, `0x00739EC0`, `0x004D3710`, `0x0070D990`, and `0x0044E440`;
    lifecycle/drain anchors `0x005F65F0` and `0x00725C70`)
- `[RESOLVED] D-OQ-16 — Can movement/lifecycle Reveal register an object before its coordinate and Mark state commit? → No. Reveal commits coordinate, requires
    successful Mark, and only then performs eligible Logic tail registration;
    Mark failure restores InLimbo.` (evidence: fresh `0x005F4EC0` decompile and
    `0x005F4F4A..0x005F5040` assembly)
- `[RESOLVED] D-OQ-17 — Can Techno Limbo clear contacts as an unordered ID sweep after Conceal? → No. It broadcasts BREAK in ascending contact-slot order before
    common Conceal; each send clears sender matches before synchronous receiver
    handling.` (evidence: fresh `0x0065AA80`, `0x0065ACE0`, `0x0065A970`)
- `[RESOLVED] D-OQ-18 — Can the destination-cell callback walk kill the mover before the new Mark and per-cell suffix? → Yes. The +0x90 failure branch at 0x4B1989 goes straight to the epilogue, bypassing +0x124(1) and +0x18C.` (evidence: fresh `disassemble_bytes(0x004B1840..0x004B19A5)`)
- `[RESOLVED] D-OQ-19 — Do both scoped Infantry crush branches share sound, explicit Mark-remove, and explicit pre-UnInit Limbo? → No. The 0x741853..0x74188D branch performs field copy and crusher callbacks, then direct victim +0xF8.` (evidence: fresh `disassemble_bytes(0x00741840..0x00741935)`)
- `[RESOLVED] D-OQ-20 — Does the crush helper recheck crusher alive/limbo before following each saved victim link? → No. 0x74191D/0x74191F tests only saved-next non-null and loops.` (evidence: fresh `0x00741840..0x00741935` disassembly)
- `[RESOLVED] D-OQ-21 — Can a Tube virtual callee mutate lifecycle before the class suffix, and do completion/blocked wrappers still run the suffix call? → Yes. Infantry garrison completion reaches Limbo/Conceal inside +0x18C, then the wrapper still calls ungated +0x4A0; Unit completion runs +0x18C, +0x544, and ungated +0x4A0; blocked Unit/Infantry exits run occupant scatter callbacks, owner +0x544(+0.0), and ungated +0x4A0 without completion +0x18C. This resolves Checkpoint-C OQ-24 only for call scheduling.` (evidence: `0x51B350`; `0x51BA9B`; `0x51972D`; `0x522931`; `0x51BACD`; `0x7359F0`; `0x73603F`; `0x73604F`; `0x7363BB`; Unit/Infantry COL slot reads)
- `[RESOLVED] D-OQ-22 — Does current clean Rust still raw-remove crush victims instead of using canonical UnInit? → No. It queues LifecycleRequest::Uninit and applies central Simulation::uninit, but only after the complete movement call, so same-object and later-object visibility remain DRIFT.` (evidence: clean HEAD `movement_tick.rs:1775..1800`; `world/mod.rs:2139..2144`)
- `[RESOLVED] D-OQ-23 — Does clean Rust's assignment-before-drain order prove authoritative frame parity? → No. The assignment precedes the drain, but 45-Hz/22-ms ticks project (total_sim_ms * 15) / 1000, so most drains retain N while native increments once per normally reached Main Tick. Relative call placement is a bounded MATCH; frame cadence/value is DRIFT.` (evidence: clean HEAD `world/mod.rs:2029..2044`; `src/util/fixed_math.rs`; `src/app_types.rs`; native `0x0055DE9F`; scheduling report SHA in Section 14.1)
- `[DEFERRED] D-OQ-24 — What are the exact effects and order of every
    Detach_From_All_Lists listener for every derived ground class?` (category: `requires-different-system-context`; reason: the bounded Infantry-crush chain
    proves the required scheduling boundary but not every registry/listener
    family; next-step-if-pursued: build a receiver/listener census rooted at
    `0x007258D0` and each ground class `+0xD4/+0xF8` binding)
- `[DEFERRED] D-OQ-25 — Which exact native route/zone/path caches correspond to every
    Rust blocker and OccupancyGrid::generation consumer?` (category: `requires-different-system-context`; reason: immediate CellClass recalculation
    is proved, but the broader pathfinding graph is a separate subsystem;
    next-step-if-pursued: trace every consumer of the affected CellClass
    attributes and compare it with each Rust cache read)
- `[DEFERRED] D-OQ-26 — Do retail mixer presentation time and concrete IDs reproduce the
    statically proved sound/object-order fixtures?` (category: `needs-runtime-debugger`; reason: static code proves event creation order, not
    device/mixer presentation timestamps; next-step-if-pursued: execute the
    Checkpoint-E oracle with object IDs, RNG cursor, event sequence, and time)
- `[DEFERRED] D-OQ-27 — What is the exhaustive Unit destination-wrapper behavior for
    every Teleport/docking subclass and every OnArrival +0x687 producer?` (category: `out-of-scope`; reason: D needs the normal Unit/Foot arrival owner,
    not every class-specific arrival mission; next-step-if-pursued: investigate
    the wrapper/producer family as a separate arrival-system report)
- `[DEFERRED] D-OQ-28 — Does every alternate/custom weapon-predicate branch and every DestroyOverlay(-1) effect match the current Rust
    shared wall teardown?` (category: `requires-different-system-context`; reason: stock BFRT reachability and owner/timing are closed, while alternate predicates plus full overlay connectivity/animation cleanup span the wall/overlay subsystem;
    next-step-if-pursued: audit `0x00480CB0` and its active wall callers against
    the Rust overlay mutation path)
- `[DEFERRED] D-OQ-29 — How should save/load reconstruct objects inside each lifecycle
    stage?` (category: `out-of-scope`; reason: persistence was not part of the
    atomic in-frame ownership question; next-step-if-pursued: trace native
    Save/Load around active, limbo, and pending-delete states)
- `[DEFERRED] D-OQ-30 — What is the exact BREAK receiver/subclass effect census for every ground contact partner reachable during movement-caused Limbo?` (category: `requires-different-system-context`; reason: D proves broadcast and sender/receiver ordering, while Building GrandOpening, refinery cascades, and other class effects span the radio/mission subsystems; next-step-if-pursued: enumerate each active receiver binding and compare its pre-common-clear mutations with `src/sim/radio/receive.rs`)
- `[DEFERRED] D-OQ-31 — Does the Rust DeferredCellCheck::Vehicle branch correctly suppress a computed pending_bridge_update for every clear and blocked outcome?` (category: `requires-different-system-context`; reason: the non-deferred commit boundary is visible in D, while exact bridge-crossing acceptance and layer state belong to the bridge/occupancy seam; next-step-if-pursued: trace one native and Rust clear crossing plus each blocked result through OnBridge, bridge occupancy, Mark lists, and the next object read)
- `[RESOLVED] D-OQ-32 — What exact value/order does common Tube speed state use? → Shared 0x004D3710 clamps into Techno +0x578/+0x57C; blocked Unit/Infantry write positive zero, Unit completion writes one after +0x18C, Infantry ordinary completion writes one before +0x18C, and Infantry's +0x174 completion arm preserves its prior value.` (evidence: `0x004D3710`; `0x51B8F8..0x51B8FC`; `0x51BA6F..0x51BA9B`; `0x735F66..0x735F6A`; `0x736028..0x73604F`; class slots)
- `[RESOLVED] D-OQ-33 — Is Tube +0x4A0 merely an ungated suffix marker? → No. 0x0070D990 directly sequences discovery, visibility/out-code, an unconditional radar-surface rectangle query, a gated type-5 call, coordinate correction, tracker remove, unconditional cache writes, tracker add, and independent periodic dirtying; +0x494/+0x498/+0x49C bindings and small bodies are closed.` (evidence: `0x0070D990..0x0070DC37`; unconditional query `0x70DA48..0x70DA69`; Unit/Infantry COL and slot reads; `0x0070CC90/0x0070CCC0/0x0070CCF0`)
- `[DEFERRED] D-OQ-34 — What are the exhaustive live-object +0x324 visibility predicate and downstream tracker/pixel semantics reached by regular and blocked Tube owners?` (category: `requires-different-system-context`; reason: D verifies the direct `+0x4A0` sequence and concealed-owner branch, but `0x0070D1D0` contains substantial discovery/owner/alliance/ability/shroud/height/cloak/sensor/out-code logic and tracker helpers have broader consumers; next-step-if-pursued: exhaust `0x0070D1D0..0x0070D410`, `0x00655560`, and `0x00655740`, then compare every capacity/clamp/dirty/pixel consumer with Rust visibility/minimap state)
- `[RESOLVED] D-OQ-35 — Is Rust's war-factory privilege map built once before every mover? → No. It is rebuilt from live state per mover at movement_tick.rs:1048..1049, but its contact-derived entry is frozen through that mover's inner budget and the relation itself is cleaned only in the global postpass.` (evidence: clean HEAD `movement_tick.rs`; `production/war_factory_exit.rs`; factory report hash in Section 14.1)
- `[DEFERRED] D-OQ-36 — Do non-ground death families preserve their exact class-specific lifecycle effects if the shared Rust lifecycle authority changes for the ground cutover?` (category: `out-of-scope`; reason: Checkpoint D's atomic population is ground, so non-ground callers remain a separate parity scope only while their callers and shared lifecycle-visible behavior are unchanged; next-step-if-pursued: enumerate non-ground UnInit/Limbo/Conceal callers and run regression traces before any shared lifecycle change that can affect them)

There are no silent open questions inside the claimed owner/order scope.

## 11. Visual, UI, and Audio Ledger

| Surface | Native timing/result | Rust requirement | Status |
|---|---|---|---|
| Crush sound event | created before victim cleanup, in cell-list encounter order | enqueue before lifecycle mutation in identical encounter order; no ID sort | verified owner/order |
| Wall sound | created before `DestroyOverlay(-1)` and rocking writes | preserve event/state order at each fully entered cell | verified owner/order |
| Unit rocking | written during per-cell wall crush | cannot be omitted or delayed to end-of-movement scan | verified owner/order |
| Gate sound/animation state | gate mission/object turn | execute in gate slot, not after all movers | verified owner/order |
| Mixer/device presentation | downstream of event creation | must not perturb sim order/RNG; runtime timestamps need oracle | deferred to E |
| Selection/display removal on conceal | inside synchronous Conceal | remove before next object; presentation layer may consume ordered event/state later | verified owner |
| Current lifecycle output application | current Rust emits ordered outputs | `src/app_sim_tick.rs:619..630` discards every variant; implement display/audio/animation consumers | DRIFT |
| Tube suffix after in-leaf Conceal | class wrapper still calls `+0x4A0`; discovery may update first, concrete visibility is false, the radar-surface rectangle query still runs, no type-5 call or `+0x494` add follows, conditional tracker removal precedes unconditional cache writes, and local flash dirty remains independent | keep the suffix after lifecycle mutation; record rectangle query, `+0x41B/+0x423`, cache, event, and dirty results | verified bounded owner/order; full visibility family blocked |
| Tube type-5 radar event | unconditional radar-surface rectangle query precedes eligibility; call then requires nonzero out-code and visible; Tube sign suppresses only with object bit 2 set; callee may deduplicate | add a real `EnemyObjectSensed` producer at the exact suffix point and preserve query/event and signed `/256` packing order | DRIFT in current Rust; gate verified |
| Tube radar tracker/cache | remove old tracker through `+0x498`, always write `+0x208` then `+0x20C`, add new tracker through `+0x494`, then independently cadence-gate `+0x49C` | model exact membership/cache order; generic floating renderer cache is not equivalent | DRIFT; direct sequence/small bodies verified, downstream pixel equivalence blocked |
| UI/sidebar | no scoped direct owner found | no D-specific UI mutation required | not applicable |

## 12. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Live Logic iteration re-reads count; first-match removal compacts left and tail append can enter the same pass | `0x0055AFB0`; `0x0055BAE0` | existing `object_ai_stage` already uses the live helper for partial object AI, but following Phase-1 ground movement snapshots order and builds a second mover vector | `src/sim/world/mod.rs::advance_tick`; clean `src/sim/world/techno_ai.rs`; `src/sim/movement/movement_tick.rs`; existing `LogicVector` | extend the existing production live host to own ground locomotion and synchronous lifecycle visibility | victim-before/after-cursor and tail-Reveal fixtures record exact run/skip sequence | do not create a second live scheduler, use stable-ID snapshots, or retain a handled-ID bridge |
| Accepted Drive cell performs old Mark removal, coordinate/bridge update, destination callbacks, then survivor-gated new Mark and per-cell work | fresh `0x004B1840..0x004B19A5` disassembly | local move exists, but callback-caused owner death gate is not proved under the bulk host | `movement_tick.rs`; `movement_step.rs`; `movement_bridge.rs`; lifecycle seam | preserve both survivor and callback-death outcomes before next object | callback kills mover after old unmark: new Mark and `+0x18C` effects remain absent | do not make destination Mark unconditional |
| A computed bridge-state mutation must either commit at the accepted current-cell boundary or be proved inapplicable on a blocked crossing | native accepted-cell OnBridge ordering; current source split | non-deferred Rust applies locally; `DeferredCellCheck::Vehicle` suppresses the update and does not pass it to the handler | `movement_tick.rs:1494..1757`; `movement_occupancy.rs::handle_deferred_occupancy`; `movement_bridge.rs` | preserve local commit and resolve deferred clear/blocked propagation before cutover | clear and each blocked deferred-vehicle result record OnBridge, layer, occupancy list, and observer state | do not silently drop or always-apply the update without the branch proof |
| Unit-following-Infantry NavCom refresh reads target live inside the Unit's Drive Process | `0x4B05D0..0x4B0640`; `0x4B0971..0x4B09A3` | broader `drive_reaims` global snapshot runs before movers | `movement_tick.rs` reaim preparation; per-object Drive host | enforce exact Unit/Infantry gates and read target coordinate at follower slot | Infantry A moves before Unit B; only eligible B reaims to A's new coordinate | do not generalize this branch to every moving target |
| Normal crush follows CellClass order, saves next, does not recheck crusher per victim, and completes victim lifecycle synchronously; direct-`+0xF8` Infantry branch has a different prefix | `0x741853..0x74191F`; concrete Infantry COL/slots/callees | early unmark plus sorted/deduped tail sound/request; special branch absent | `movement_occupancy.rs`; `movement_tick.rs`; `bump_crush.rs`; `world/lifecycle.rs` | call existing canonical lifecycle inside encounter; model both branches and saved-next traversal | two victims; first callback conceals crusher; second still visited; separate direct-`+0xF8` state trace | do not sort IDs, abort on crusher state, or invent a common prefix |
| Scatter snapshots selected CellClass list and dispatches per occupant; Unit callback consumes branch-conditional inclusive RNG draws | `0x00481670`; Unit `+0x174 -> 0x00743A50` | one-blocker/eight-way helper plus pass-wide dedup | `movement_occupancy.rs`; `bump_crush.rs`; Scenario RNG | reproduce population, order, eligibility, and exact draw count/ranges | mixed occupant fixture records list order and full RNG state before/after | do not use a Rust-vs-Rust hash as native parity proof |
| Infantry `+0x18C` can remove the owner from live Logic, yet its wrapper still calls `+0x4A0`; Unit and blocked paths retain their exact branch order | `0x51B350`; `0x51BA9B/0x51BACD`; `0x7359F0`; `0x73603F/0x7363BB`; class COL/slots | global Tube pass has no class call/lifecycle transaction | `tube_movement.rs`; `movement_tick.rs`; existing per-object Unit/Infantry host; lifecycle/radio | keep branch-specific completion/blocked calls and ungated suffix in one preempted object turn | destination-garrison Infantry becomes InLimbo/absent from live Logic and still enters `+0x4A0`; blocked exit omits `+0x18C` | do not add an alive/limbo early return or run completion `+0x18C` on blocked exit |
| Common Tube applied/current fraction uses `+0x544`: blocked zero; Unit completion one after `+0x18C`; Infantry ordinary one before `+0x18C`; Infantry `+0x174` arm preserves prior value | `0x004D3710`; exact Unit/Infantry callsites and slots | no cross-Foot owner field or Tube zero/one/preserve transaction; generic Drive state can run later | `components.rs`; `tube_movement.rs`; `movement_tick.rs`; `drive_locomotion.rs`; `movement_step.rs` | add native-equivalent common state, exact ordering and canonical bits; prohibit same-tick generic overwrite after Tube wrapper | seed `0.375`; assert blocked `+0.0`, Unit one after lifecycle, Infantry ordinary one before lifecycle, and equal-arm preservation with full-state snapshots | do not map this only to Drive target/current fields or normalize away branch timing |
| Tube `+0x4A0` performs discovery, visibility/out-code, an unconditional radar-surface rectangle query, gated type-5 call, tracker removal, cache writes, tracker add, and independent periodic dirtying after the leaf | `0x0070D990`; `0x70DA48..0x70DA69`; Unit/Infantry `+0x2C/+0x324/+0x494/+0x498/+0x49C/+0x4A0` slots; small bodies | no discovery/tracker fields, type-5 producer, or native query/remove/write/add order; generic floating screen cache is non-equivalent | `game_entity.rs`; `components.rs`; `radar.rs`; `tube_movement.rs`; app visibility/minimap and lifecycle-output consumers | implement bounded direct order and concealed-owner outcome; close full `+0x324`/tracker/pixel blocker before activation | tracked visible move; concealed garrison matrix; live/blocked bit2-by-Tube-sign event matrix; record rectangle query/cache/tracker/event/dirty sequence | do not call the full visibility family verified, suppress/query-skip the suffix after conceal, infer rectangle-call side effects, or treat the enum/cache as parity |
| Gate request is mover-time, but progression/sound belongs to the gate's own ordered object turn | `0x00578AD0`; `0x00452540`; `0x0044E440` | requests occur during movement; all gates progress afterward | `movement_occupancy.rs`; `gate_runtime.rs`; Techno/Mission host | retain request point and route progression through gate object slot | compare `mover A -> gate -> mover B` with `gate -> mover A` | do not advance all gates after all movers |
| War-factory contact breaks synchronously in current Unit per-cell work before another same-budget cell entry | `0x73A93D`; `0x006F4AB0`; factory report SHA `E86C7A0C6AC9805FDAD53B8C63E87382521B66318912640680F607D889F41FE7` | live contact seeds a per-mover exception map, but its contact-derived entry is frozen through that mover budget and cleanup remains global | `movement_tick.rs:1048..1049`; `production/war_factory_exit.rs`; radio | preserve unrelated gate/repair/bunker/bib/C4 entries while making only contact-derived privilege live and dispatching breakup inline | Unit leaves footprint then attempts a second entry in one Drive budget; first map build sees contact, second entry does not | do not delete unrelated map semantics or keep contact privilege until a postpass |
| Stock BFRT wall crush creates sound, destroys overlay, and writes rocking state at each fully entered cell before generic crush | `0x73B04D..0x73B06E`; wall report SHA `A6C68D...F778` | final-standing-cell global scan; wall sound/rocking absent | `world/mod.rs::apply_wall_crush_on_driveover`; per-cell Unit path; overlay/audio output | move stock effect sequence inline; separately audit alternate predicates/full overlay cleanup | Unit enters and leaves wall cell in one budget; observer sees removal and ordered sound/rock | do not defer to final cell or claim alternate/full `DestroyOverlay` closure |
| Current Rust Reveal/BREAK/Conceal/UnInit/LogicVector authority matches several bounded order slices | `0x5F4EC0/0x65AA80/0x65ACE0/0x65A970/0x5F4D30/0x5F65F0`; `docs/research/BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md` SHA `3D295875F6FF1EA91B5730DD070040E05A856A9137B0ED2CA3DB011AAFB6F71C`; clean commit `95bef99d` | current Rust delta: none observed for named bounded slices; integration/listener/receiver coverage remains mismatched or unchecked | `world/lifecycle.rs`; `world/logic_vector.rs`; `radio/mod.rs`; `radio/receive.rs` | retain existing authority, invoke it in current object turn, complete detach listeners and receiver effects | full normal/direct crush traces plus Reveal tail append and BREAK receiver mutation | do not replace the authority or call it only after all movers |
| Conceal output boundaries are ordered gameplay/presentation obligations | `0x005F4D30`; current ordered output emission | app consumes all `LifecycleOutput` variants as no-ops | `src/app_sim_tick.rs`; render/audio/animation consumers | apply each output in emitted order without feeding nondeterminism back into sim | conceal fixture observes selection/display/anim/Voc/dirty/redraw effects in order | do not treat emitted-but-discarded outputs as parity |
| Common physical delete is one late same-Main-Tick drain after native increments its authoritative frame once; gameplay removal is already synchronous | `0x00725C70`; `0x0055DE9F`; `0x0055E160`; scheduling report SHA in Section 14.1 | Rust assigns projected `binary_frame` before its drain, but 45-Hz ticks map to 15-Hz values and most drains do not observe `N -> N+1`; movement request and app animated-death timing also drift | `world/mod.rs`; `world/lifecycle.rs`; `app_sim_tick.rs`; `util/fixed_math.rs`; `app_types.rs` | retain one drain and relative placement, move eligible UnInit requests to native owners, and align the authoritative per-Main-Tick frame counter/cadence | native frame N lifecycle visibility, exactly one N+1 increment, then one physical finalization; current Rust must fail before cadence fix | do not call relative assignment order frame-value parity, add another drain, or delay gameplay removal to this service |
| Normal Foot arrival owner is current/next eligible Drive slot; Unit uses wrapper `0x741970`, not direct `0x4D94B0` | `0x004B0500`; `0x004DF0D0`; `0x004D82B0`; slot read | global pending-arrival sweep; class wrapper census incomplete | pending-arrival code in `movement_tick.rs`; per-object host | migrate proved normal owner while treating class-specific wrappers as blocker | observer ordering around empty and queued arrival; no front-loading | do not generalize the normal wrapper result to Teleport/docking subclasses |

### Stale Docs / Follow-up Docs

- In `docs/research/SLICE6_DEFERRED_DELETE_DYING_WINDOW_GHIDRA_REPORT.md`, replace
  any claim that common UnInit necessarily survives another full tick with:
  "gameplay removal is synchronous; common physical allocation survives only
  until the one late pending-delete drain in the same Main Tick."
- In `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`, use
  the same lifetime correction and keep caller-specific Mark/limbo state at the
  generic UnInit detach boundary.
- Checkpoint-C OQ-24 is superseded by Section 4.12: a concrete Infantry Tube
  completion can conceal/remove the owner from live Logic inside `+0x18C`, while
  completion and blocked-exit wrappers still run their ungated class suffix.
- Checkpoint C remains PRIMARY for binary/INI population and precedence, but its
  `cacc073f...` Rust handoff is **STALE_OR_CONFLICTED**; D's clean
  `cbf4d871...` blobs control every current-Rust conclusion.
- The ground movement design/contract must consume clean HEAD `cbf4d871...`,
  retain the lifecycle authority from `95bef99d`, and incorporate Sections
  12.1-12.4 before implementation approval.

### 12.1 Atomic removal boundary

The production flip must enumerate and remove every old caller below in the same
change that installs the per-object ground host:

- retain and extend the existing production `object_ai_stage`/
  `for_each_live_object` host; do not add a parallel live scheduler;
- the top-level production `movement::tick_movement_with_grids` global Phase-1
  gameplay-host call ceases to be production authority; only extracted pure or
  per-object helpers may survive under `object_ai_stage`;
- global `drive_reaims` preparation;
- one-time `blocker_neighbor_counts` and gameplay-authoritative
  `entity_block_sets` prepasses;
- global pending Drive arrival sweep;
- snapshot mover authority for migrated ground classes;
- `tick_low_bridge_tube_movement`;
- `tick_forced_drive_tracks`;
- population-wide formation minimum-speed sync;
- globally deferred/sorted `crush_kills` sound/health/request emission;
- pass-wide `already_scattered` suppression;
- the post-`tick_movement_with_grids` `pending_lifecycle_requests` drain as a
  movement-lifecycle owner;
- every `contains_crush_victim` handled-ID skip;
- global finished-entity finalization;
- global locomotor phase, Hover vertical, and stable-ID Ship wake updates;
- ground `tick_teleport_movement` and `tick_locomotor_piggyback_restore`;
- miner `tick_miners_with_overlay_registry` snapshot/batch pipeline;
- post-movement gate runtime sweep;
- post-movement war-factory contact cleanup;
- the contact-derived branch of per-mover `live_building_entry_skips` is removed
  or made live; unrelated gate/repair/bunker/bib/C4 exceptions are preserved;
- post-movement wall-crush scan;
- any production handled-ID bridge that would let both old and new paths own an
  effect.

Pure read-only precomputation can survive only if its results commit in live
native order and cannot hide an earlier object's mutation from a later object.
Borrow-checker staging is not a gameplay deferral license.

### 12.2 Canonical lifecycle seam

Retain and complete the existing Rust-native lifecycle authority for
Reveal/Conceal/Limbo/UnInit and pending physical deletion. Movement already
routes delayed requests through it; the required change is to invoke it inside
the current object's native-order effect point and complete its explicit
listener/receiver/output gaps. The seam must support:

- ordered detach/listener callbacks while alive byte `+0x90` is still set,
  with caller-specific registration/Mark/limbo state (normal crush already
  absent from live Logic/unmarked/limbo; direct `+0xF8` branch may still be in
  live Logic/marked/not-limbo);
- occupation removal and immediate per-cell derived-state refresh;
- display/selection/audio/animation detach signals;
- live-Logic removal with native compaction semantics;
- alive/dead state transition;
- late physical-free queue;
- later-object reads after each committed transition;
- ordered BREAK receiver/subclass effects and actual `LifecycleOutput`
  consumers; and
- Unit/Infantry Tube post-leaf suffix execution even after in-leaf conceal.

Do not copy native raw vector pointers. A Rust ordered scheduler may model the
same skip/append behavior explicitly.

### 12.3 Required discriminating fixtures

1. **Victim after cursor:** live order `[crusher, victim, N]`; crusher completes
   sound -> Mark removal -> BREAK -> Conceal/UnInit -> live compaction at frame N,
   victim never runs, and N still runs before the late frame/drain tail.
2. **Victim before cursor:** live order `[victim, crusher, N]`; when crusher
   removes the already-earlier victim, N shifts into the consumed index and is
   skipped after the outer increment. A current/self-removal variant proves the
   same shifted-successor rule.
3. **Tail reveal:** a current object reveals/registers a tail object; the re-read
   count admits it in the same pass.
4. **Crush list and sound:** two eligible victims in native cell-list order;
   sound/callback/conceal/UnInit order follows the list without stable-ID sort.
5. **Crusher changes state mid-list:** first victim's synchronous callbacks kill
   or conceal the crusher; saved-next second victim is still visited before the
   Unit/Drive post-helper gate.
6. **Direct-`+0xF8` Infantry branch:** record field-copy and crusher callbacks,
   then prove detach/listeners can see the victim alive, marked, not-limbo, and
   potentially still in live Logic before UnInit's derived Limbo; no normal crush
   sound/Mark prefix occurs.
7. **Accepted-cell owner death:** destination-cell callback walk kills the mover;
   old Mark is gone, coordinate/bridge state is committed, and new Mark plus
   per-cell suffix do not execute.
8. **Scatter RNG:** mixed eligible/ineligible occupants prove native list
   population and exact conditional inclusive draws.
9. **Moving target:** Infantry A moves before following Unit B; eligible B reaims
   to A's new coordinate in B's Drive slot, while other class pairs do not take
   the proved branch.
10. **Finished mover:** A completes before observer B; B sees cleared track and
   all current-object per-cell effects.
11. **Arrival order:** an object's empty/queued arrival runs only when its own
   next slot is reached, before or after an observer according to live order.
12. **Gate inversion:** compare `mover A -> gate -> mover B` with
   `gate -> mover A`; preserve request/progression visibility.
13. **Factory privilege:** a Unit's per-mover map initially sees the live contact,
     then the Unit leaves the building footprint and attempts a second cell entry
     in the same Drive budget; native contact privilege is already gone while the
     current frozen Rust contact-derived entry must not survive. Unrelated map
     exceptions remain unchanged.
14. **Entered-and-left wall:** a Unit fully enters and leaves a wall cell in one
    budget; sound, overlay removal, rocking, and later-object visibility still
    occur at entry.
15. **Tube call/lifecycle order:** Infantry completion garrisons and becomes
     InLimbo/absent from live Logic inside `+0x18C`, then still executes
     `+0x4A0`; Unit completion executes optional `+0x174 -> +0x18C ->
     +0x544 -> +0x4A0` without an inserted guard. Blocked exits record occupant
     `+0x174` callbacks, omit completion `+0x18C`, and still reach `+0x4A0`.
16. **Tube common speed fraction:** seed exact full owner state with fraction
     `0.375`. Blocked Unit/Infantry produce canonical `+0.0` while retaining Tube
     state; Unit success produces `1.0` after `+0x18C` even if lifecycle mutates;
     Infantry ordinary success produces `1.0` before `+0x18C`; Infantry
     `+0x5A4`-equal/`+0x174` success preserves `0.375`. No forced/ordinary bulk
     movement or Drive-only fraction overwrite may run afterward that tick.
17. **Tube post-leaf effects:** cross live/blocked, `+0x14` bit 2, signed Tube
     state, `+0x324` out-code/visibility, tracked/untracked state, and flash-due
     state. Record discovery, the unconditional radar-surface rectangle query,
     type-5 call and packed coordinates, `+0x498` old-
     coordinate removal, unconditional cache writes, `+0x494` new-coordinate add,
     and independent `+0x49C` dirtying. A garrison-concealed Infantry matrix
     records optional discovery first, false visibility, the rectangle query,
     no type-5/on-screen/add,
     normally no second removal after Limbo already removed membership, cache
     writes, and independently possible local flash dirty.
18. **Cache split:** early-unmarked victim cannot reappear in a later owner's
     blocker set or remain in one-time neighbor counts before canonical UnInit.
19. **Lifecycle outputs:** ordered Conceal outputs produce real selection,
     display, animation, Voc, dirty/drawn, and redraw effects rather than no-ops.
20. **Deferred vehicle bridge state:** compare a clear crossing and every blocked
     result; record whether `pending_bridge_update` should commit, plus OnBridge,
     bridge occupancy, selected CellClass list, and the next observer's layer read.
21. **Pending delete:** the full crush trace occurs while the observed frame is
     N; later objects see gameplay membership gone at N; normal tail commits N+1;
     common physical storage frees only in the late drain at that placement.
     Current Rust's 45-Hz projected 15-Hz frame value must fail this fixture on
     ticks where its assignment remains N; relative assignment-before-drain order
     alone is not acceptance.

Every fixture must record live object order, object IDs, coordinates/cells,
NavCom/track/Tube state, common speed fraction, occupancy lists, alive/InLimbo/
dead/live-Logic state, contact/gate/wall state, discovery/tracker/cache/radar
events, sound-event sequence, RNG cursor, frame value, and whether each later
object ran. Rust-only hashes are regression ratchets until Checkpoint E supplies
retail-derived observations.

### 12.4 Stop condition

Checkpoint D authorizes planning and inert extraction, not production routing.
Production remains blocked until:

1. the implementation contract/design are reconciled with this report;
2. the full Checkpoint C population and every D owner move atomically, including
   the closed Unit/Infantry Tube call schedule and `+0x544` mechanism plus the
   bounded direct `+0x4A0` effects;
3. the old bulk/postlude owners in Section 12.1 have no production callers or
   handled-ID bridge;
4. the gameplay-bearing D blockers remain explicit planning gates and are closed
   before production activation: complete detach/listener coverage, BREAK receiver
   effects, broader cache correspondence, the vehicle-deferred
   `pending_bridge_update` seam, alternate predicate/full `DestroyOverlay`
   equivalence, relevant class-specific arrival wrappers, and the full
   `+0x324` visibility plus tracker/pixel downstream equivalence, and native
   one-increment-per-reached-Main-Tick authoritative frame cadence;
5. Tube common applied/current fraction state, exact zero/one/preserve branches,
   substantive post-leaf effects, and same-tick no-overwrite rule have focused
   passing Rust tests;
6. first-post-load Tube/lifecycle reconstruction is closed if the cutover changes
   serialized fields/reconstruction or supports in-flight Tube save/load;
   otherwise it remains a separately named persistence-parity scope and such
   in-flight support is explicitly withheld;
7. non-ground death remains a separate class-system parity scope only while this
   cutover leaves its callers and shared lifecycle-visible behavior unchanged;
   any shared lifecycle change triggers non-ground regression closure; and
8. Checkpoint E executes retail-derived object-order, lifecycle, cache-visible,
   arrival, Tube speed/effect, radar/display, sound, gate, wall, and frame/drain
   oracle fixtures.

Static D PASS does not waive these blockers. In particular, the Tube call
schedule and common `+0x544` mechanism are closed, while complete `+0x4A0`
visibility/tracker/pixel effects remain blocked; all of them still require
implementation and an executable oracle before production activation.

## 13. Negative Findings

- No independent global movement-effect tail was found for the scoped actions.
- No normal-crush RNG call was found in `0x007416A0`.
- No proof supports stable-ID sorting of crush victims or sound events.
- No proof supports aborting the saved-next crush victim walk when an earlier
  victim callback changes crusher lifecycle state.
- No proof supports merging the direct-`+0xF8` Infantry branch into the normal
  sound/Mark/Limbo prefix.
- No proof supports a movement-pass-wide `already_scattered` suppression set.
- No proof supports front-loading every pending arrival before object slots.
- No proof supports applying the proved dynamic NavCom branch to class pairs
  other than Unit owner following Infantry target.
- No proof supports advancing every gate after every mover.
- No proof supports keeping war-factory contact through further same-invocation
  entries after its per-cell breakup point.
- No proof supports wall damage only at the mover's final standing cell.
- No proof supports raw EntityStore removal as equivalent to native UnInit.
- No proof supports delaying canonical UnInit until every mover has completed,
  even though current Rust now uses the canonical authority at that late point.
- No proof supports inserting an alive/limbo guard before the Unit/Infantry
  post-Tube `+0x4A0` suffix.
- No proof supports representing common Tube `+0x544` state only in a Drive
  locomotor or letting forced/ordinary bulk movement overwrite the wrapper's
  exact zero/one/preserve outcome later in the same tick.
- No proof supports treating Rust's floating renderer screen cache or an
  unproduced radar-event enum as equivalent to the Tube `+0x4A0` tracker/event
  transaction.
- No proof supports register-only Reveal as equivalent to native
  coordinate-commit/Mark-before-register behavior.
- No proof supports an all-entity contact-ID scrub as equivalent to ordered
  Techno BREAK broadcast and synchronous receiver side effects.
- No proof supports treating `OccupancyGrid::generation` as parity evidence;
  only the required consumer-visible result is authoritative.
- No proof supports the older claim that a common UnInit object necessarily
  remains for another complete simulation tick.
- No proof supports treating Unit `+0x480` as a direct binding to `0x004D94B0`.

## 14. Sources and Evidence Roles

### 14.1 Primary binary evidence

- Live read-only `gamemd.exe` program identity and direct
  `read_memory`/`inspect_memory_content`/`get_function_by_address`/
  `decompile_function`/`disassemble_function`/`disassemble_bytes` calls for the
  addresses enumerated in this report. **Role: PRIMARY.**
- `docs/research/GROUND_PHASE1_LOCOMOTOR_POPULATION_AND_PRECEDENCE_GHIDRA_REPORT.md`
  at SHA-256 `CBE8307F6AF27760A151D0A599C5D7400727840E3C6C2195FFA1598E82ADE37D`.
  **Role: PRIMARY** for binary/INI population and precedence only; its
  `cacc073f...` Rust snapshot is superseded by Section 14.2.
- `docs/research/DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md`
  at SHA-256 `3B94CF7E896B058CA1ECEBAB69CA63D0B736D7C46AD5D35B137FD6934CCCC93E`.
  **Role: PRIMARY** for track metadata/initializer semantics; not rederived here.
- `docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md` at SHA-256
  `0A728B262FA8358C6FDE931C93216EC5C7378D51EDC1A07BBD38FBFD4E689683`.
  **Role: PRIMARY** for speed calculation; not rederived here.
- `docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`
  at SHA-256 `4D85178F0EF454AA34472537EF8FA33DB501026C6703897BA1D4A91EB990FD63`.
  **Role: PRIMARY** for the pre-locomotor host bracket.
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md` at
  SHA-256 `5A9E6CB3DE67E3637C001A42EC6C7D34FEFD2AEDA097EFD82BBBCB388038C263`.
  **Role: PRIMARY** for object-pass/Drive scheduling and native Main-Tick cadence
  context.
- `docs/research/DIRECT_REMOVAL_SAMEPASS_MUTATION_BLOCKERS_GHIDRA_REPORT.md` at
  SHA-256 `57B13BB2E8B9C3FECBBC3F7ACDF758FB0889FC48ED25A1F6030C470B56D7D27B`,
  `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` at
  SHA-256 `11E1F14DC532A12CEDC7A1B9E16BF4B0DEB676EF5A7066D32DCAF347A60D221B`,
  and `docs/research/CELLCLASS_SUBSTRATE_LIVE_OBJECT_LIST_WRITERS_GHIDRA_REPORT.md`
  at SHA-256 `F4432B1100F08BAC7B02150C3F8585BEC7BA4327D755E239B0E2D9BA3A4BF93A`.
  **Role: PRIMARY** where consistent with the fresh live spot-checks above.
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
  at SHA-256 `84F58402AE8CC710FA4657F9617775157BBC64796904E3800E4A2644C0AE007F`.
  **Role: PRIMARY** for Reveal tail-registration/order evidence, reconciled with
  the fresh `0x005F4EC0` body/assembly pass in Section 4.5.1.
- `docs/research/BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`
  at SHA-256 `3D295875F6FF1EA91B5730DD070040E05A856A9137B0ED2CA3DB011AAFB6F71C`.
  **Role: PRIMARY** for Techno Limbo BREAK routing and contact-slot semantics,
  reconciled with the fresh `0x0065AA80/0x0065ACE0/0x0065A970` body pass in
  Section 4.5.2.
- `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md` at SHA-256
  `5E380661B31C9C2647317D3F86A5A05E64D438B0AC5E691A8E26B9231A695B1F`.
  **Role: PRIMARY** for the gate request/mission state-machine facts.
- `docs/research/WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md` at SHA-256
  `E86C7A0C6AC9805FDAD53B8C63E87382521B66318912640680F607D889F41FE7`.
  **Role: PRIMARY** for factory contact/row privilege facts.
- `docs/research/WALL_CRUSH_ON_DRIVEOVER_GHIDRA_REPORT.md` at SHA-256
  `A6C68D29F55819883DE3411ED2C5E0AC03B3687DB30FD9A0CC8E301F9A78F778`.
  **Role: PRIMARY** for active stock BFRT wall reachability.
- `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md` at SHA-256
  `C278FE66F15F1B1361B2363CD992D6BCEADA78E715C42B05CF94479CCBE298FF`.
  **Role: PRIMARY** where consistent with the fresh branch/order corrections in
  Section 4.3; fresh evidence here controls on conflict.
- `docs/research/DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`
  at SHA-256 `3E118321A77988B4A66164DB3757DD122ECC50C68D91B18DC51D6826165DD994`
  and `docs/research/NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md` at SHA-256
  `17D60FD474905C0F2B5E1B0B830BBFDCD36B8998413877B8B14A445A805330E0`.
  **Role: PRIMARY** for the bounded normal arrival path; class wrappers remain
  explicit blockers.
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`
  at SHA-256 `2943C6850EE29100AECE123E7020732E98476BFAE758752A861FB7940A889DF6`.
  **Role: PRIMARY** for Tube producer/leaf context, reconciled with the fresh D
  completion/lifecycle disassembly.

### 14.2 Rust evidence

Clean HEAD is `cbf4d8711d6c136964a2e9210c442e1c79542d69`; committed lifecycle change
`95bef99dc2c121d37b9e45298b32926d5667dd6e` is included. These exact HEAD blobs
were read directly. **Role: PRIMARY** for the current clean-Rust comparison.

| Exact path | HEAD blob |
|---|---|
| `src/sim/world/mod.rs` | `e4985e95b133e8678e1e4f69c01dca0e526083de` |
| `src/sim/world/lifecycle.rs` | `7b093b9bb2471fc95dce0b0a88190112fefdd10b` |
| `src/sim/world/logic_vector.rs` | `bd3954f8dad3b3f7ee99b38c7a186bb597ab3026` |
| `src/sim/world/techno_ai.rs` (clean HEAD only) | `ec9ba915aa830e813322b21ea2616b5f5f977915` |
| `src/sim/world/world_spawn.rs` | `fe3ce9e368c00c5c446d3631602d41e7c97a30a5` |
| `src/sim/game_entity.rs` | `141639c6b9815754e31e81b428db6eaf3a508511` |
| `src/sim/components.rs` | `67696e886d567f0902191c3e0a72ea9ce64b1307` |
| `src/sim/radar.rs` | `8c818bbe9100827b576b41abbd54cf87bad3e381` |
| `src/sim/vision/mod.rs` | `7bb6af8d3f661e40b77f1a1c448fe700a4676e95` |
| `src/util/fixed_math.rs` | `e25e4be290ef559b2b24bc229400350f62bc9a91` |
| `src/app_types.rs` | `14eb8fd1e89633b01599fd9d7725f62b7bdcc672` |
| `src/sim/lifecycle_request.rs` | `fa8d717e37bb61548d7bbd3be38e1efe4f4913c0` |
| `src/sim/movement/mod.rs` | `b7970a40fa561b5b5f2de351100d20ca6a12414d` |
| `src/sim/movement/drive_locomotion.rs` | `032306d3572efa747441bc770971c758e2fb38e7` |
| `src/sim/movement/locomotor.rs` | `2179c3dd2a37f8c54e09b1cd89c1cdf3a47ccbbf` |
| `src/sim/movement/navcom.rs` | `09d7d9722da01bcfed2c7123f7d2b19f55d9fbd6` |
| `src/sim/movement/movement_tick.rs` | `4b5cbf65b5e1e3265c8d9ab0167ca0115fa94aae` |
| `src/sim/movement/movement_occupancy.rs` | `d1fc3f49330ce7faba0d47e086c82e644ac78cff` |
| `src/sim/movement/movement_step.rs` | `c91dafb9d0a486e52491d0d13fec87cbf256c249` |
| `src/sim/movement/movement_bridge.rs` | `b8b43779e891487b977a2f88a610e82b0492d994` |
| `src/sim/movement/bump_crush.rs` | `931074250e6c210a5dfbfb732bf50b1ac2ded2a2` |
| `src/sim/movement/tube_movement.rs` | `60f6f31a0a315469f472700f3bf15ccb0c2b6f1f` |
| `src/sim/movement/teleport_movement.rs` | `6c05622bbb559237ec82975ad87c7e7ebeadfe67` |
| `src/sim/occupancy.rs` | `016a65ced898859c6d70953b378d4b499abfac06` |
| `src/sim/radio/mod.rs` | `755e737cc740798e56ca201fb5960fbc5d52257e` |
| `src/sim/radio/receive.rs` | `1931ce149ac9289a6de6099d99f7a1f0d874c0bb` |
| `src/sim/gate_runtime.rs` | `e34209fba952b083362a742cf7a98a85a464c02c` |
| `src/sim/production/war_factory_exit.rs` | `90eeb61aef04b7775612b7d4eed616c09f521d97` |
| `src/sim/miner/miner_system.rs` | `01317c66280cfa540f6f1db4d79bb7e3624e2255` |
| `src/sim/production/production_economy.rs` | `27f1b31c7fb33f3931e5873094ac45ced9164fac` |
| `src/app_sim_tick.rs` | `0dd7f3bf4f891d263a22abd9094c58deee37059b` |
| `src/app_instances/helpers.rs` | `38771748ba22e1e734ce694941eec3a5a4b6392c` |
| `src/app_render/build_instances.rs` | `b734e833d791e97d0917a1c4f0e087c69cf4bef1` |
| `src/render/minimap.rs` | `0064d42faa7c0fbd9aaf5c8eca83a2b4b69f492d` |

The shared worktree's companion-owned `src/sim/world/techno_ai.rs` differs from
the clean HEAD blob above. The clean blob is **PRIMARY** for the existing
production live-host comparison; only the dirty working copy is
**STALE_OR_CONFLICTED** and excluded.

### 14.3 Conflicted or derivative prose

- `docs/research/SLICE6_DEFERRED_DELETE_DYING_WINDOW_GHIDRA_REPORT.md`.
  **Role: PRIMARY** for its resolved `IsDead`/drain evidence; its earlier
  “one-or-more ticks” overview/handoff language is **STALE_OR_CONFLICTED**.
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`.
  **Role: PRIMARY** for verified Conceal/cell operations where freshly
  rechecked; older “one tick/up to one tick” lifetime wording and superseded
  passenger/death prose are **STALE_OR_CONFLICTED**.
- `docs/plans/2026-07-21-ordered-reveal-conceal-uninit-lifecycle-authority-plan.md`.
  **Role: DERIVATIVE** planning input only; not binary evidence.
- `docs/plans/2026-07-20-ground-movement-atomic-flip-readiness-investigation-plan.md`.
  **Role: DERIVATIVE** scope/exit criteria only.

## 15. Final Statement

Checkpoint D passes the bounded owner-or-explicit-blocker coverage map:
current-object movement effects must commit within that object's turn; affected
gates progress in their own ordered turns; only physical deletion belongs to a
late drain. Checkpoint-C OQ-24 is now resolved: Tube-internal lifecycle mutation
does not suppress the ungated Unit/Infantry class suffix. The common `+0x544`
speed-state mechanism and bounded direct `+0x4A0` sequence are closed; the full
`+0x324` visibility family and downstream tracker/pixel equivalence remain
explicit blockers.

Clean HEAD `cbf4d871...` contains a useful bounded lifecycle authority, but its
snapshot movement host, globally late crush requests, incomplete listeners and
BREAK receivers, no-op lifecycle outputs, and remaining bulk Tube/forced/cache/
arrival/finalizer/phase/Teleport/miner/postlude owners are DRIFT. Moving only
Drive arithmetic cannot activate the native per-object host. Rust's projected
15-Hz `binary_frame` assignment precedes its drain but does not reproduce the
native authoritative frame increment on every reached Main Tick.

Production remains **NO-GO**. The next runtime gate is Checkpoint E's executable
retail-derived object-order, lifecycle, occupancy/cache, RNG, sound-event,
contact, gate, wall, arrival, Tube speed/radar/display effects, and frame/drain
fixtures. Save/load Tube/lifecycle state gates any changed persistence or
supported in-flight reconstruction; non-ground death remains separate only
while its callers and shared lifecycle behavior are unchanged. The named D
system-context blockers and atomic old-caller removal boundary must also be
reconciled into the design/contract before any production cutover. Until then,
parity remains unverified.
