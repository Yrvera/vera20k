# Damage PostMortem / `CausesDelayKill` Reinvestigation

**Date:** 2026-07-13  
**Binary:** active retail Yuri's Revenge `gamemd.exe` 1.001 (`x86:LE:32`)  
**Mode:** exhaustive bounded slice, static/read-only Ghidra only  
**Assigned scope:** `CausesDelayKill`, `EligibleForDelayKill`, delay interpolation,
repeated lethal hits, life/health restoration, result 5, shared latch/timer state,
cancellation, save/load/checksum behavior, and the `BuildingClass::Update` expiry
owner  
**Out of scope:** area-target collection and producer argument provenance (Task
3), projectile scheduler placement (G2), live process/oracle capture (Task 4/G3),
and Rust implementation

## Verdict

**PARTIAL for exact byte parity; COMPLETE for the active behavioral state
machine in this bounded slice.**

The player-visible mechanism is active stock YR behavior, not dormant TS code.
When a qualifying lethal hit reaches `TechnoClass::ReceiveDamage`, the original
death transaction has already run. The engine then optionally arms or shortens a
shared Building timer, writes `IsAlive = true` and `Health = 1`, and returns
damage result **5 (PostMortem)**. At expiry, `BuildingClass::Update` synchronously
calls the Building damage receiver again with current health as damage,
`C4Warhead`, distance zero, the retained source pointer, and
`ignore_defenses = 1`.

Two boundaries prevent an exact-parity certification:

1. **`BuildingClass+0x52C` is raw persisted UNKNOWN state.** The PostMortem arm
   and Building IronCurtain cancellation paths copy a non-dominating,
   uninitialized stack dword into it. The value is included in raw object save
   bytes, is not consumed by the expiry path, and is not included in the
   Building deterministic checksum. Static evidence proves that provenance but
   cannot define one deterministic native value for Rust.
2. **Stock end-to-end fixture invocation belongs to Task 3/G2.** Every stock
   `EligibleForDelayKill` object is also `Insignificant=yes`. This report does
   not guess the producer's `ignore_defenses`, exact distance origin, target
   ordering, or detonation scheduler position. It supplies exact expected
   PostMortem outcomes once a producer call reaches the verified `result == 4`
   gate.

Accordingly, this report is sufficient for behavioral design and shadow-mode
work, but **G1 exact-byte authority must remain blocked**, and direct/weapon-area
authority remains independently blocked on Task 3 plus G2.

## Evidence discipline

All Ghidra citations below name the actual static operation used. Imported
labels are treated as navigation hints; load-bearing claims use instruction
bodies, raw vtable bytes, field accesses, call order, and stock INI data.

| Mark | Meaning in this report |
|---|---|
| **VERIFIED** | Directly shown by active `gamemd.exe` instructions/data, or by current retail INI text for a content claim. |
| **INFERRED** | Strong interpretation of verified instructions, explicitly separated from the bytes themselves. |
| **UNKNOWN** | Not deterministically recoverable from the bounded static evidence. |
| **DEFERRED** | Owned by a named later task/gate rather than silently assumed. |

Primary binary anchors:

| Role | Address |
|---|---:|
| `TechnoClass::ReceiveDamage` PostMortem block | `0x00701E71`–`0x00701F72` |
| `Math__ftol` | `0x007C5F00` |
| `ObjectClass::ReceiveDamage` exact-zero callbacks | `0x005F5700` |
| `BuildingClass::ReceiveDamage` result dispatch | function `0x00442230`, dispatch around `0x00442425` |
| `BuildingClass::Update` pending-latch consumer | function `0x0043FB20`, block `0x004401D2`–`0x00440372` |
| `BuildingClass::IronCurtain` cancellation | `0x00457C90` |
| Infantry C4/Ivan shared-latch producer | function containing `0x0051A546`–`0x0051A608` |
| Building constructor timer/source initialization | `0x0043B740` |
| Building deterministic checksum fields | `0x00454260` |

## 1. Rule fields and defaults

### 1.1 Warhead fields

**VERIFIED.** `WarheadTypeClass::ReadINI` reads the three keys into these exact
fields:

| Offset | Native storage | INI key | Constructor default |
|---:|---|---|---:|
| `+0x130` | byte/bool | `CausesDelayKill` | `false` |
| `+0x134` | signed dword | `DelayKillFrames` | `5` |
| `+0x138` | IEEE binary32 | `DelayKillAtMax` | `1.0f` (`0x3F800000`) |

The constructor writes zero to `+0x130`, `5` to `+0x134`, and the same
`0x3F800000` value used by `PercentAtMax` to `+0x138`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x0075CF15, 0x0075CF50)`]
The ReadINI body passes the existing field value as the default to
`ReadBool`, `ReadInt`, and `ReadDouble`, then stores the returned values at the
same offsets.
[Ghidra: `decompile_function(gamemd.exe, 0x0075D3A0)`]

### 1.2 Building-type field

**VERIFIED.** `BuildingTypeClass+0x1551` is the byte/bool
`EligibleForDelayKill`, default `false`. It is not `SelfHealing`, `Crewed`, or a
bridge-only capability.

The string `EligibleForDelayKill` exists once at `0x0081ACB0`; its only data
xref is `BuildingTypeClass::ReadINI+0x...` at `0x00460224`. The surrounding
instructions read the old byte at `+0x1551`, pass the string and old value to
the bool reader, and store `AL` back to `+0x1551`.
[Ghidra: `search_strings(gamemd.exe, "EligibleForDelayKill")`;
`get_xrefs_to(gamemd.exe, 0x0081ACB0)`;
`disassemble_bytes(gamemd.exe, 0x00460200, 0x00460250)`]
The constructor writes the zero-valued `BL` to `+0x1551` at `0x0045DFFF`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x0045DFEC, 0x0045E015)`]

