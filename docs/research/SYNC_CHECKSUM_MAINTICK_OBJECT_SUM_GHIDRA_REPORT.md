# SYNC CHECKSUM — Main_Tick Object Sum & Per-Object CRC Path
## Ghidra Research Report — Slot 1 / Re-Swarm 2026-05-28

**Report target:** The per-frame multiplayer SYNC CHECKSUM mechanism in `Main_Tick @ 0x0055D360`:  
specifically `FUN_00473ae0` (stream-write helper), `FUN_00473b10` (stream-read helper), the
`g_CurrentObjects` sum loop, the packed object-value encoding, and the relationship to the
per-object `*ComputeCRC` vtable path.

**Addresses:**
- `Main_Tick @ 0x0055D360`
- `FUN_00473ae0 @ 0x00473ae0` — recording stream **write** helper (wraps `FUN_00432050`)
- `FUN_00473b10 @ 0x00473b10` — recording stream **read** helper (wraps `FUN_004322a0`)
- `FUN_006d6170 @ 0x006D6170` — reads 8-byte state_hash from `ScenarioClass + 0xD64/0xD68`
- `FUN_006d6000 @ 0x006D6000` — validates + writes 8-byte state_hash back to `Scen + 0xD64/0xD74`
- `FUN_006e6ab0 @ 0x006E6AB0` — unpacks one object's packed type-index/kind value into a 2-word struct
- `FUN_0055f690 @ 0x0055F690` — packs a 32-bit per-object value into a local struct during replay verify
- `AbstractClass__ComputeCRC @ 0x00410410`
- `BombClass__ComputeCRC @ 0x00438A90`
- `CRCEngine__AddData @ 0x004A1DE0`

**Confidence:** HIGH on the full recording/playback checksum block structure, HIGH on the per-object
sum formula and which objects are iterated, HIGH on `FUN_00473ae0/b10` semantic roles. The per-object
`*ComputeCRC` / `CRCEngine` path is a **separate mechanism** that feeds network desync detection (not
the recording stream checksum). Callers of `FootClass__ComputeChecksum` (`0x004DBAD0`) and
`FactoryClass__vtable_13` (`0x004CA430`) are dead-end saves/convoy path with no active live callers
to confirmed YR network desync detection in this investigation.

**Active in YR:**
- Recording/playback checksum block (`DAT_00A8D5F8 & 1` or `& 2`): Conditional — active only when
  `g_GameMode == 0` (single-player) in recording mode. The object-sum loop is the checksum for the
  recording/playback stream fidelity. **Active in standard single-player with recordings. NOT the
  live multiplayer desync detector.**
- Per-object `*ComputeCRC` vtable path: Called by `Save`/`Load` functions and some `ComputeChecksum`
  methods. No confirmed live network desync caller path found this session — see Remaining Uncertainty.

---

## Investigation Preflight

- **Target question:** What exactly do `FUN_00473ae0`/`FUN_00473b10` do, what do they sum, which
  fields contribute to the per-object value, and how does this relate to the `*ComputeCRC` vtable path?
- **Non-goals:** RNG algorithm (slot 2), desync-comparison behavior/on-mismatch network messaging
  (slot 4), PerTickUpdate subsystem ordering (slot 5).
- **Evidence needed to mark COMPLETE:** (1) Both helpers' roles fully understood. (2) Per-object value
  formula verified from binary. (3) Iteration set identified (g_CurrentObjects / selection set). (4)
  ComputeCRC path's caller set established. (5) Whether these two mechanisms cross-feed each other.
- **Stop conditions:** All five points answered with direct Ghidra decompilation evidence.

---

## 1. FUN_00473ae0 — Stream Write Helper

Verified via `decompile_function 0x00473ae0`.

```
void __thiscall FUN_00473ae0(int *param_1, undefined4 param_2, undefined4 param_3)
{
    if (param_1[0x16] != 0) {       // stream buffer is valid/active
        iVar1 = *param_1;
        uVar2 = (*vtable+4)();      // get current stream position
        (*vtable+0x40)(0xD, 0, uVar2); // error if seek fails?
    }
    FUN_00432050(param_2, param_3); // actual WriteFile via CRCPipe/FileClass wrapper
    return;
}
```

