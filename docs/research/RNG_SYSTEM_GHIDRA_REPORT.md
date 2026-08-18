# RNG System — Ghidra Research Report

**Addresses:**
- `0x0065C7E0` — `Random__RandomRanged(this, low, high)` (primary game RNG entry point)
- `0x0065C780` — `Random__Next(this)` (single-call RNG step, used internally)
- `0x0065C6D0` — `RandomClass_Seed(this, u32 seed)` (state init from u32 seed)
- `0x0052FC20` — `Init_Random_Number_System` (game-start RNG init / entropy gather / dual-instance seeding)
- `0x00886B88` — `g_MainRng` (static RandomClass instance, 0x3F4 bytes) — main game RNG
- `Scen + 0x218` — `Scen->Random` (RandomClass instance inside heap-allocated ScenarioClass)
- `0x00ABE890` — `g_MapGenRng` (static RandomClass instance) — used only by random-map generator
- `0x00A8ED94` — `g_SeedU32` (4-byte seed; source for all three RandomClass instances at init)
- `0x00839644`, `0x00839694` — key-mixing tables used by `RandomClass_Seed`; **only first 4 dwords (16 bytes) of each are read** by the Feistel loop (corrected 2026-05-28: was "16-dword each" — decompile_function 0x0065c6d0 — ROOT_CAUSE: INFERENCE_HARDENED)

**Confidence:** HIGH (algorithm, layout, semantics, seed pipeline, dual instances — all verified by decompilation and call-site assembly)

**Active in YR:** Yes — all three RandomClass instances are used in every standard YR skirmish, single-player mission, and multiplayer game.

---

## 1. Overview

YR's RNG is a **lagged Fibonacci generator with XOR** — specifically `R(250, 103)` — using a 250-entry state of `u32` words and two phase-locked indices at offset 103. There is **no LCG, no Mersenne Twister, no xorshift, no CRT `rand()`** in the simulation path.

Three independent RandomClass instances exist, all seeded with the same `u32` seed at game start: `g_MainRng` (general sim), `Scen->Random` (persisted with ScenarioClass save state), and `g_MapGenRng` (random-map generator only). All three drift independently as the game consumes them at different points.