## 2. Exact eligibility and ordered PostMortem transition

### 2.1 Eligibility gates

**VERIFIED.** All five conditions below must hold in this order:

1. The immediately preceding `ObjectClass::ReceiveDamage` result is exactly
   `4` (`CMP EDI,4` at `0x00701E71`).
2. The receiver's virtual `WhatAmI` call at vtable `+0x2C` returns `6`
   (Building).
3. The warhead pointer is non-null.
4. `warhead+0x130` (`CausesDelayKill`) is nonzero.
5. `receiver->BuildingType(+0x520)+0x1551`
   (`EligibleForDelayKill`) is nonzero.

[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701E50, 0x00701F70)`, especially
`0x00701E71`–`0x00701ECA`]

There is **no separate `ignore_defenses` test in the PostMortem block**. That
argument and all earlier immunity/armor rules matter only insofar as they
determine whether the Object receiver returns 4. A nonlethal hit, a blocked hit,
an already-dead target, a non-Building, a null warhead, or either false rule flag
does not arm this state here.

The incoming damage value is not read by the delay formula. It has already done
its job by causing result 4. Duration depends only on the caller-provided signed
distance and three warhead fields.

### 2.2 Ordered transition

| Sequence | Predicate/read | Native action | Result/state |
|---:|---|---|---|
| 1 | Object receiver reaches exact zero HP | Exact-zero callbacks and Building virtual destroy path execute | Object result becomes 4 |
| 2 | Five gates above pass | Compute signed delay duration with x87 sequence | Candidate duration in low `EAX` |
| 3 | `+0x6DF == 0` | Arm latch and write timer triple | Pending state created |
| 3a | `+0x6DF != 0` | Compute signed remaining time and keep the smaller duration | Timer may remain byte-for-byte unchanged |
| 4 | Always after qualifying PostMortem, even when timer was kept | `receiver+0x90 = 1`; `receiver+0x6C = 1` | Alive at one HP |
| 5 | Immediate | `EAX = 5`; return | `PostMortem` |

[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701E71, 0x00701F72)`]

## 3. Exact delay interpolation

### 3.1 Inputs and units

Define:

- `B = *(i32 *)(warhead+0x134)` (`DelayKillFrames`)
- `A = *(binary32 *)(warhead+0x138)` (`DelayKillAtMax`)
- `C = *(binary32 *)(warhead+0x124)` (`CellSpread`)
- `d = caller distance`, read as a signed dword from the receiver stack

The delay block first truncates `C` to an integer, then shifts that signed
low-32 result left eight bits. This converts whole cells to the receiver's
256-units-per-cell distance denominator. Thus the block expects `d` in the same
world-lepton unit. **DEFERRED:** Task 3 must prove the producer's exact coordinate
origin and value provenance; this block itself proves the unit conversion it
performs.

### 3.2 Instruction-exact operation order

**VERIFIED.** For ordinary finite inputs, the semantic expression is:

```text
den_i32 = (low_i32(Math__ftol_x87(C)) << 8) with 32-bit wrap
slope_ext80 = (ext80(A) * ext80(B)) - ext80(B)
delay_ext80 = ext80(B) + ext80(d) * (slope_ext80 / ext80(den_i32))
new_duration_i32 = low_i32(Math__ftol_x87(delay_ext80))
```

Equivalent mathematical shorthand, **only when it preserves that operation
order and those conversions**, is:

```text
trunc_x87(B + d * (B * (A - 1)) / (trunc_x87(C) << 8))
```

The actual stack sequence is more precise than the shorthand:

1. `FILD B`
2. `FLD A`
3. `FMUL ST1` => `A * B`
4. `FSUB ST0,ST1` => `(A * B) - B`
5. `FLD C`; call `Math__ftol`
6. `SHL EAX,8`; `FILD` the signed shifted result
7. `FDIVP`
8. `FIMUL d`
9. `FADD ST0,ST1`, where the retained `ST1` is the original `B`
10. call `Math__ftol`; later `FSTP ST0` discards the retained `B`

[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701ED0, 0x00701F18)`]

`Math__ftol` uses `FISTP qword`, returns its low dword in `EAX`, and loads x87
control word `0x0E7F` when the current control word differs. That control word
masks exceptions, selects 53-bit precision control, and rounds toward zero.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x007C5F00, 0x007C5F3D)`;
`read_memory(gamemd.exe, 0x00822D80, 4)` => `7f0e0000`]

Consequences that an implementation must preserve:

- `B`, `d`, and the shifted denominator are signed 32-bit inputs to x87.
- `A` and `C` enter as exact binary32 values, not reparsed host decimals.
- Fractional `CellSpread` is truncated **before** multiplication by 256.
- The `<< 8` is a wrapping x86 32-bit shift, not a saturating multiply.
- Intermediate arithmetic stays on the x87 stack; early binary32/binary64
  stores change results.
- There is no denominator-zero guard or output clamp in this block. Custom-data
  exceptional cases must use the native x87 exception/indefinite semantics, not
  a Rust convenience branch.

### 3.3 Stock `OilExplosionWH` values

Retail `rulesmd.ini` supplies:

```ini
[OilExplosionWH]
CellSpread=4
PercentAtMax=.5
CausesDelayKill=yes
DelayKillFrames=5
DelayKillAtMax=7.0
```

[`ini/rulesmd.ini:27201`–`27210`]

For those exactly representable inputs:

```text
denominator = 4 << 8 = 1024 leptons
duration(d) = trunc_toward_zero(5 + (30 * d) / 1024)
```

Binary-derived checkpoints (not a parity certification by themselves):

| Distance `d` | Cell interpretation | Duration |
|---:|---:|---:|
| 0 | center | 5 |
| 34 | 0.1328125 cells | 5 |
| 35 | 0.13671875 cells | 6 |
| 256 | 1 cell | 12 |
| 512 | 2 cells | 20 |
| 768 | 3 cells | 27 |
| 1023 | just inside 4 cells | 34 |
| 1024 | 4 cells | 35 |

Concrete conversion walk: for `d=256`, the x87 numerator contribution is
`30*256/1024 = 7.5`; adding base `5` yields `12.5`; `Math__ftol` truncates to
`12`.

## 4. Repeated hits and shared pending state

### 4.1 Remaining-time calculation

**VERIFIED.** When `building+0x6DF != 0`, the code reads signed duration
`+0x530` and signed start frame `+0x528`:

```text
if start == -1:
    remaining = duration
else:
    elapsed = current_frame - start            // wrapping i32 machine arithmetic
    remaining = duration - elapsed if elapsed < duration else 0
```

The comparison is signed. It then performs:

```text
if new_duration >= remaining:
    keep all existing latch/timer metadata
else:
    overwrite start/raw-mid/duration with the new candidate
```

Therefore only a **strictly shorter** candidate replaces the timer. Equal and
longer candidates keep the existing timer. There is no duration extension.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701F0E, 0x00701F60)`]

Regardless of whether the timer is replaced, the current lethal hit has already
run the Object exact-zero transaction. The PostMortem block then restores
`IsAlive` and Health and returns 5 again. Repeated qualifying lethal hits can
therefore re-run trigger/kill/destroy callbacks even when the timer bytes do not
change.

### 4.2 Shared state with infantry C4/Ivan planting

**VERIFIED.** `+0x6DF` and the `+0x528/+0x52C/+0x530` timer triple are not a
private OilExplosion timer. The active Infantry planting path:

- tests `building+0x6DF` at `0x0051A546`;
- if clear, sets it at `0x0051A5A7`;
- writes the planting infantry pointer to `building+0x540`;
- writes the same timer triple through `building+0x528`;
- if already set, takes the existing-pending path and does not overwrite the
  Building latch/timer/source.

[Ghidra: `disassemble_bytes(gamemd.exe, 0x0051A520, 0x0051A610)`]

The PostMortem arm never writes `+0x540`. A fresh Building has `+0x540 = 0`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x0043B740, 0x0043B7A0)`]
Thus:

- a fresh OilExplosion PostMortem timer normally has a null expiry source;
- a PostMortem hit that strictly shortens an existing infantry-planted timer
  preserves that infantry source pointer;
- an infantry plant cannot create a second independent timer while the shared
  latch is set.

This shared-producer contract is load-bearing for Rust: independent
`pending_c4_detonation` and `delay_kill` timers are mechanism drift even if a
single sampled death frame happens to match.

## 5. Timer/latch/source field ledger

| Field | Native role | Initialization | PostMortem arm | Other load-bearing writers | Consumers/persistence | Status |
|---:|---|---|---|---|---|---|
| `+0x528` | signed start frame | current frame | current frame when new/shorter | infantry plant; IronCurtain cancellation | Update remaining-time calculation; checksum normalizes remaining; raw save/load | **VERIFIED** |
| `+0x52C` | opaque middle dword | **not initialized by Building constructor** | copies undominated stack local | infantry plant writes a local; IronCurtain copies another uninitialized local | raw save/load; no expiry read; omitted from Building checksum | **UNKNOWN raw value** |
| `+0x530` | signed duration | 0 | computed duration when new/shorter | infantry plant duration; IronCurtain writes 0 | Update remaining-time calculation; checksum hashes remaining; raw save/load | **VERIFIED** |
| `+0x540` | retained source object pointer | null | untouched | infantry plant sets source; removal notification may null it; IronCurtain/bridge expiry clear it | regular expiry passes it as source object; pointer fixup/load; checksum hashes referent identity | **VERIFIED** |
| `+0x6DF` | shared pending latch byte | 0 | set to 1 when new/shorter | infantry plant sets; IronCurtain clears; bridge-hut expiry clears | Update expiry gate; checksum; raw save/load | **VERIFIED** |