`FUN_00432050 @ 0x00432050` confirmed as the write-side pipe class (`WriteFile` at `FUN_0065cdd0`
inside). It takes `(data_ptr, byte_count)` and writes exactly `param_3` bytes from `param_2` to the
recording file handle.

**Call convention in Main_Tick:** `FUN_00473ae0(&some_local, N)` where `some_local` is the value to
write and `N` is the byte count. The first argument is a POINTER to the value, not the value itself.
Verified via `decompile_function 0x0055D360` — all call sites pass `&local_var`.

**`FUN_00473ae0` only fires when `DAT_00A8D5F8 & 1` (recording mode).**

---

## 2. FUN_00473b10 — Stream Read Helper

Verified via `decompile_function 0x00473b10`.

```
int __thiscall FUN_00473b10(int *param_1, undefined4 param_2, int param_3)
{
    // vtable+0x18 = IsEOF? check
    // if not EOF: try Request(1) → bVar1 = true on success
    if (param_1[0x16] == 0) {
        param_3 = FUN_004322a0(param_2, param_3); // ReadFile via Straw/Pipe
        if (bVar1) { (*vtable+0x34)(); }
    } else {
        iVar3 = param_1[0x17] - param_1[0x19];  // remaining = total - consumed
        if (iVar3 < param_3) param_3 = iVar3;
        if (param_3 != 0) {
            FUN_007ca090(param_2, param_1[0x19] + param_1[0x16], param_3); // memmove from buffer
            param_1[0x19] += param_3;
        }
        if (bVar1) { (*vtable+0x34)(); return param_3; }
    }
    return param_3;
}
```

`FUN_004322a0 @ 0x004322a0` is the read-side pipe (`ReadFile` at `FUN_0065cce0`). Returns number of
bytes actually read.

**`FUN_00473b10` only fires when `DAT_00A8D5F8 & 2` (playback mode).** Returns the count of bytes
read; Main_Tick checks `if (iVar4 == N)` before treating the read data as valid.

`FUN_007ca090 @ 0x007CA090` is confirmed as the standard optimized `memmove` (handles overlapping
buffers with forward/backward switching). Not a CRC or hash function.

---

## 3. Main_Tick Recording Block — Complete Annotated Sequence

Verified via `decompile_function 0x0055D360`. The entire block is gated by
`if ((DAT_00A8D5F8 & 1) || (DAT_00A8D5F8 & 2))`.

### 3.1 Recording Path (`DAT_00A8D5F8 & 1`)

```
Step 1: pDVar7 = FUN_006d6170(g_ScenarioClass_Instance, local_180)
        // reads Scen+0xD64 (4 bytes) and Scen+0xD68 (4 bytes) into local_180 (8 bytes)
        // local_180 = { Scen[0xD64], Scen[0xD68] }
        // verified via decompile_function 0x006D6170

Step 2: local_1b4 = local_180[0]; local_1b0 = local_180[1]
        // unpack from the 8-byte struct into two locals

Step 3: FUN_00473ae0(&local_1b4, 8)
        // WRITE 8 bytes to stream: the 8-byte state_hash from ScenarioClass

Step 4: local_1a8 = g_CurrentObjects_Count
        FUN_00473ae0(&local_1a8, 4)
        // WRITE 4 bytes: number of currently selected objects

Step 5: // First pass over g_CurrentObjects — compute SUM:
        local_1a4 = 0
        for (iVar4 = 0; iVar4 < local_1a8; iVar4++) {
            puVar8 = FUN_006e6ab0(g_CurrentObjects_Data[iVar4])
            // puVar8 is a 2-word result:
            //   puVar8[0] = raw value (3 bytes: type-index or coordinate lower 24 bits)
            //   puVar8[1] byte = type-kind byte (RTTI discriminant or 0x34 for standard objects)
            if ((byte)puVar8[1] == 0) {
                uVar12 = 0xFFFFFFFF   // null/invalid object sentinel
            } else {
                uVar12 = (uint)(byte)puVar8[1] << 24 | (*puVar8 & 0xFFFFFF)
                // packs TypeKind into high byte, type-index into low 24 bits
            }
            local_1a4 += uVar12      // ADDITIVE sum (wraps on overflow)
        }
        FUN_00473ae0(&local_1a4, 4)
        // WRITE 4 bytes: the cumulative sum of packed type-index|kind values

Step 6: // Second pass — write EACH object's packed value individually:
        for (iVar4 = 0; iVar4 < local_1a8; iVar4++) {
            puVar8 = FUN_006e6ab0(g_CurrentObjects_Data[iVar4])
            if ((byte)puVar8[1] == 0) {
                local_194 = 0xFFFFFFFF
            } else {
                local_194 = (uint)(byte)puVar8[1] << 24 | (*puVar8 & 0xFFFFFF)
            }
            FUN_00473ae0(&local_194, 4)
            // WRITE 4 bytes per object
        }

Step 7: FUN_00473ae0(&DAT_00ABCDFC, 4)
        FUN_00473ae0(&DAT_00ABCE00, 4)
        // WRITE 8 bytes: mouse / cursor position state
        // then clear both globals to 0
        _DAT_00ABCDFC = 0; _DAT_00ABCE00 = 0
```