Seed sourcing:
- **Single-player / skirmish:** SHA-1 entropy pool stirred with `GetSystemTime` fields and `GetTickCount` → 4 bytes extracted → `g_SeedU32`. Triggered when `g_GameMode == 0` (campaign/SP) or `g_GameMode == 5` (offline skirmish) — both confirmed via `decompile_function 0x0052fc20`.
- **Multiplayer (Internet / LAN):** Seed is established by network handshake **before** `Init_Random_Number_System` runs (the function's gating flag `DAT_00A8B8B8` signals "seed already set, skip entropy gather").
- **Replay playback:** Seed is read from the recording file as the second 4-byte field.
- **Recording save:** Seed is written to the recording file at the start of recording.

This makes the RNG deterministic across all clients in a multiplayer game (assuming identical consumption order) and across replays of the same game.

---

## 2. RandomClass — Layout and Algorithm

### 2.1 Struct Layout (size = `0x3F4` bytes = 1012)

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| `0x000` | `u8`  | `disabled_flag` | If non-zero, RNG never advances; all draws return `low` and state is untouched. Initialized to `0` (active) by `RandomClass_Seed`. |
| `0x001` | `u8[3]` | padding | Unused; 3 bytes of alignment. |
| `0x004` | `i32` | `index_a` | First state cursor. Seeded to `0`. Increments by 1 per draw; wraps when it would become `250`. |
| `0x008` | `i32` | `index_b` | Second state cursor. Seeded to `103` (`0x67`). Increments by 1 per draw; wraps when it would become `250`. Phase-locked to `index_a` (offset stays 103). |
| `0x00C` | `u32[250]` | `state[]` | The lagged-Fibonacci state. Filled at seed time by 250 iterations of a 4-round Feistel-like mixer (see §2.4). |

Total: `0x00C + 250*4 = 0x3F4` bytes per instance.

### 2.2 Algorithm — `Random__Next` at `0x0065C780`

Single-step pseudocode (verified from binary):
```
if (this.disabled_flag != 0) return 0;
this.state[this.index_a] ^= this.state[this.index_b];
result = this.state[this.index_a];
this.index_a += 1;
this.index_b += 1;
if (this.index_a > 249) this.index_a = 0;   // i.e. iVar3 > 0xF9 -> reset
if (this.index_b > 249) this.index_b = 0;
return result;
```

This is **R(250, 103) XOR Lagged Fibonacci**. Maximal period for this lag pair with non-degenerate initial state is approximately `(2^250 - 1) * 2^31`. Effectively infinite for game purposes. Not cryptographically secure — that is not a requirement here.

### 2.3 Range-bounded Draw — `Random__RandomRanged(this, low, high)` at `0x0065C7E0`

Faithful pseudocode (verified line-by-line from binary):
```
if (low == high) return low;                 // EARLY-OUT: no state advance, no consumption

if (high < low) swap(low, high);             // Negative ranges are normalized (swap)

range = (u32)(high - low);                   // INCLUSIVE-INCLUSIVE: returns value in [low, high]

// 1. Find the MSB index of `range` to know how many bits to draw.
msb = 31;
if ((range & 0x80000000) == 0) {             // If MSB not set, scan down...
    while (msb > 0 && (range & (1 << msb)) == 0) msb -= 1;
}                                            // else (range > 2^31), msb stays at 31.

// 2. Rejection sample: draw a random value, mask to (msb+1) bits, retry if > range.
mask = ~(0xFFFFFFFF << (msb + 1));           // i.e. (1 << (msb+1)) - 1, except correct for msb=31
draw = range + 1;                            // force entry into the loop
while ((i32)range < (i32)draw) {
    if (this.disabled_flag != 0) { draw = 0; break; }    // disabled -> always 0
    draw = Random__Next(this) & mask;
}

return low + draw;
```

**Crucial properties** (all verified):
- **Inclusive on BOTH ends.** `RandomRanged(1, 4)` returns one of `{1, 2, 3, 4}`.
- **`low == high` does NOT consume an RNG draw.** This matters for parity: a code path that calls `RandomRanged(N, N)` does not advance state.
- **`disabled_flag != 0` does NOT consume an RNG draw.** Returns `low`.
- **Uniform distribution.** Rejection sampling avoids the modulo bias of `next() % range`. Cost: ~2x average draws in worst case (range = 2^k + 1).
- **`low > high` is silently swapped** — no error, no warning. The result is still in `[min(low,high), max(low,high)]` inclusive.
- **The signed-vs-unsigned comparison `(i32)range < (i32)draw`** is technically suspect for ranges with MSB set (range > 0x7FFFFFFF). In practice no game caller passes such a range; all observed callsites use small bounded ranges (≤ a few thousand).

### 2.4 Seed Init — `RandomClass_Seed` at `0x0065C6D0`

Takes `(this, u32 seed)`. Pseudocode (verified):
```
this.disabled_flag = ???                     // not touched here; cleared at END
this.index_a = 0;
this.index_b = 0x67;                         // = 103
seed_table_1 = (u32*)0x00839644;             // only first 4 dwords (16 bytes) read by Feistel loop (corrected 2026-05-28: was "16 dwords"; binary loop exits at iVar6==0x10 accessing byte offsets 0,4,8,12 only — decompile_function 0x0065c6d0 — ROOT_CAUSE: INFERENCE_HARDENED)
seed_table_2 = (u32*)0x00839694;             // only first 4 dwords (16 bytes) read by Feistel loop (corrected 2026-05-28: was "16 dwords"; same loop — decompile_function 0x0065c6d0 — ROOT_CAUSE: INFERENCE_HARDENED)
counter = 0;
for (i = 0; i < 250; i++) {
    a = seed;
    b = counter;
    for (round = 0; round < 4; round++) {       // 4 Feistel-like rounds (loop bound iVar6 < 0x10 where iVar6=iVar5+4; corrected 2026-05-28: was "iVar5 < 0x10" — decompile_function 0x0065c6d0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
        mixed = seed_table_1[round] ^ b;
        hi = (i32)mixed >> 16;
        lo = mixed & 0xFFFF;
        ext = ~(hi*hi) + (lo*lo);               // = lo*lo - hi*hi - 1   (signed)
        next = ((rotl16(ext)) ^ seed_table_2[round]) + hi*lo;
        next ^= a;
        a = b;
        b = next;
    }
    this.state[i] = b;
    counter += 1;
}
this.disabled_flag = 0;                      // Activate the RNG
```

Where `rotl16(x) = (x >> 16) | (x << 16)` (verified from `iVar4 >> 0x10 | iVar4 * 0x10000`).

The two tables at `0x00839644` and `0x00839694` are static data. The Feistel loop reads **only the first 4 dwords (16 bytes) of each table** at byte offsets 0, 4, 8, 12. (corrected 2026-05-28: was "64-byte (16 × u32) constants"; loop bound `iVar6 < 0x10` with step 4 reads exactly 4 entries — decompile_function 0x0065c6d0 — ROOT_CAUSE: INFERENCE_HARDENED.) The physical extent of the tables in BSS extends beyond 16 bytes but is irrelevant to the algorithm. **First 64 bytes of `0x00839644`** (verified by `read_memory`; only the first 16 bytes / 4 dwords are consumed by the seed function):
```
87 68 a9 ba   2c d3 17 1e   3c dc bc 03   b2 d1 33 0f
1d 49 a6 76   5d d8 70 c5   e3 b1 82 e3   62 43 db 78
d4 a9 39 74   c5 8a ea 9c   5c 7c 53 89   5d f5 88 25
1d 5e 5b 41   95 3d 6e 21   e7 62 c6 85   68 b3 8a 5e
8c cc a5 3e   74 0f 6a d2   2b 22 a9 f3   e4 d7 aa 48   ...
```

These two tables must be reproduced exactly to match gamemd's output. Otherwise the same seed will produce a different state and every roll diverges.

### 2.5 Magic / Constants

- `0xFA = 250` — state array length and index wrap threshold.
- `0x67 = 103` — initial value of `index_b` (the lag). Determines the recurrence relation `state[i] ^= state[(i+103) mod 250]` per draw.
- `0x80000000` — MSB probe in range-finding.
- `0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0` — SHA-1 initialization vector (used in the **entropy pool**, not in RandomClass itself).
- `0x800 = 2048` — bit-count threshold in the entropy pool; when the per-bit counter reaches this, the pool is mixed via SHA-1.
- `0x14 = 20` — SHA-1 digest size in bytes; used by entropy mixer.

---

## 3. Three RandomClass Instances — Roles

### 3.1 `g_MainRng` at `0x00886B88` (static BSS)

The main game RNG, used by the vast majority of callsites. Confirmed callers (verified by xref + assembly inspection):
- `WarheadTypeClass__Detonate` — scatter / damage rolls
- `SoundEvent__AdvancePlaylist`, `SoundEvent__LoadSamples`, `SoundEvent__SelectNextSample` — sound variant selection
- `BuildingClass__Mission_Missile` — missile launch jitter
- `EBolt__DrawRecursiveBolt`, `EBolt__Init` — Tesla bolt branching
- `LaserDrawClass__Draw` — laser jitter
- `RadBeam__DrawAndTickAll` — rad beam wobble
- `HouseClass__Update` — house AI rolls (verified at `0x004F887D`: `MOV ECX, 0x886B88; CALL Random__RandomRanged`)
- `FootClass__AI`, `InfantryClass__PerCellProcess`, `UnitClass__PerCellProcess` — per-cell behavior rolls
- `TechnoClass__ReceiveDamage` — damage-response rolls
- `TechnoClass__IncreaseGattlingStage`, `TechnoClass__SpawnRadEruption` — particle spawn rolls
- `LightningStorm__GroundStrike`, `LightningStorm__Process`, `LightningStorm__Start` — lightning bolt direction

This RNG is consumed both by sim-deterministic logic and by visual/sound code. All clients in a multiplayer game produce the same sequence because the seed is shared. **Visual code consuming from the same RNG as sim is intentional** — it keeps the consumption order well-defined.

`g_MainRng` is **not saved with savegames**. On load, it would either restart from a re-derived seed or carry whatever stale state it had — see Open Questions §7.

### 3.2 `Scen->Random` at `Scen + 0x218`

A RandomClass instance embedded in the heap-allocated ScenarioClass at offset `0x218`. Confirmed callers via assembly inspection:
- `InfantryClass__Scatter` at `0x0051D2AC` and `0x0051D36D` — direction roll `RandomRanged(0, 4)` for scatter facing
- `HouseClass__Update` at `0x004F88FA` — cell-state-conditional roll `RandomRanged(0, 2)`
- A conditional roll in `Main_Tick` at `0x004F8895` near MP/LAN mode

This RNG is **part of ScenarioClass state**, which means it travels with savegame serialization (when saves include scenario state) and replay recordings. Inferred role: persistent sim sub-stream that must produce identical sequences after save/load.

**Note:** `g_ScenarioClass_Instance` at static address `0x00A8B230` is a **pointer** to the heap-allocated instance, not the instance itself. Accesses look like `MOV EAX, [0x00A8B230]; LEA ECX, [EAX + 0x218]`. This is why direct `0x00A8B448` xrefs returned no results.

**Per-call-site RNG-instance binding (g_MainRng vs Scen->Random) is verified at assembly level** (cross-ref `RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE_GHIDRA_REPORT.md` §8; re-confirmed 2026-05-29 via `disassemble_function 0x004F8440`): inside `HouseClass__Update`, the two house-AI rolls bind `g_MainRng` (`0x004F887D` / `0x004F8895`: `MOV ECX, 0x886B88; CALL Random__RandomRanged 0x0065C7E0`), while the cell-state-conditional roll binds `Scen->Random` (`0x004F88FA`: `MOV EAX, [0x00A8B230]; LEA ECX, [EAX + 0x218]; CALL 0x0065C7E0`). The binding is not visible in the C decompile (both render as bare `RandomRanged` calls) but is unambiguous in the disassembly — so this is no longer an open question.

### 3.3 `g_MapGenRng` at `0x00ABE890` (static BSS)

A RandomClass instance used **only by the random-map generator** (`FUN_00598960`). Seeded from a separate map-seed field on the LoadOptionsClass / `MapSeedClass` (`ScenarioClass__Constructor + 0x74` source). All ~150 callsites for this instance (xrefs in the `0x58????` – `0x5A????` range) are random-map terrain/water/region/tiberium generation helpers.

This is not part of normal game-tick RNG and never gets consumed during a normal mission or skirmish — it's only stirred while generating a procedural map.

---

## 4. Seed Pipeline — `Init_Random_Number_System` at `0x0052FC20`

Called from `Main_Game` at `0x0052E619` (after the scenario is loaded but before `ScenarioClass__Start_Scenario`). One pass per game start. (corrected 2026-05-29: was `0x0052E614` — that address holds the preceding `CALL 0x00550720`; the `CALL Init_Random_Number_System` is at `0x0052E619` — verified via `get_xrefs_to 0x0052FC20` (xref "From 0052e619 in Main_Game") + `read_memory 0x0052E614` bytes `e8 07 11 02 00`=CALL→0x00550720, then `e8 02 16 00 00`=CALL→0x0052FC20.)

### 4.1 Gate

```
if (DAT_00A8B8B8 == 0          // session/connection state flag, default 0 in SP
    && (DAT_00A8D5F8 & 2) == 0 // bit 1 = "replay playback in progress"
    && DAT_00AA0444 == 0)      // FUN_0053E720 — returns this, likely "demo header loaded"
{
    // SP / skirmish path: generate fresh seed (only when g_GameMode == 0 or g_GameMode == 5; corrected 2026-05-28: was just "SP/skirmish" — decompile_function 0x0052fc20 confirms `(g_GameMode == 0) || (g_GameMode == 5)` gate — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
    GetSystemTime(&st);
    entropy_mix_word(pool, st.wMilliseconds | (st.wSecond << 16));
    entropy_mix_bit(pool, st.wSecond);
    entropy_mix_bit(pool, st.wSecond >> 1);
    entropy_mix_bit(pool, st.wSecond >> 2);
    entropy_mix_bit(pool, st.wSecond >> 3);
    entropy_mix_bit(pool, st.wSecond >> 4);
    entropy_mix_bit(pool, st.wMinute);
    entropy_mix_bit(pool, st.wMinute >> 1);
    entropy_mix_bit(pool, st.wMinute >> 2);
    entropy_mix_bit(pool, st.wMinute >> 3);
    entropy_mix_bit(pool, st.wMinute >> 4);
    entropy_mix_bit(pool, st.wHour);
    entropy_mix_bit(pool, st.wDay);
    entropy_mix_bit(pool, st.wDayOfWeek);
    entropy_mix_bit(pool, st.wMonth);
    entropy_mix_bit(pool, st.wYear);
    // corrected 2026-05-29: the guard is the independent global DAT_00A8ED98,
    // NOT a member of the entropy-pool struct ("pool.extract_count"). The decompile
    // reads `DVar2 = DAT_00a8ed98; if (DAT_00a8ed98 == 0) {...}` — verified via
    // decompile_function 0x0052FC20.
    if (DAT_00A8ED98 == 0) {
        entropy_extract(&DAT_00A8ED94, 4);          // FUN_00661C10(&DAT_00A8ED94, 4)
        DAT_00A8ED94 = GetTickCount();              // overwrite with ticks
    }
}
// else: DAT_00A8ED94 was set elsewhere — by recording-replay-load (Main_Game) or by MP network handshake
```

The function then logs `"Seed is %08x"` (string at `0x008265F0`) to the heap pool log — this matches the well-known `redalert2.log` line for seed reporting.

### 4.2 Seeding the Three Instances

```
// 1. Seed Scen->Random
RandomClass_Seed(stack_buf, DAT_00A8ED94);
memcpy(g_ScenarioClass_Instance->_field_218, stack_buf, 0x3F4);  // 253 dwords

// 2. Seed g_MainRng (regenerate from same seed -> same state)
RandomClass_Seed(stack_buf, DAT_00A8ED94);
memcpy(&DAT_00886B88, stack_buf, 0x3F4);
```

So `Scen->Random` and `g_MainRng` both start in **identical state**. They diverge from this point on solely based on the order in which the game consumes them.

`g_MapGenRng` is NOT seeded here — it's seeded separately by the random-map generator from `MapSeedClass`.

### 4.3 Replay Recording Sourcing

When the game starts in replay-playback mode (`DAT_00A8D5F8 & 2`), `Main_Game` reads the seed from the recording file at `0x0052DCB6`:
```
read_4_bytes(&DAT_00822CF4);                          // magic / version
read_4_bytes(&DAT_00A8ED94);                          // <-- THE SEED
read_4_bytes(&g_ScenarioClass_Instance->_field_1254); // scenario settings
read_104h_bytes(&g_ScenarioClass_Instance->_field_125C); // scenario filename
read_4_bytes(&DAT_00A8EC90);                          // 
read_4_bytes(&DAT_00A8E960);                          // tiberium-growth/spread flags
read_B8h_bytes(&DAT_00A8EB60);                        // session config
```

When recording (bit 0 set), the same fields are written. The seed is the **second 4-byte field** in the recording header.

### 4.4 Entropy Mixer (SP path only)

The pool is a SHA-1-based state at offset `+0x14` of the pool struct. Each byte/bit injection writes via `pool[0x14 + (bit_counter / 8)] ^= 1 << (bit_counter & 7)`. When the bit counter hits `0x800` (2048 bits = 256 bytes), `FUN_0069D960` + `FUN_0069D9E0` runs a SHA-1 absorb+digest with init constants `{0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0}`. The digest is then mixed back into the pool 0x14 bytes at a time (256 times — a strong stretching step).

Extracted bytes are read via `FUN_00661C10` with a counter-mod-32 index.

**This is not used in multiplayer** — there, the seed is set from the network handshake before this code runs, and the gate at the top of `Init_Random_Number_System` skips the SP entropy mixing entirely.

---

## 5. Multiplayer Sync Mechanism

### 5.1 Seed Sync

In multiplayer modes (`g_GameMode == 3 LAN`, `g_GameMode == 4 Internet`), `DAT_00A8ED94` is set during network handshake **before** `Init_Random_Number_System` runs. The session-state flag `DAT_00A8B8B8` is set non-zero during MP setup, which causes the gate at top of `Init_Random_Number_System` to skip entropy gathering and use `DAT_00A8ED94` as-is.

> **RESOLVED 2026-05-28 (swarm slot-2, see `RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE_GHIDRA_REPORT.md`).** Three verified writers of `DAT_00A8ED94`:
> - **LAN guest:** `FUN_005b67f0 @ 0x005b67f0` ("Decoding game options") copies packet struct field `+0x92` directly into `DAT_00A8ED94` (no scramble), invoked from LAN packet handler `FUN_005b6020` `case 0x65`. (verified via `decompile_function 0x005b67f0`, `0x005b6020`; parent-confirmed write site `0x005b6ab6` via `get_xrefs_to 0x00A8ED94`)
> - **Internet/WOL:** `FUN_005e3d10 @ 0x005e3d10` parses the WOL options string with `CRT__strtok`/`CRT__atoi`; the **first decimal token** becomes `DAT_00A8ED94`. (verified via `decompile_function 0x005e3d10` at `0x005e4dc1`)
> - **LAN host:** `FUN_005b82f0 @ 0x005b82f0` generates the seed: `Random__RandomRanged(1, 0x7FFF)` on `g_MainRng` → `_srand` → `DAT_00A8ED94 = _rand()`, then broadcasts it in the game-options packet. (verified via `decompile_function 0x005b82f0`)
>
> The seed travels verbatim host→guests; only the resulting `u32` need match across clients (the host's generation mechanism is not lockstep-observable).

All clients receive the same seed, so all three RandomClass instances start in identical state on every client.

### 5.2 Per-Frame Sync Hash (Recording / Replay)

`Main_Tick` includes a state-hash check when `DAT_00A8D5F8 & 1` (recording) or `& 2` (playing back):

> **Scope clarification (swarm slot-1/slot-4, 2026-05-28).** This block is the **single-player recording/replay** path only (`DAT_00A8D5F8` flags are not set in LAN/WOL skirmish). `g_CurrentObjects` is the **selection list**, not the LogicClass active-object vector — so the "sum" is a checksum of *what the player has selected*, not of simulation state. The summed values are packed via `FUN_006E6AB0`; the two stream helpers `FUN_00473ae0`/`FUN_00473b10` are `WriteFile`/`ReadFile` wrappers, not hash functions. (verified via `decompile_function 0x0055D360`, `0x006E6AB0`, `0x00473ae0`, `0x00432050`; see `SYNC_CHECKSUM_MAINTICK_OBJECT_SUM_GHIDRA_REPORT.md`.) This is **not** the live multiplayer desync mechanism — see §5.3 and `DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md`.

**Recording (write path):**
```
read state_hash from MapClass (Scen + 0xD64 / 0xD68);   // 8 bytes
write 8 bytes to recording stream;
write g_CurrentObjects_Count (4 bytes);
sum = SUM over selected objects of: 0xFFFFFFFF if null,
      else ((TypeKind_byte << 24) | (object_heap_pool_id & 0xFFFFFF));
write sum (4 bytes);                                    // arithmetic sum, NOT a hash
for each selected object: write its packed (TypeKind_byte<<24 | heap_pool_id&0xFFFFFF);
write DAT_00ABCDFC and DAT_00ABCE00 (4 + 4 bytes);  // mouse / cursor state
```

**Playback (read path):**
```
read 8 bytes -> validate via FUN_006D6000 (writes to Scen + 0xD64, 0xD74);
read selected_count;
recompute local current sum;
read expected sum;
if (expected_sum != recomputed_sum) {
    Desync_Handler();    // wipe selection
}
... re-select objects from stream ...
read DAT_00ABCDFC + DAT_00ABCE00;
```

**`Desync_Handler` at `0x0048DC90` is mislabeled** — it does NOT handle network desync. It deselects all currently-selected objects:
```
while (g_CurrentObjects_Count != 0) (*vtable_unselect)(g_CurrentObjects_Data[0]);
Selection__ResetMode();
```

The Ghidra label is from an old guess and should be renamed to `Deselect_All` or `Selection__ClearAll`.

### 5.3 Per-Frame CRC32 Hash

Per-object state is hashed via the `CRCEngine` (`CRCEngine__AddData` at `0x004A1DE0`) using a CRC-32 with byte lookup table at `0x0081F7B4`. Each game class with state has a `*ComputeCRC` vtable method:
- `AbstractClass__ComputeCRC` at `0x00410410` — hashes ID (+0x10) and Owner (+0x20)
- `BombClass__ComputeCRC` at `0x00438A90`
- `DiskLaserClass__ComputeCRC` at `0x004A7B80`
- `SpawnManagerClass__ComputeCRC` at `0x006B7DE0`
- `TiberiumClass__ComputeCRC` at `0x00721DC0`

> **REFUTED 2026-05-28 (swarm slots 1+4, parent-verified live).** The hypothesis that these per-object CRC hashes are "summed into a per-frame state hash" used for live MP desync detection is **wrong**. There is **no live per-frame CRC compose/compare**:
> - The `*ComputeCRC` chain is **dead** in normal play: `FootClass__ComputeChecksum @ 0x004DBAD0` has exactly one caller, `FootClass__Save_Convoy_State @ 0x00744640`, which itself has **zero callers** (TS/campaign-era convoy save). (parent-verified via `get_function_callers 0x004DBAD0` and `get_function_callers 0x00744640`)
> - `Network_ServiceLoop @ 0x0048D080` was decompiled and contains **no hash compose or compare** — for modes 3/4 it delegates to `FUN_0048D1E0`, a command-queue lockstep drain. (verified via `decompile_function 0x0048D080`, `0x0048D1E0`)
>
> Live MP correctness is enforced by **command-gate lockstep** (all clients commit commands before `g_CurrentFrameCounter++`, gated by the 4 stop flags), **not** by hash-compare-then-abort. The `*ComputeCRC` vtable methods physically exist but are not wired into a live per-frame loop. See `DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md`.

### 5.4 The Mystery RNG-Spend in `Main_Tick`

At `Main_Tick` near `0x0055D...`, there is a conditional call to `Random__RandomRanged(0, 2)` gated on:
- `g_GameMode == 3 || g_GameMode == 4` (LAN/Internet)
- Some tag pointer (`DAT_00A8B23C`) non-null and its `vtable + 4` returns false
- The cell at mouse-cursor X has specific type/flags

This appears to be a **deliberate RNG-eating call** to keep two clients' streams aligned even when one client has a different cursor state. The exact mechanism wasn't fully decoded; further verification needed if implementing MP. (Deferred — §7.)

---

## 6. Current Rust Implementation Status

### What we have

> **UPDATED 2026-05-28.** The "xorshift64\*" description below is **STALE** — `src/sim/rng.rs` has already been migrated to the gamemd algorithm. Current state of `src/sim/rng.rs` (read directly):
> - **`SimRng`** is the **R(250,103) XOR lagged-Fibonacci** generator: `state: Vec<u32>` of 250 words, `index_a`/`index_b` (b seeded `0x67`), `disabled` flag, 4-round seed mixer (`INIT_TABLE_1`/`INIT_TABLE_2`, 4×`u32` each).
> - `next_range_u32_inclusive` matches gamemd `RandomRanged`: inclusive sorted bounds, no draw consumed when `low == high`, rejection sampling on a power-of-two mask (no modulo bias).
> - A passing **exact-output parity test** exists (`test_gamemd_raw_sequence_seed_one`: seed=1 → `0x78B7_6ED5`, `0x275D_74AE`, `0xDA63_B931`).
> - Still a **single** RNG stream on `Simulation` (no `scenario_rng` yet); `next_u32` modulo-1e6 raw-draw path is used by `terrain_spawn.rs` (TIBTRE) — already raw, not `RandomRanged`.
>
> *(Original 2025 scan, now historical:)*

- **`SimRng`** at [src/sim/rng.rs](src/sim/rng.rs) — xorshift64* (Marsaglia/Vigna), single 64-bit state. **Different algorithm from gamemd.**
- Single RNG stream on `Simulation` struct ([src/sim/world/mod.rs#L235](src/sim/world/mod.rs#L235)).
- Default seed `0x5EED_CAFE_D15E_A5E5` ([src/sim/world/mod.rs#L71](src/sim/world/mod.rs#L71)).
- Callsites: scatter, smudge dispatch, bridge state, fire/smoke/gas particles, ore growth, terrain spawners, particle spawn.
- Tests use fixed seeds for determinism — good.

### What's missing for parity

1. ~~**Algorithm mismatch.**~~ **RESOLVED 2026-05-28** — `src/sim/rng.rs` now implements R(250,103) XOR-LFG with a passing exact-output test for seed=1. No longer a mismatch. (Caveat: full seed-derivation-table parity across *all* seeds is UNVERIFIED — the doc's claim of 16-dword tables at `0x00839644`/`0x00839694` vs the Rust 4-entry `INIT_TABLE` mixer has not been reconciled; spot-checked only at seed=1.)
2. **No dual-stream design.** *(STILL OPEN — highest-priority gap.)* Rust has one `SimRng`; gamemd has `g_MainRng` + `Scen->Random` (plus map-gen). Per swarm slot-2/slot-3, `Scen->Random` is drawn by infantry/unit scatter direction, sub-cell rotation, TIBTRE probability+direction, survivor smudge, anim scorch/crater, and one HouseClass cell-state roll; everything else uses `g_MainRng`. The two-stream split is required for save/restore + lockstep parity.
3. **Seed-derivation tables: parity UNVERIFIED.** The two tables at `0x00839644` / `0x00839694` each contribute **4 dwords (16 bytes)** to the Feistel mixer (corrected 2026-05-28: was "16-dword each" — now confirmed 4 dwords each used). The 4-entry `INIT_TABLE_1`/`INIT_TABLE_2` in `src/sim/rng.rs` align with this loop structure. The seed=1 exact-output test passes, but full all-seeds parity is not proven.
4. **No entropy pool.** SP seed currently comes from a hardcoded constant; gamemd derives it from SHA-1 mixing of `GetSystemTime` + `GetTickCount`. Acceptable for the engine reimplementation (we are not bound to the same entropy source — we just need the same `u32` seed eventually feeding the same algorithm), but if a user expects the same start-of-game outcome as gamemd, we'd need to reproduce the seed too.
5. **`RandomRanged(low, high)` semantics: inclusive on BOTH ends, no-op on `low == high`, no-op on disabled flag.** Our `next_range_u32_inclusive` matches the inclusive semantics; need to verify no-op-on-equal behavior and rejection-sampling distribution.

### Parity priority

Per the project's parity bar: **algorithm + seed-derivation tables + dual-stream + RandomRanged semantics must all match exactly** to reproduce gamemd's visible behavior. Same input (seed + call order) must produce identical output (every damage roll, every scatter direction, every animation jitter).

---

## 7. Open Questions — Final State

- `[RESOLVED] Q1` — What algorithm? → **R(250, 103) XOR Lagged Fibonacci**. (evidence: `0x0065C780` Random__Next + `0x0065C6D0` seed function)
- `[RESOLVED] Q2` — RandomClass layout? → **0x3F4 bytes: flag@0, idx_a@4, idx_b@8, state[250]@0xC**. (evidence: `0x0065C780` field accesses + 253-dword copy at `0x0052FE3F`)
- `[RESOLVED] Q3` — How many RNG instances? → **Three**: `g_MainRng` (0x886B88), `Scen->Random` (Scen+0x218), `g_MapGenRng` (0xABE890). (evidence: callsite assembly at HouseClass__Update, InfantryClass__Scatter, FUN_00598960)
- `[RESOLVED] Q4` — Where is the seed planted? → **`FUN_0052FC20` (Init_Random_Number_System)**, called from `Main_Game @ 0x0052E619`. (corrected 2026-05-29: was `0x0052E614` — that is the preceding `CALL 0x00550720`; the `CALL Init_Random_Number_System` is at `0x0052E619` — verified via `get_xrefs_to 0x0052FC20` + `read_memory 0x0052E614`.)
- `[RESOLVED] Q5` — Inclusive or exclusive bounds? → **Inclusive on both ends**. (evidence: `range = high - low` and final `result + low`)
- `[RESOLVED] Q6` — Behavior when `low == high`? → **Returns `low`, does NOT consume state**. (evidence: early `if (param_2 != param_3)` short-circuit)
- `[RESOLVED] Q7` — Behavior when `low > high`? → **Swap, then proceed normally**. (evidence: `if (param_3 < param_2) swap`)
- `[RESOLVED] Q8` — Disabled flag behavior? → **`flag != 0` → returns `low`, state NOT advanced**. (evidence: inner-loop check `if (*param_1 == '\0')`)
- `[RESOLVED] Q9` — `Sync_Random` function in YR? → **No such function exists.** `Desync_Handler @ 0x0048DC90` is mislabeled — it's selection-clear, not RNG-sync. (evidence: full decompile)
- `[RESOLVED] Q10` — Replay seed mechanism? → **Seed is the 2nd 4-byte field in the recording stream**, read at `Main_Game 0x0052DCB6`. (evidence: decompile of Main_Game replay-load block)
- `[RESOLVED] Q11` — `g_FrameCounter`-derived pseudo-random? → **Not RNG**; it's just `g_CurrentFrameCounter` at `0x00A8ED84`, used for various periodic effects but not a random number stream.
- `[RESOLVED] Q12` — Network-seed scramble? → **None found.** Seed travels verbatim in MP handshake.
- `[RESOLVED] Q13` — Entropy source in SP? → **SHA-1 pool stirred with `GetSystemTime` fields and `GetTickCount`**. (evidence: FUN_0052FC20 + FUN_00661770 + FUN_0069D9E0 finalize)
- `[RESOLVED] Q14` — Uniform distribution? → **Yes**, via rejection sampling on `range`-bit mask. No modulo bias.
- `[RESOLVED] Q15` — `g_ScenarioClass_Instance` at 0x00A8B230 — is it a pointer or the instance? → **Pointer**; instance is heap-allocated; `Scen->Random` address is dynamic. (evidence: `MOV EAX, [0x00A8B230]; LEA ECX, [EAX + 0x218]` in HouseClass__Update)
- `[RESOLVED] Q16` — Seed-derivation tables? → **`0x00839644` and `0x00839694`, 16 × `u32` each**. First 16 bytes of table 1 dumped in §2.5 above. Must be reproduced verbatim for parity.
- `[RESOLVED] Q17` — Period of the LFG? → **~2^250** (effectively infinite for game purposes).
- `[RESOLVED] Q18` — Is `rand()` (CRT) used anywhere in sim? → **No** in studied callers. All studied callers use RandomClass.

- `[RESOLVED 2026-05-28] D1` — Exact network handshake function that writes `DAT_00A8ED94` in MP. → **Three writers found** (LAN guest `FUN_005b67f0` packet field `+0x92`; Internet `FUN_005e3d10` WOL options-string first token; LAN host `FUN_005b82f0` `_rand()`). See §5.1 and `RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE_GHIDRA_REPORT.md`. (verified via `decompile_function` at each + parent `get_xrefs_to 0x00A8ED94`)
- `[DEFERRED] D2` — How `DAT_00A8B8B8` gets set in MP (signals "seed already set"). (category: `out-of-scope`; reason: paired with D1; next-step-if-pursued: trace MP session-state machine.)
- `[RESOLVED/REFUTED 2026-05-28] D3` — Where the per-frame CRC32 game-state hash is composed and what fields it covers. → **No live per-frame CRC compose exists.** The `*ComputeCRC` chain is dead (`FootClass__ComputeChecksum`'s only caller `Save_Convoy_State` has zero callers), and `Network_ServiceLoop` does no hash compose/compare — live MP uses command-gate lockstep. See §5.3 and `DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md`. (parent-verified via `get_function_callers 0x004DBAD0`/`0x00744640`; `decompile_function 0x0048D080`/`0x0048D1E0`)
- `[DEFERRED] D4` — The exact reason for the conditional `Random__RandomRanged(0,2)` call in `Main_Tick` under MP modes (the "mystery RNG-spend"). (category: `bounded-cost-too-high`; reason: requires cross-correlation with mouse/network state across clients; next-step-if-pursued: instrument both clients and compare RNG-state snapshots before and after this call.)
- `[DEFERRED] D5` — Save/load behavior of `g_MainRng` (is it serialized?). (category: `out-of-scope`; reason: requires investigating save-system layout; next-step-if-pursued: decompile `ScenarioClass::Save` / `ScenarioClass::Load`.)

**Coverage statement:** This report covers the RNG **algorithm, layout, three instances, seed pipeline, init gates, recording/replay seed sourcing, and call-site semantics** at HIGH confidence. It does NOT cover the MP network handshake source of `DAT_00A8ED94`, the full per-frame state-hash composition, or the savegame serialization of RNG state — those are referenced as deferred items that need their own investigations.

---

## Sources

**Ghidra addresses decompiled:**
- `0x0065C7E0` Random__RandomRanged
- `0x0065C780` Random__Next
- `0x0065C6D0` RandomClass_Seed
- `0x0052FC20` Init_Random_Number_System
- `0x006832C0` ScenarioClass__Constructor
- `0x00683560` (related Scen reset)
- `0x00598960` Random-map orchestrator (writes `g_MapGenRng`)
- `0x00595680` MapSeedClass__Constructor
- `0x00661770` entropy-mixer (bit inject)
- `0x00661850` entropy-mixer (byte inject)
- `0x00661C10` entropy extract
- `0x0069D9E0` SHA-1 finalize (in pool)
- `0x0065C8B0` (pool counter step)
- `0x0048DC90` `Desync_Handler` (mislabeled — selection clear)
- `0x004A1DE0` CRCEngine__AddData (CRC-32)
- `0x00410410` AbstractClass__ComputeCRC
- `0x00652230` Event-queue advance (writes `DAT_00A8B8B8`)
- `0x0055D360` Main_Tick (recording/replay sync block)
- `0x0052D9A0` Main_Game (recording header read)
- `0x004F8440` HouseClass__Update (sampled for RNG `this` arg)
- `0x0051D0D0` InfantryClass__Scatter (sampled for RNG `this` arg)
- `0x0053E720` (returns `DAT_00AA0444`)

**Static data inspected:**
- `0x00839644` (16 u32 mixing table 1 — first 16 bytes dumped)
- `0x00839694` (16 u32 mixing table 2 — referenced; not yet dumped, recommended next step)
- `0x0081F7B4` (CRC-32 lookup table — referenced)
- `0x00ABE890` (g_MapGenRng instance — verified zero at static-init time)

**Doc references:**
- `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` — `0x00ABE890 g_GlobalRng` claim (corrected here: that address is `g_MapGenRng`, not the main RNG)
- `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` — confirmed `RandomRanged(1,4)` and `RandomRanged(0,2) - 1` patterns
- `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` — confirmed `RandomRanged(1, BridgeStrength)` inclusive semantics
- `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` — confirmed `RandomRanged(0x100, 0x300)` lepton offsets
- `CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md` — confirmed lockstep-relevant RNG sequence requirement
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` — claim about "g_FrameCounter-derived pseudo-random" — clarified here: it's a frame counter, not RNG.

**INI files checked:**
- `ini/rulesmd.ini` and `ini/artmd.ini` — confirmed broad RNG-consuming key set (RandomRate, ScatterChance, AnimationProbability, etc.) but no key directly controls the RandomClass algorithm or seed.

**Rust files referenced:**
- [src/sim/rng.rs](src/sim/rng.rs)
- [src/sim/world/mod.rs](src/sim/world/mod.rs)
- [Cargo.toml](Cargo.toml)