### 5.1 Why `+0x52C` is an exact-byte blocker

**VERIFIED provenance; UNKNOWN value.** `TechnoClass::ReceiveDamage` allocates
`0xB4` stack bytes but does not initialize the relevant slot. The first direct
access to `[ESP+0x4C]` in the function is the read at `0x00701F41`; direct writes
to that slot occur only later at `0x00702785` and `0x007029BD`, after the early
PostMortem return. The read is stored to `building+0x52C`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701900, 0x007019C0)`;
`search_instructions(gamemd.exe, function=0x00701900, operand_pattern=0x4c)`;
`decompile_function(gamemd.exe, 0x00701900)`]

The Building constructor initializes `+0x528`, `+0x530`, and `+0x540`, but has no
write to `+0x52C` in the constructor field block.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x0043B740, 0x0043B7A0)`]

The IronCurtain path repeats the pattern: after allocating stack locals, it reads
an uninitialized local at `[ESP+8]` and stores it to the timer middle dword.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x00457C90, 0x00457CE0)`]

This report does not rename the value as a seed, facing, or timer semantic. No
such role was verified. It is raw process-history-dependent state that exact
byte parity cannot silently normalize.

## 6. What the HP=1 restoration does and does not undo

### 6.1 Retained lethal-path calls

**VERIFIED control flow.** The Object receiver reaches exact zero before the
PostMortem gate. At zero it:

1. calls vtable `+0xE0` with the source object, or `+0xE4` with the source house
   according to the existing source-selection branch;
2. sets its local result to 4; and
3. calls vtable `+0xDC` with argument 1.

[Ghidra: `decompile_function(gamemd.exe, 0x005F5700)`, exact-zero block]

For Building's raw vtable, `+0xDC/+0xE0/+0xE4` resolve respectively to
`0x0044EBF0`, `0x00702D40`, and `0x00703230`.
[Ghidra: `read_memory(gamemd.exe, 0x007E3F98, 12)` =>
`f0eb4400402d700030327000`]

The `+0xE0/+0xE4` bodies execute kill/house accounting, event, statistics, and
veterancy-related paths. The Building `+0xDC(1)` body abandons/releases an active
Factory when present, runs its house-factory cancellation checks, calls another
Building virtual, and then calls `ObjectClass::Destroy`.
[Ghidra: `decompile_function(gamemd.exe, 0x00702D40)`;
`decompile_function(gamemd.exe, 0x00703230)`;
`decompile_function(gamemd.exe, 0x0044EBF0)`]

`ObjectClass::Destroy` detaches a line trail, may deselect, clears the Display
last-reference pointer when applicable, and calls the observer/removal
notification dispatcher at `0x007258D0`.
[Ghidra: `decompile_function(gamemd.exe, 0x005F5280)`;
`decompile_function(gamemd.exe, 0x007258D0)`]

The dispatcher is **not evidence of Logic scheduler unregistration**. It walks
RTTI/listener registries and detaches references. This slice found no
remove-then-reinsert operation around PostMortem. Rust must not invent one from
the stale label alone.

### 6.2 The only universal restoration writes

**VERIFIED.** After optional timer replacement, the PostMortem block universally
writes only:

```text
receiver+0x90 = 1       // IsAlive/life byte
receiver+0x6C = 1       // signed Health dword
return 5
```

It does not call compensating trigger, score, factory, selection, observer, or
reference-registration functions. It does not restore the old HP, undo the
exact-zero callbacks, or write `+0x540`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x00701F41, 0x00701F73)`]

Therefore the lethal path is not a transaction rolled back to the pre-hit
state. It is a death transaction followed by a narrow one-HP/life restoration.

### 6.3 Building wrapper handling of result 5