Total written per frame (when recording): 8 + 4 + 4 + (4 × N_selected) + 4 + 4 bytes.

### 3.2 Playback Path (`DAT_00A8D5F8 & 2`)

```
Step 1: iVar4 = FUN_00473b10(&local_1b4, 8)
        if (iVar4 == 8) { FUN_006d6000(&local_1b4) }
        // READ 8 bytes → validate/commit state_hash to Scen+0xD64/0xD74
        // FUN_006d6000 also writes to +0xD68/0xD78 and sets +0xD7D = 1
        // verified via decompile_function 0x006D6000

Step 2: iVar4 = FUN_00473b10(&local_1a8, 4)
        if (iVar4 == 4) {
            // Recompute local sum (same formula as recording step 5):
            local_1a4 = 0
            for all g_CurrentObjects_Count objects {
                packed = FUN_006e6ab0(g_CurrentObjects_Data[i])
                compute uVar12 same formula as above
                local_1a4 += uVar12
            }
            
            // READ expected sum:
            FUN_00473b10(&local_190, 4)
            
            // COMPARE:
            if (local_190 != local_1a4) { Desync_Handler() }
            // Desync_Handler @ 0x0048DC90 clears the selection list, NOT a network alert
            
            // Re-select objects from stream:
            g_SelectionVoice_Enable = 1
            for each of local_1a8 objects {
                iVar6 = FUN_00473b10(&local_194, 4)
                if (iVar6 == 4) {
                    local_198 = 0
                    FUN_0055f690(local_180, local_194)
                    // FUN_0055f690 unpacks the 32-bit packed value back to a 2-word struct
                    piVar16 = FUN_006e6ff0()  // look up object by packed value (binary search)
                    if (piVar16 && local_190 != local_1a4) {
                        (*piVar16 vtable+0x14C)()   // unselect object
                        g_SelectionVoice_Enable = 0
                    }
                }
            }
            g_SelectionVoice_Enable = 1
        }

Step 3: FUN_00473b10(&DAT_00ABCDFC, 4)
        FUN_00473b10(&DAT_00ABCE00, 4)
        FUN_004f42f0(0)    // apply mouse/cursor state
        RenderFrame_main()
```

---

## 4. Per-Object Value Encoding — FUN_006e6ab0

Verified via `decompile_function 0x006E6AB0`. The function returns a 2-word struct:

```c
struct ObjectValue {
    uint32_t raw_value;      // [word 0]: depends on RTTI branch
    uint8_t type_kind;       // [word 1, byte 0]: RTTI discriminant (0 = null, 0xB = coord?, 0x34 = standard object)
};

if (param_2 == NULL) {
    result.type_kind = 0;
    result.raw_value = 0;
    return;
}
iVar2 = (*vtable+0x2C)();   // GetTypeID / RTTI type selector
if (iVar2 == 0xB) {
    result.type_kind = 0x0B;
    // special coord-like packing: result.raw_value = low16 + high16 * 1000
    param_2._2_2_ = short(uint(*piVar1) >> 16);
    result.raw_value = (short)*piVar1 + param_2._2_2_ * 1000;
    return;
}
// Standard path (iVar2 != 0xB):
result.type_kind = 0x34;
iVar2 = (*vtable2[0x10])(param_2 + 1);  // fetch_id: gets object's numeric heap-pool ID
result.raw_value = iVar2;
return;
```

The packed 32-bit value written to stream is:
```
if (type_kind == 0) → 0xFFFFFFFF  (null sentinel)
else                 → (type_kind << 24) | (raw_value & 0xFFFFFF)
```

This is **not a hash** — it is a compact type-identity encoding. The "sum" is an arithmetic checksum
over object identities, not a CRC or cryptographic digest.

---

## 5. What g_CurrentObjects Is

The loop iterates `g_CurrentObjects_Data` (array of pointers) with count `g_CurrentObjects_Count`.
This is the **current selection set** — the set of objects the local player has currently selected.
Verified by cross-referencing callers: `DisplayClass__BandBox_LeftUp @ 0x004AB9B0`,
`FootClass__ClickedAction_Object @ 0x004D74E0`, `BuildingClass__SetRallyPoint @ 0x00443860`, etc.,
all point to a selection-management array.

This is **NOT** `LogicClass`'s active-object vector. The checksum is over the local player's
**selection state**, not the full sim state.

---

## 6. Per-Object *ComputeCRC / CRCEngine Path — Separate Mechanism

Verified via `decompile_function 0x00410410`, `0x00438A90`, `0x004A1DE0`, `0x004DBAD0`,
`get_function_callers 0x00410410`, `get_function_callers 0x004DBAD0`.

### 6.1 AbstractClass__ComputeCRC @ 0x00410410

```c
void AbstractClass__ComputeCRC(int param_1) {
    FUN_004a1d50(*(param_1 + 0x10));   // hashes ID field at +0x10 (4 bytes via CRCEngine)
    FUN_004a1ca0(*(param_1 + 0x20));   // hashes Owner field at +0x20 (1 byte via CRCEngine)
}
```

Fields: object ID at `+0x10`, owner byte at `+0x20`. All CRC-32 via `CRCEngine__AddData @ 0x004A1DE0`
with lookup table at `0x0081F7B4`.

### 6.2 CRCEngine__AddData @ 0x004A1DE0

Standard CRC-32 with Ethernet polynomial via `DAT_0081F7B4` (256-entry 32-bit lookup table).
Processes bytes in groups of 4. Handles partial final word by padding. Returns `~crc_accumulator`
at end (standard CRC-32 inversion). No 64-bit output — this is plain 32-bit CRC.

### 6.3 Other *ComputeCRC Methods

Confirmed callers chain: `BombClass__ComputeCRC` → hashes `+0x28`, `+0x24`, `+0x2C` linked object
IDs + `+0x58` bool. `SpawnManagerClass__ComputeCRC` hashes timer fields, spawn target IDs.
`FootClass__ComputeChecksum @ 0x004DBAD0` hashes position vectors, velocity magnitude (via sqrt!),
destination, convoy IDs, bool flags. `FactoryClass__vtable_13 @ 0x004CA430` hashes production queue
state (IsSuspended, IsDifferent, Balance, OriginalBalance, object IDs, house ID).

### 6.4 ComputeCRC Caller Chain — Active in YR?

`get_function_callers 0x004DBAD0` → only caller is `FootClass__Save_Convoy_State @ 0x00744640`.
`get_function_callers 0x00744640` → **no callers found**.

`get_function_callers 0x004CA430` (FactoryClass__vtable_13) → **no callers found**.

`get_function_callers 0x006CE910` (SuperWeaponTypeClass__ComputeChecksum) → **no callers found**.

`get_function_callers 0x00512170` (TypeClass sub-ComputeCRC) → **no callers found**.
`get_function_callers 0x007171A0` (TechnoTypeClass__ComputeCRC-like) → **no callers found**.

**Verdict:** The `*ComputeCRC` vtable family and `CRCEngine__AddData` are NOT called from the
recording/playback checksum block in Main_Tick. They are called from Save/Load routines and some
detached convoy/checksum paths. No confirmed live YR desync-detection caller was found in this
investigation. This is a **separate mechanism** from the object-sum loop in Main_Tick's recording
block.

---

## 7. ScenarioClass State Hash Fields