**VERIFIED.** `BuildingClass::ReceiveDamage` calls the Techno receiver at
`0x00442425`, checks the restored life byte, and dispatches result values 2–5
through a jump table. Raw table entry 3 (result 5 after subtracting 2) is
`0x0044247D`; the result-4 entry is `0x004424A2`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x004423E0, 0x004424A0)`;
`read_memory(gamemd.exe, 0x00442C18, 16)`]

The result-5 branch only unwinds/frees the temporary snapshot vector when
needed, then returns the current result. It skips the Building result-4 derived
death-cleanup branch and the normal surviving post-damage tail. Work performed
before the Techno call and inside the Object/Techno call remains performed.

## 7. Cancellation, healing, removal, and save/load

### 7.1 IronCurtain / ForceShield entry cancels pending state

**VERIFIED.** At entry, `BuildingClass::IronCurtain` checks `+0x6DF`. If set, it
performs these writes before delegating to `TechnoClass::IronCurtain`:

```text
+0x6DF = 0
+0x540 = 0
+0x528 = current_frame
+0x52C = uninitialized local dword
+0x530 = 0
```

[Ghidra: `disassemble_bytes(gamemd.exe, 0x00457C90, 0x00457CE0)`;
`decompile_function(gamemd.exe, 0x00457C90)`]

The wrapper receives the force-shield mode argument but executes this
cancellation before delegation, so pending shared state is not merely paused
behind invulnerability. This directly contradicts current Rust's C4 comments
and retry behavior.

### 7.2 Healing and ordinary damage

**VERIFIED by absence of a cancellation branch plus expiry input.** Healing or
repair does not clear `+0x6DF`. The pending block occurs late in
`BuildingClass::Update`, after the parent Techno update and after the
Building-specific repair/power and auto-production helpers. At expiry it copies
the Building's **current signed Health** and uses that as the new damage input.
[Ghidra: `decompile_function(gamemd.exe, 0x0043FB20)`;
`disassemble_bytes(gamemd.exe, 0x004401D2, 0x00440378)`]

Thus healing can raise the HP seen by expiry, but expiry raises its forced
damage to the same current value. Ordinary lethal damage before expiry can
still kill the Building through the normal receiver path.

### 7.3 Source removal

**VERIFIED.** The Building removal-notification handler clears `+0x540` when its
referenced object is removed, while leaving the pending latch/timer intact.
[Ghidra: `decompile_function(gamemd.exe, 0x0044E910)`]
Expiry can therefore become sourceless if the planted source disappears first.

### 7.4 Save/load and deterministic checksum

**VERIFIED.** `AbstractClass::Save` writes the virtual object size as a raw byte
block; `AbstractClass::Load` reads that block. Building/Techno/Radio load layers
subsequently perform their pointer fixups, including the `+0x540` reference.
Consequently `+0x528`, raw `+0x52C`, `+0x530`, `+0x540`, and `+0x6DF` persist in
native save bytes.
[Ghidra: `decompile_function(gamemd.exe, 0x00410320)`;
`decompile_function(gamemd.exe, 0x00410380)`;
`decompile_function(gamemd.exe, 0x00453E20)`]

The separate Building deterministic checksum path:

- converts `+0x528/+0x530` to signed remaining duration and hashes that value;
- hashes the `+0x540` referent identity when non-null;
- hashes byte `+0x6DF`; and
- does **not** read or hash `+0x52C`.

[Ghidra: `decompile_function(gamemd.exe, 0x00454260)`]

This distinction is why `+0x52C` is simultaneously a raw-save byte problem and
not a native deterministic-checksum input.

## 8. Expiry owner and exact recursive damage call

### 8.1 Timer test

**VERIFIED.** `BuildingClass::Update` owns the consumer. When `+0x6DF` is clear,
it skips the block. When set:

```text
duration = *(i32 *)(this+0x530)
start = *(i32 *)(this+0x528)

if start != -1:
    elapsed = current_frame - start
    if elapsed < duration:
        duration -= elapsed
    else:
        expire now

if duration != 0:
    skip expiry
else:
    expire now
```

All arithmetic and comparisons are signed 32-bit x86 operations. There is no
`u64`, saturating elapsed, or independent countdown field.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x004401D2, 0x00440206)`]

### 8.2 Regular Building expiry

**VERIFIED.** If `BuildingType+0x16B6` (`BridgeRepairHut`) is false, Update saves
current Health and synchronously invokes virtual `+0x16C` (Building
`ReceiveDamage`) with this raw argument set:

| Argument | Value at expiry |
|---|---|
| damage pointer | address of local initialized from current signed Health |
| distance | `0` |
| warhead | `RulesClass+0xFA8` (`C4Warhead`) |
| source object | `building+0x540` |
| ignore defenses | `1` |
| source house | `0` |
| final/extra argument | `0` |

[Ghidra: `disassemble_bytes(gamemd.exe, 0x00440333, 0x0044035E)`]

The regular branch does not clear `+0x6DF` or `+0x540` before or after the call.
It checks `+0x90` immediately afterward. If the receiver unexpectedly leaves
the object alive, Update calls virtual `+0x124(2)` and the still-set latch makes
the expiry eligible to retry on the next Building update.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x0044035E, 0x00440378)`]

For a fresh OilExplosion PostMortem state, `+0x540` is normally null, so expiry
is a sourceless forced C4Warhead hit. If OilExplosion shortened an existing
infantry-planted timer, expiry retains that infantry source.

### 8.3 BridgeRepairHut branch

**VERIFIED.** If `BuildingType+0x16B6` is true, the same expired shared latch
instead scans the 5×5 area and dispatches high/low bridge destruction. It then
clears `+0x6DF` and `+0x540`; if the Building remains alive, it calls virtual
`+0x124(2)`.
[Ghidra: `disassemble_bytes(gamemd.exe, 0x00440206, 0x00440331)`]

This is a special consumer of the generic shared latch, not evidence that
`+0x6DF` or `EligibleForDelayKill` is bridge-specific.

## 9. Stock YR content and fixtures

### 9.1 Complete current `rulesmd.ini` inventory

**VERIFIED retail-data inventory.** Exactly one stock YR warhead section sets
`CausesDelayKill=yes`: `OilExplosionWH`.
[`ini/rulesmd.ini:27201`–`27210`]

Exactly three stock sections set `EligibleForDelayKill=yes`:

| Type | Strength | Armor | Other relevant stock facts | Death weapon |
|---|---:|---|---|---|
| `CAMISC01` | 5 | concrete | `Insignificant=yes`, `Explodes=yes`, `CanC4=no` | `BarrelExplosion` |
| `CAMISC02` | 5 | concrete | `Insignificant=yes`, `Explodes=yes`, `CanC4=no` | `BarrelExplosion` |
| `AMMOCRAT` | 1 | wood | `Insignificant=yes`, `Explodes=yes`, `CanC4=no` | `BarrelExplosion` |

[`ini/rulesmd.ini:14930`–`14949`;
`ini/rulesmd.ini:14952`–`14971`;
`ini/rulesmd.ini:22277`–`22299`]

The two stock weapons using `OilExplosionWH` are:

| Weapon | Damage | Role in stock data |
|---|---:|---|
| `OilExplosion` | 600 | `CAOILD` death weapon |
| `BarrelExplosion` | 200 | eligible barrel/ammo-crate death weapon |

[`ini/rulesmd.ini:22358`–`22367`]

`CAOILD` has Strength 1000, steel armor, `Explodes=yes`, and
`DeathWeapon=OilExplosion`, but it does **not** set
`EligibleForDelayKill=yes`. It is a stock producer of the special warhead, not a
stock eligible target.
[`ini/rulesmd.ini:13928`–`13950`]

### 9.2 Per-type PostMortem expectations

Once an actual producer call reaches the verified `Object result == 4` gate,
all three eligible types use the same timer expression because target Strength,
Armor, and type identity do not enter the duration block:

| Target | `d=0` | `d=256` | `d=512` | `d=768` | `d=1024` | Restored state/result |
|---|---:|---:|---:|---:|---:|---|
| `CAMISC01` | 5 | 12 | 20 | 27 | 35 | Alive, HP=1, result 5 |
| `CAMISC02` | 5 | 12 | 20 | 27 | 35 | Alive, HP=1, result 5 |
| `AMMOCRAT` | 5 | 12 | 20 | 27 | 35 | Alive, HP=1, result 5 |

**DEFERRED fixture boundary:** because all three are `Insignificant=yes`, this
report does not assert that a hand-constructed ordinary damage call with
`ignore_defenses=false` reaches result 4. Task 3 must provide the active area
producer's exact call arguments and per-target distance. G2 must place that call
at the verified impact scheduler position. The table is the exact receiver-side
expected outcome once those prerequisites are met.

## 10. Contradictions and superseded prior claims

This report does not edit prior reports because its assigned sole output is this
file. The following claims are superseded for this mechanism:

| Prior claim | Verdict | Current evidence |
|---|---|---|
| No standard YR warhead uses the delay-kill keys; likely TS legacy | **WRONG** | `OilExplosionWH` sets all three keys and the active receiver consumes them. |
| `BuildingType+0x1551` is `SelfHealing`, `Crewed`, or bridge-hut capability | **WRONG** | String xref and ReadINI body prove `EligibleForDelayKill`. |
| Delay uses a generic `distance/CellSpread` float ratio | **MISLEADING** | It truncates `CellSpread` to i32, shifts by 8, and uses the exact x87 stack order above. |
| `+0x52C` may be facing or a seed | **UNSUPPORTED** | It receives undominated stack data, has no expiry read, and is omitted from checksum. Keep UNKNOWN. |
| `+0x6DF` is bridge-only | **WRONG** | Both PostMortem and infantry planting produce it; Building Update chooses regular or bridge-hut expiry. |
| Oil Derrick is the eligible delayed target | **MISLEADING** | `CAOILD` produces `OilExplosionWH`; the only eligible stock targets are `CAMISC01`, `CAMISC02`, and `AMMOCRAT`. |
| Applying IronCurtain merely postpones planted damage until immunity ends | **WRONG** | Building IronCurtain clears the shared latch/source and zeros duration immediately. |
| `0x007258D0` proves Logic scheduler removal | **WRONG/UNPROVEN** | Its body is observer/reference-removal dispatch; no scheduler remove/reinsert was found in this path. |

Applicable stale sources include
`WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`,
`WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md`,
`RECEIVE_DAMAGE_GHIDRA_REPORT.md`,
`BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`, and early sections of bridge
damage reports. Their still-correct raw addresses remain navigation aids; their
contradictory field meanings and high-level prose are not authority.

## 11. Current Rust disparity inventory

The current Rust tree has no mechanism-equivalent PostMortem path:

1. `WarheadType` stores `cell_spread` and `percent_at_max` but has no
   `CausesDelayKill`, signed `DelayKillFrames`, or exact binary32
   `DelayKillAtMax`; its parser therefore drops all three stock keys.
   [`src/rules/warhead_type.rs:30`–`119`, `123`–`202`]
2. `ObjectType` has no `EligibleForDelayKill` field/parser.
   [`src/rules/object_type.rs:141` onward]
3. The damage substrate's `DamageState` stops at `Dead`; no PostMortem result is
   representable, and `receive_damage` returns a pure HP delta without ordered
   exact-zero side effects or one-HP restoration.
   [`src/sim/combat/damage/mod.rs:142`–`158`;
   `src/sim/combat/damage/receive.rs:34`–`111`]
4. Live normal and area paths directly apply unsigned saturating subtraction and
   defer death handling to a later phase. They cannot express signed native HP,
   synchronous death callbacks followed by restoration, or result 5.
   [`src/sim/combat/mod.rs:1076`–`1095`, `1849`–`1904`]
5. Current area code integer-square-roots the lepton distance, divides by 256
   with integer truncation, converts that whole-cell value to fixed point, and
   discards the original per-target lepton distance before damage return. That
   loses `d=34` versus `d=35` and other PostMortem thresholds.
   [`src/sim/combat/combat_aoe.rs:182`–`216`]
6. `GameEntity` has a separate `pending_c4_detonation`; its state uses
   `u64 plant_start_tick` and attacker ID. The current updater uses saturating
   `u64` elapsed and explicitly retains the marker through IronCurtain so it can
   retry. Native uses the shared signed-i32 latch/timer/source state and cancels
   it on Building IronCurtain.
   [`src/sim/game_entity.rs:443`–`456`;
   `src/sim/components.rs:1041`–`1060`;
   `src/sim/world/world_orders.rs:434`–`635`]
7. Current world hash folds `pending_c4_detonation` directly; native Building
   checksum folds semantic remaining duration, source referent identity, and
   latch, while omitting raw `+0x52C`.
   [`src/sim/world/world_hash.rs:515`–`624`]
8. `GameEntity` is serialized wholesale and snapshots are versioned. Any later
   authoritative shared pending-state addition requires coordinated snapshot
   ownership/versioning rather than an unversioned field insertion.
   [`src/sim/game_entity.rs:189`–`194`;
   `src/sim/snapshot.rs:71`, `140`–`183`]

Each difference is **DRIFT**, not an internal implementation choice, until exact
equivalence is positively proved.

## 12. Implementation handoff (no code in this investigation)

### Required semantic deltas

1. Parse and retain exact warhead inputs: byte `CausesDelayKill`, signed i32
   `DelayKillFrames`, and raw binary32 `DelayKillAtMax`, with verified defaults.
2. Parse `EligibleForDelayKill` on Building types, default false.
3. Represent native result 5 distinctly from `Dead`.
4. Execute PostMortem only after the Object exact-zero callbacks/destroy call and
   at the verified Techno position. Do not preempt or roll back those effects.
5. Compute duration with the exact x87 operation/conversion sequence over the
   preserved caller distance.
6. Use one shared Building pending mechanism for OilExplosion PostMortem and
   infantry C4/Ivan, including source retention and mutual exclusion.
7. Preserve signed-i32 frame arithmetic and strict-shorter replacement.
8. Restore exactly life byte 1 and signed Health 1, then return PostMortem.
9. Place the expiry consumer in the Building update order, passing current HP,
   distance 0, C4Warhead, retained source, `ignore_defenses=1`, null source house,
   and final zero synchronously.
10. Cancel shared pending state at the Building IronCurtain/ForceShield entry.
11. Serialize the semantic pending state and hash native-equivalent remaining
    duration/latch/source identity.
12. Do not invent a semantic for `+0x52C`. Resolve the exact-byte policy as an
    explicit project decision or oracle/schema exception; until then keep
    authority blocked.

### Acceptance tests derived from this evidence

- Parser/default tests for all four rule/type fields and all three stock eligible
  objects.
- Raw-bit/x87 tests for `CellSpread` truncation, `<<8` wrapping, exact operation
  order, low-EAX `Math__ftol`, denominator-zero, NaN/Inf, signed negative, and
  overflow cases.
- Stock finite checkpoints at `d=0,34,35,256,512,768,1023,1024`.
- Eligibility matrix covering each of the five gates independently.
- Repeated-hit tests: shorter replaces; equal/longer keep metadata; all
  qualifying lethal repeats still restore HP/life and return 5.
- Ordered-effect test proving exact-zero callbacks/factory abandonment precede
  restoration and are not compensated.
- Result-5 Building wrapper test proving result-4 derived tail is skipped.
- Healing-before-expiry and ordinary-lethal-before-expiry tests.
- IronCurtain/ForceShield cancellation test.
- Shared infantry-plant/PostMortem source-retention and mutual-exclusion tests.
- Save/load and native-equivalent checksum tests, with raw `+0x52C` explicitly
  quarantined as unresolved byte state.
- Task-3/G2/G3 retail fixtures for each eligible stock type at exact producer
  distances and scheduler frames. Rust-vs-Rust fixtures are regression ratchets,
  not native parity proof.

## 13. Adversarial questions

| Challenge | Answer | Status/evidence |
|---|---|---|
| Can a nonlethal `OilExplosionWH` hit arm the timer? | No. The first gate is prior result exactly 4. | **VERIFIED**, `0x00701E71`. |
| Is `+0x1551` actually SelfHealing or a bridge flag? | No. The ReadINI string/data flow is `EligibleForDelayKill`. | **VERIFIED**, string `0x0081ACB0`, xref `0x00460224`. |
| Can a repeated qualifying hit extend the timer? | No. Equal/longer keeps existing; only strictly shorter overwrites. | **VERIFIED**, signed `JGE` at `0x00701F3F`. |
| Does HP=1 restoration undo kill credit, triggers, factory cancellation, or observer detach? | No compensating calls exist; those calls occur before the two restoration writes. | **VERIFIED control flow**. |
| Does healing cancel delayed death? | No latch clear is present; expiry uses then-current HP as forced damage. | **VERIFIED**. |
| Does IronCurtain merely block damage until it expires? | No. Building IronCurtain clears latch/source and zeros duration before delegation. | **VERIFIED**, `0x00457CA0`–`0x00457CC0`. |
| Is `+0x52C` a useful seed/facing field? | No use was verified; the arm reads undominated stack data, expiry/checksum omit it. | **UNKNOWN value; unsupported semantic**. |
| Is Oil Derrick itself a delayed-kill target? | No stock eligibility key exists on `CAOILD`; it is the producer of `OilExplosionWH`. | **VERIFIED INI**. |
| Can Rust keep separate C4 and Oil delay timers if the death frame matches? | No. Native producers share latch/timer/source, affect one another, and share cancellation/expiry. | **VERIFIED mechanism; separate timers are DRIFT**. |
| Does regular expiry clear the latch before recursive damage? | No. Only the BridgeRepairHut branch clears it. | **VERIFIED**, `0x00440320` versus `0x00440333`. |
| Are the stock duration numbers enough to certify end-to-end parity? | No. Producer arguments/order/timing and live native capture remain Task 3/G2/G3 work. | **DEFERRED**. |

## 14. Cold spot-checks

1. **Field identity from the string, not prior prose.** Starting at the literal
   `EligibleForDelayKill` string found one address (`0x0081ACB0`), one ReadINI
   xref (`0x00460224`), and an immediate store to `BuildingType+0x1551`. This
   independently rejects the old SelfHealing/Crewed/bridge labels.
2. **Expiry owner from the latch read, not the old Building Update report.** Starting
   at `BuildingClass::Update`'s read of `+0x6DF` at `0x004401D2` led through the
   signed timer test to the raw `+0x16C` call arguments at `0x00440333` and the
   distinct bridge-hut clear at `0x00440320`.
3. **Result 5 from raw jump-table bytes.** Starting at the post-Techno switch and
   reading `0x00442C18` mapped result 5 to `0x0044247D`, independently of imported
   pseudocode labels.

## 15. Coverage and open-question ledger

| Required item | Status | Closure |
|---|---|---|
| Rule/type identities and defaults | **RESOLVED** | Constructor, ReadINI strings, and field stores verified. |
| Eligibility | **RESOLVED** | Five exact gates and ordering verified. |
| Damage/distance inputs | **PARTIAL** | Damage role and receiver-side signed distance use resolved; producer provenance deferred to Task 3/G2. |
| Floating operation order/conversions/endpoints | **RESOLVED** | Instruction stack, CW, signed inputs, and stock endpoints verified. |
| Timer/latch/source fields | **PARTIAL** | Semantics closed except raw `+0x52C` value, which remains UNKNOWN. |
| Repeated-hit selection | **RESOLVED** | Strictly shorter wins; tie/longer keep existing. |
| Health/life restoration and result 5 | **RESOLVED** | Exact writes and return verified. |
| Retained/reversed effects | **RESOLVED for bounded slice** | Exact-zero/destroy calls retained; only life/HP universally restored; no scheduler reinsert inferred. |
| Cancellation/healing/source removal | **RESOLVED** | IC cancellation, no healing cancellation, source nulling verified. |
| Expiry owner/call | **RESOLVED** | Building Update signed timer and recursive argument set verified. |
| Save/load/checksum | **RESOLVED with byte blocker** | Raw persistence and semantic checksum fields verified; `+0x52C` remains raw UNKNOWN. |
| Stock eligible types and warhead data | **RESOLVED** | Complete current `rulesmd.ini` inventory. |
| Stock end-to-end retail invocation | **DEFERRED — Task 3/G2/G3** | Requires exact producer args/order/scheduler plus live fixtures. |
| Exact native value of `+0x52C` | **DEFERRED — authority blocker** | Process-stack-history dependent; no deterministic semantic value found. |

### Gate handoff

- **Task 2C behavioral mechanism:** COMPLETE.
- **Task 2C exact-byte result:** PARTIAL because `+0x52C` is UNKNOWN/raw persisted
  state.
- **G1:** do not certify exact byte parity until the project explicitly resolves
  the raw-state policy.
- **G2:** still required for projectile/effect impact scheduler placement.
- **Task 3:** still required for active producer `ignore_defenses`, exact per-target
  distance provenance, ordering, and special-producer arguments.
- **G3/Task 4:** still required for native retail trace/oracle fixtures; the
  coordinated input-lab session reported that it is not the damage Oracle owner
  and holds no damage manifest/export metadata.

## 16. Zero-add pass

The final pass re-read the bounded instruction blocks, the three constructor/
ReadINI field identities, current retail INI occurrences, and current Rust
touchpoints. No helper identity, offset, field name, content user, or dynamic
timing claim was added without direct evidence. Non-load-bearing helper internals
were intentionally not expanded. All remaining uncertainty is named above as
UNKNOWN or DEFERRED rather than converted into an implementation assumption.