`FUN_006d6170 @ 0x006D6170` reads exactly 8 bytes: `Scen + 0xD64` (4 bytes) and `Scen + 0xD68`
(4 bytes) into the output buffer. These are the game's opaque "state hash" fields embedded in
ScenarioClass.

`FUN_006d6000 @ 0x006D6000` writes the 8-byte value back into `Scen + 0xD64` and `Scen + 0xD74`
(same low 4 bytes to two slots) and `Scen + 0xD68` and `Scen + 0xD78`. Also calls
`FUN_006d8b30()` and sets `*(Scen + 0xD7D) = 1`. The function validates via `FUN_006d8640` before
committing; if validation fails or map-editor mode is active, it uses the input value directly.

The **content** of these 8 bytes (what subsystem computes them, what fields they cover) is **outside
the scope of this investigation** (Remaining Uncertainty). They are simply read from and written to
the ScenarioClass state blob.

---

## 8. Implementation Handoff

### 8.1 Handoff: Recording/Playback Checksum Is Selection-Based, Not Sim-Based

**Verified behavior:** The Main_Tick recording block checksums the **local player's selection list**
(`g_CurrentObjects`), not the full simulation state. It is a replay-stream consistency check, not a
multiplayer desync detector. The "sum" is arithmetic over packed `(TypeKind << 24 | HeapPoolID)` values.

**Rust delta:** `Simulation::state_hash()` in `src/sim/world/world_hash.rs` hashes comprehensive sim
state (entities, production, fog, bridges, etc.). This is the RIGHT approach for multiplayer desync
detection. There is **no analog of the selection-list checksum** in the Rust port — which is correct
for multiplayer since it is a recording-only mechanism.

**Affected surface:** If a recording/replay feature is added to Rust, the replay stream must include
a selection-list checksum using this exact formula (packed TypeKind|HeapPoolID, additive sum,
sentinel 0xFFFFFFFF for null). The `Simulation::state_hash` is NOT the same as this recording
checksum.

**Acceptance scenario:** Replay a recorded single-player session; on frame N the selection set
differs from what the recording stream encoded — the checksum mismatch triggers `Deselect_All`
(not a desync alert).

**Proposed test name:** `test_recording_selection_checksum_sentinel_and_packing`

**Risk:** MEDIUM — recording feature is not yet in scope but this formula must be exact when added.

### 8.2 Handoff: *ComputeCRC / CRCEngine Path Has No Confirmed Live YR Network Caller

**Verified behavior:** No live caller chain for `FootClass__ComputeChecksum`, `FactoryClass__vtable_13`,
`SuperWeaponTypeClass__ComputeChecksum`, or `TechnoTypeClass::ComputeCRC-like` was found. All
confirmed callers lead to Save/Load paths or dead-end convoy functions.

**Rust delta:** Do NOT implement a CRCEngine-based per-object hash for live multiplayer desync
detection. The existing `Simulation::state_hash()` (DefaultHasher) covers the correct sim surface.

**Affected surface:** `src/sim/world/world_hash.rs` — no change needed for this finding.

**Acceptance scenario:** Multiplayer desync detection using `state_hash()` remains the port's
mechanism. No CRC-32 object loop to wire up.

**Proposed test name:** `test_crc_engine_path_not_used_for_live_desync` (negative test: document that
ComputeCRC callers are dead in standard YR)

**Risk:** LOW — the concern is accidentally implementing a CRC-32 loop that doesn't match the actual
in-use mechanism.

### 8.3 Handoff: Rust state_hash Covers More Fields Than the gamemd Recording Sum

**Verified behavior:** gamemd's recording-stream per-frame hash covers only: 8-byte ScenarioClass
state_hash blob + selection-list packed type-kind/ID arithmetic sum + cursor position (2 × 4 bytes).
It does NOT hash: entity positions, health, movement state, production queues, fog, etc.

**Rust delta:** `Simulation::state_hash()` in `world_hash.rs` hashes all of those. This is strictly
better for multiplayer correctness. No reduction is needed; the Rust hash is not mimicking the
recording-stream mechanism.

**Affected surface:** `src/sim/world/world_hash.rs` — confirm that `Simulation::state_hash()` is
used for multiplayer desync detection, NOT for recording stream fidelity checks.

**Proposed test name:** `test_sim_state_hash_used_for_desync_not_recording_stream_sum`

**Risk:** LOW (documentation/contract risk, not an implementation risk).

---

## 9. Negative Facts / Do Not Do

1. **Do NOT treat `FUN_00473ae0` or `FUN_00473b10` as CRC-computing functions.** They are file I/O
   helpers wrapping `WriteFile`/`ReadFile` pipe wrappers. Verified via `decompile_function 0x00473ae0`,
   `decompile_function 0x00432050`, `decompile_function 0x0065cdd0`.

2. **Do NOT treat `g_CurrentObjects` as the full sim active-object set.** It is the selection list.
   Verified by cross-referencing callers of `FUN_006E6AB0` in Main_Tick: `DisplayClass__BandBox_LeftUp`,
   `FootClass__ClickedAction_Object`, and the Main_Tick count variable `g_CurrentObjects_Count`.

3. **Do NOT implement the `*ComputeCRC` / `CRCEngine__AddData` chain for live multiplayer desync.**
   No confirmed live YR caller exists. Verified via `get_function_callers 0x004DBAD0` and
   `get_function_callers 0x004CA430` returning empty.

4. **Do NOT treat the recording-stream checksum as the multiplayer desync detector.** The
   `DAT_00A8D5F8 & 1`/`& 2` gate is recording/playback only. Live MP desync detection is in the
   network path (slot 4 scope). Verified via Main_Tick decompilation control flow.

5. **Do NOT use `FUN_007ca090` as a hashing function.** It is `memmove` (optimized overlapping
   copy). Verified via `decompile_function 0x007CA090` — the function performs byte-by-byte or
   word-by-word copy with forward/backward direction selection.

---

## 10. Remaining Uncertainty

- **What subsystem writes `Scen + 0xD64` / `Scen + 0xD68`?** The 8-byte state_hash read by
  `FUN_006d6170` and committed by `FUN_006d6000`. Neither this report nor the §5.2 RNG doc
  identifies the writer. This is outside this investigation's scope but is needed to understand
  whether the recording stream's "state_hash" covers anything relevant to multiplayer parity.

- **Is there a separate live-MP per-frame desync checksum beyond the recording stream?** The
  `*ComputeCRC` / `CRCEngine` family has no confirmed live callers in YR. It is possible a
  separate function enumerates the `LogicClass` vector and calls each object's vtable `*ComputeCRC`
  slot, producing a per-frame CRC for broadcast. This was not found in this session. Slot-4
  (desync comparison/on-mismatch) should investigate `FUN_004F42F0` (called after the playback
  block) and the `Network_ServiceLoop` call chain.

- **`FUN_006e6ab0` when `vtable+0x2C` returns `0xB`:** The `0xB` type_kind branch packs coordinates
  rather than a heap ID. What object class has RTTI type 11 (0xB)? Likely an overlay or terrain
  tile class. Not impactful for the sum formula but relevant if extending the selection encoding.

---

## 11. Stale Content in RNG_SYSTEM_GHIDRA_REPORT.md §5.2

Section §5.2 of `docs/research/RNG_SYSTEM_GHIDRA_REPORT.md` says:

> `sum = SUM over selected objects of (TechnoTypeIndex | (TypeKind << 24));`

**Correction:** The packed value is `(TypeKind_byte << 24) | (raw_value & 0xFFFFFF)` where
`raw_value` is the object's **heap-pool numeric ID** (not the TechnoTypeIndex). The TypeKind byte
is `0x34` for standard objects (not a TechnoType discriminant). For null objects the value is
`0xFFFFFFFF`. The "TechnoTypeIndex" description is inaccurate.

Also §5.2 says `Desync_Handler()` is called when the sum mismatches. The **correct behavior** is
that `Desync_Handler @ 0x0048DC90` clears the selection list (deselects all objects) and resets
selection mode — it does NOT issue a network desync alert. This is already noted in §5.2 of the
existing doc, but the caller description is worth tightening.

No doc edit was made (read-only constraint during swarm). Proposed replacement for §5.2's "sum"
line:

```
sum = SUM over selected objects of: (0xFFFFFFFF if null) else (heap_id_byte << 24 | object_heap_pool_id & 0xFFFFFF)
```
