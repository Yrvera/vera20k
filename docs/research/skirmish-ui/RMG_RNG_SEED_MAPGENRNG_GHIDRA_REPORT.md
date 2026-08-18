# RMG RNG Seeding / Reproducibility Contract — Ghidra Research Report

**Address(es):** `Random__Seed @ 0x0065C6D0`, `Random__Next @ 0x0065C780`, generator entry `FUN_00598960 @ 0x00598960`  
**Investigation Mode:** RNG primitive + seeding contract  
**Claimed Scope:** Exact seed source field (`MapSeed+0x74`), `Random__Seed` algorithm and state layout, `Random__Next` advance and return-range contract, confirmation that all generator-phase draws use `g_MapGenRng` not the main game RNG.  
**Non-Scope (other swarm slots own):** Terrain/noise/region/water/tiberium/hill/start-point algorithms; `RandMap.Sed` file format; preview decode; UI controls.  
**Active in YR:** Conditional — only active when Skirmish launch selects a `.SED` random-map entry; the RNG primitive itself exists in all YR builds.  
**Confidence:** HIGH for all material findings (all verified from assembly/decompile this session with inline citations).

---

## Investigation Scaffolding

**Target question:** What is the exact "same seed → same RNG stream" contract? Specifically: (a) the seed source field and type at `MapSeed+0x74`; (b) what `Random__Seed 0x0065C6D0` does algorithmically (state size, algorithm family); (c) `Random__Next 0x0065C780` advance mechanics and raw return range; (d) which global instance the generator phases draw from.

**Non-goals:** Do not decode terrain noise or placement algorithms; do not trace `Scen->Random` or `g_MainRng` usage in gameplay sim; do not investigate `Init_Random_Number_System` beyond confirming the three-instance split.

**Evidence needed to mark COMPLETE:** (1) `MapSeed+0x74` type confirmed from assembly; (2) `Random__Seed` state size and loop bound from assembly immediates; (3) `Random__Next` wrap threshold from assembly; (4) at least three generator-phase call sites confirmed to load `ECX = 0x00ABE890` before calling `0x0065C780`.

**Stop conditions:** Stop when four evidence lines above are satisfied and the struct layout is algebraically derivable.

---

## 1. Seed Source Field: `MapSeed+0x74`

`FUN_00598960` reads the seed with:

```
0059897B: MOV EAX, dword ptr [EBP + 0x74]   ; EBP = MapSeed* (param_1 stored there)
00598980: PUSH EAX
00598981: LEA ECX, [ESP + 0x38]              ; stack-local temp RNG object
00598985: CALL 0x0065C6D0                    ; Random__Seed(temp, seed)
```

`verified via get_assembly_context xref_sources=0x00598985`

- `MapSeed+0x74` is a **32-bit unsigned integer** (`dword ptr` load), pushed as a DWORD to `Random__Seed`.
- The field is clamped to `0..0xFFFF` by the options normalizer at `0x005975E0` before generation. (Evidence: prior SKIRMISH_RANDOM_MAP_GENERATOR report §3; normalizer decompile `0x005975E0`.)
- Effective seed range entering `Random__Seed`: **0 to 65535 (0xFFFF)**.
- **Active in YR: Conditional** (seed non-zero only when generating a random map from a `.SED` file).

---

## 2. `Random__Seed 0x0065C6D0` — Algorithm, State Size, and Init Contract

### 2.1 Calling convention and `this` pointer

`Random__Seed` is `__thiscall`: the struct pointer arrives in `ECX`.  
At the call site (`0x00598985`), `ECX = [ESP+0x38]` — a 1024-byte stack-local temp object allocated inside `FUN_00598960`'s frame (`SUB ESP, 0x418` = 1048-byte frame).  
`verified via get_assembly_context xref_sources=0x00598985`

### 2.2 Header field initialization

```
0065C6DA: MOV dword ptr [ECX + 0x4], EAX    ; r index = 0 (EAX=0 from XOR EAX,EAX)
0065C6DD: MOV dword ptr [ECX + 0x8], 0x67   ; s index = 103 (0x67)
0065C6E5: ADD ECX, 0xC                       ; advance past header to state[0]
```

`verified via get_assembly_context xref_sources=0x0065C6D0`

- Byte at `+0x0`: the "locked/disabled" guard byte. `Random__Seed` **unconditionally clears it to 0 at its tail**: `0x0065C769 MOV EAX,[ESP+0x18]` reloads the saved `this`, then `0x0065C770 MOV byte ptr [EAX],0x0`. `Random__Next` checks `*this == 0` before proceeding, so a freshly seeded struct is always unlocked. `verified via disassemble_function 0x0065C6D0 2026-07-20`
- `+0x4` (4 bytes): r index, set to **0**.
- `+0x8` (4 bytes): s index, set to **103 (0x67)**.
- `+0xC` onwards: 250-dword state array.

### 2.3 State filling loop

```
0065C6EE: MOV dword ptr [ESP + 0x14], 0xFA  ; outer loop counter = 250 (0xFA)
0065C6F6: JMP 0x0065C6FC
; inner loop: ONE 4-round Feistel-like pass per output dword (EBX = 0,4,8,0xC):
0065C709:   MOV EDX, [EBX + 0x839644]       ; table-1 constant (EBX = 0..0xC → 0x839644..0x839650)
0065C713:   ADD EBX, 0x4                    ; EBX incremented BEFORE the table-2 load
0065C73C:   MOV EDX, [EBX + 0x839690]       ; table-2 constant (EBX = 4..0x10 → 0x839694..0x8396A0)
0065C748:   CMP EBX, 0x10 / JL 0x0065C709   ; 4 rounds
0065C757: ADD EAX, 0x4           ; advance write pointer
0065C762: DEC EAX (counter)      ; outer counter decrement
0065C767: JNZ 0x0065C6F8         ; 250 iterations total
```

`verified via disassemble_function 0x0065C6D0 2026-07-20`

- **250 dwords** are written into `this+0xC` through `this+0xC+249*4`.
- The seeding transform is NOT a standard LCG or Mersenne Twister. It is a custom hash: ONE 4-round Feistel-like pass per output dword, mixing an iteration counter and the input seed against **four constants from each of two tables** (not 16-element tables; only 4 dwords of each are consumed per pass, the same 4 every pass).
- **Table 1** — instruction displacement `0x00839644`, effective fetches at `+0..+0xC`:
  `0x00839644 = 0xBAA96887`, `0x00839648 = 0x1E17D32C`, `0x0083964C = 0x03BCDC3C`, `0x00839650 = 0x0F33D1B2`. `verified via read_memory 0x00839644 (16 bytes) 2026-07-20`
- **Table 2** — instruction displacement `0x00839690` (at `0x0065C73C`), but `ADD EBX,0x4` at `0x0065C713` executes before the load, so the effective fetches are at `+4..+0x10`:
  `0x00839694 = 0x4B0F3B58`, `0x00839698 = 0xE874F0C3`, `0x0083969C = 0x6955C5A6`, `0x008396A0 = 0x55A7CA46`.
  The dword at the displacement base itself, `0x00839690 = 0x48AAD7E4`, is **never consumed** by this loop.
  Full 16-dword dump of the `0x00839690` region for reference (`verified via read_memory 0x00839690 (64 bytes) 2026-07-20`):
  `48AAD7E4 4B0F3B58 E874F0C3 6955C5A6 55A7CA46 4D9A9D86 FE28A195 B1CA7865 6B235751 9A997A61 AA6E95C8 AAA98EE1 5AF9154C FC8E2263 390F5E8C 58FFD802`

### 2.4 Return value

`Random__Seed` returns `param_1` (the `ECX` pointer to the same struct) in EAX — i.e., the seeded struct itself.  
At call site in `FUN_00598960`: `MOV ESI, EAX` immediately after the call. `verified via get_assembly_context xref_sources=0x00598985`

### 2.5 Total struct size and copy count

After seeding, `FUN_00598960` copies 0xFD = 253 dwords from ESI (= seeded struct) to `g_MapGenRng`:

```
0059898A: MOV ECX, 0xFD          ; 253 dwords = 1012 bytes
0059898F: MOV ESI, EAX           ; source = seeded temp struct
00598996: MOV EDI, 0xABE890      ; destination = g_MapGenRng
0059899B: MOVSD.REP ES:EDI, ESI  ; 253 × 4-byte copy
```

`verified via get_assembly_context xref_sources=0x00598985`

- Struct layout: `[0x0]=locked_byte+3_pad`, `[0x4]=r_index`, `[0x8]=s_index`, `[0xC..0x3F3]=state[0..249]` (0xC + 250×4 = 0x3F4 exclusive; verified via disassemble_function 0x0065C6D0 2026-07-20 — 250-dword fill starting at `this+0xC`)
- 253 dwords = 3 header dwords (`+0x0`, `+0x4`, `+0x8`) + 250 state dwords = **full struct copy**.
- **Struct size: 253 × 4 = 1012 bytes = 0x3F4 bytes** (the algorithm uses exactly this amount).

---

## 3. `Random__Next 0x0065C780` — Advance Mechanics and Return Range

### 3.1 Guard check

```
0065C780: CMP byte ptr [ECX], 0x0   ; check locked flag at this+0x0
0065C783: JZ 0x0065C788             ; if zero (not locked), proceed
0065C785: XOR EAX, EAX              ; else return 0
0065C787: RET
```

`verified via get_assembly_context xref_sources=0x0065C780`

### 3.2 State advance (lagged-Fibonacci XOR)

```
0065C788: MOV EAX, dword ptr [ECX + 0x4]       ; r = this->r
0065C78B: MOV EDX, dword ptr [ECX + 0x8]       ; s = this->s
0065C78F: MOV EDX, dword ptr [ECX + EDX*4+0xC] ; load state[s]
0065C793: MOV ESI, dword ptr [ECX + EAX*4+0xC] ; load state[r]
0065C797: LEA EAX, [ECX + EAX*4+0xC]           ; &state[r]
0065C79B: XOR ESI, EDX                          ; state[r] ^= state[s]
0065C79D: MOV dword ptr [EAX], ESI             ; store back
0065C79F: MOV EDX, dword ptr [ECX + 0x4]       ; reload r
0065C7A2: MOV ESI, dword ptr [ECX + 0x8]       ; reload s
0065C7A5: MOV EAX, dword ptr [ECX + EDX*4+0xC] ; state[r] (post-XOR)  ← EAX = return value
0065C7A9: INC EDX                               ; r++
0065C7AA: INC ESI                               ; s++
0065C7AB: CMP EDX, 0xFA                         ; if r >= 250, wrap
0065C7B1: MOV dword ptr [ECX + 0x4], EDX
0065C7B4: MOV dword ptr [ECX + 0x8], ESI
0065C7B7: JL 0x0065C7C0
0065C7B9: MOV dword ptr [ECX + 0x4], 0x0       ; r wraps to 0
0065C7C0: CMP ESI, 0xFA                         ; if s >= 250, wrap
0065C7C7: JL 0x0065C7D0
0065C7C9: MOV dword ptr [ECX + 0x8], 0x0       ; s wraps to 0
0065C7D0: RET
```

`verified via get_assembly_context xref_sources=0x0065C780`

- **Algorithm: Lagged-Fibonacci Generator (LFG), variant XOR, lags R=250, S=103** (initial r=0, s=103; both wrap at 250).
- **Return value**: raw `uint32` — `state[r]` after the XOR update. No masking, no modulo. Full 32-bit unsigned range `[0, 2^32)`.
- **Range reduction**: Done entirely by callers via `FILD → FMUL [0x007ED898] → FMUL max_val → FISTS` or equivalent. The constant at `0x007ED898` is bit pattern `0x3DF0000000100000` = **2^-32 × (1 + 2^-32)** — it is **NOT bit-exact `1/2^32`** (exact `1/2^32` would be `0x3DF0000000000000`; the stored mantissa has bit 20 set (mantissa field 0x0000000100000)). A Rust `1.0/4294967296.0` literal will NOT reproduce it; use `f64::from_bits(0x3DF0000000100000)`. Callers use it to convert the raw uint32 to an (approximately) `[0.0, 1.0)` double, then scale by the desired range. `verified via read_memory 0x007ED898 (bytes 00 00 10 00 00 00 F0 3D) 2026-07-20`

---

## 4. RNG Instance Routing — Generator Phases Use `g_MapGenRng` Exclusively

### 4.1 Global addresses confirmed

| Global | Address | Evidence |
|---|---|---|
| `g_MapGenRng` | `0x00ABE890` | `MOV EDI, 0xABE890` at `0x00598996`; `MOV ECX, 0xABE890` before `Random__Next` at every sampled call site |
| `g_MainRng` | `0x00886B88` | `MOV EDI, 0x886B88` in `Init_Random_Number_System` at `0x0052FEA0` |
| `Scen->Random` | `ScenarioClass_ptr+0x218` | `LEA EDI, [ECX+0x218]` in `Init_Random_Number_System` at `0x0052FE26`; `ScenarioClass` pointer at `0x00A8B230` |

`verified via get_assembly_context xref_sources=0x0052fe17,0x0052fe3e,0x0052fe3d` and `get_assembly_context xref_sources=0x00598985`

### 4.2 Phase call sites — ECX load before `Random__Next 0x0065C780`

Every sampled generator-phase caller loads `ECX = 0x00ABE890` immediately before calling `0x0065C780`:

| Call site | Function | ECX load instruction |
|---|---|---|
| `0x0058CAFE` | `FUN_0058C800` (region flood-fill constructor — region-partition phase; flood-fills cells matching the `+0x11b` land-type byte into the `DAT_00ABED10` region array, then allocates the 0x50-byte region object; verified via decompile_function 0x0058C800 2026-07-20) | `MOV ECX, 0xABE890` at `0x0058CAF9` |
| `0x0058D787` | `FUN_0058D620` (region init) | `MOV ECX, 0xABE890` at `0x0058D782` |
| `0x0058D7D2` | `FUN_0058D620` | `MOV ECX, 0xABE890` at `0x0058D7CD` |
| `0x0059A4C7` | `CCINIClass__Constructor` (map init phase) | `MOV ECX, 0xABE890` at `0x0059A4B6` |
| `0x0059C6DF` | `FUN_0059C630` (water gen) | `MOV ECX, 0xABE890` at `0x0059C6DA` |

`verified via get_assembly_context xref_sources=0x0058cafe,0x0058d787,0x0058d7d2,0x0059a4c7,0x0059c6df`

None of the generator-phase callers of `Random__Next` load `ECX = 0x00886B88` (g_MainRng) or `ECX = Scen+0x218`.

### 4.3 `Init_Random_Number_System` — three instances seeded independently

`Init_Random_Number_System @ 0x0052FE00` calls `Random__Seed` four times (two paths × two instances) to initialize `Scen->Random (+0x218)` and `g_MainRng (0x886B88)`. It does **not** touch `g_MapGenRng (0x00ABE890)`. The map-gen RNG is seeded exclusively by `FUN_00598960` at generation time.  
`verified via decompile_function 0x0052fe00`

---

## 5. Implementation Handoff

### 5.1 Rust delta items

**Handoff 1:**  
Verified behavior: `g_MapGenRng` is a 1012-byte (253-dword) lagged-Fibonacci XOR generator (R=250, S=103) seeded from `MapSeed+0x74` (uint32, clamped 0..65535) via a custom Feistel-hash state fill of 250 dwords. The RNG struct has a 12-byte header (locked u8+3 pad, r u32, s u32) and a 250-element u32 state array.  
Rust delta: In the map-gen module, create a `struct MapGenRng { locked: bool, r: u32, s: u32, state: [u32; 250] }` and implement `seed(seed_value: u16)` (fills header fields and state via the 250-iteration Feistel transform matching the two constant tables at binary 0x00839644 / 0x00839694), and `next_u32() -> u32` (XOR-LFG advance with R=250, S=103, wrap at 250).  
Affected surface: `src/map_gen/rng.rs` (new file) or extension of existing `src/sim/rng.rs` (which already has the LFG for gameplay RNG).  
Acceptance scenario: With seed `0x1234`, call `next_u32()` N times; the output stream must reproduce identically regardless of call-order isolation — test by running two independent seed-and-draw sequences with the same seed.  
Proposed Rust test name: `test_mapgen_rng_same_seed_same_stream`  
Risk: LOW — the LFG advance is already implemented in `src/sim/rng.rs`; only the seeding transform (Feistel fill from the two constant tables) is new.

**Handoff 2:**  
Verified behavior: `Random__Next` returns a raw `uint32` in `[0, 2^32)`. Callers scale to a target range via `(raw_u32 as f64) * K * max_val` then floor, where `K` is the double at `0x007ED898` with bit pattern `0x3DF0000000100000` = 2^-32 × (1 + 2^-32) — NOT bit-exact `1/2^32` (verified via read_memory 0x007ED898 2026-07-20). There is no internal modulo.  
Rust delta: The generator phases should call `rng.next_u32()` and apply `(val as f64 * f64::from_bits(0x3DF0000000100000) * range as f64) as u32` for range reduction. Do NOT use a `1.0/4294967296.0` literal — it does not reproduce the binary's constant bit-for-bit. Do NOT use Rust `% range` directly on the raw value — that produces a different distribution.  
Affected surface: Every generator-phase function that draws random values.  
Acceptance scenario: For a range `[0, 100)`, the distribution of 10,000 draws should be approximately uniform; a known seed should reproduce the exact sequence of scaled values.  
Proposed Rust test name: `test_mapgen_rng_range_scaling_matches_gamemd`  
Risk: LOW — the `(f64 * range) as u32` pattern is the standard gamemd caller idiom observed across all sampled sites.

**Handoff 3:**  
Verified behavior: `g_MapGenRng` is a completely separate instance from `g_MainRng` and `Scen->Random`. `Init_Random_Number_System` never initializes `g_MapGenRng`; it is only seeded in `FUN_00598960` immediately before generation.  
Rust delta: The Rust map-gen entrypoint must own a local `MapGenRng` instance seeded from `MapSeed.seed` (the `+0x74` field). It must NOT use `sim::World::rng` or any gameplay RNG instance. The map-gen RNG is not part of the sim state hash.  
Affected surface: `src/map_gen/` entrypoint; any place that might naively pass the global sim RNG to terrain functions.  
Acceptance scenario: Run two skirmish games with the same map seed; both produce identical map layouts. Then run two games with different seeds; map layouts differ at the first RNG draw.  
Proposed Rust test name: `test_mapgen_separate_rng_instance_from_sim`  
Risk: MEDIUM — if any generator phase accidentally draws from the sim RNG, map generation will not be reproducible across multiplayer clients who share the seed but not the sim RNG state.

### 5.2 Negative Facts / Do Not Do

1. **Do not seed the map-gen RNG from `g_MainRng` or `Scen->Random`.** The three instances are initialized independently. `verified via decompile_function 0x0052fe00` (Init_Random_Number_System writes only Scen+0x218 and g_MainRng, never g_MapGenRng) and `get_assembly_context xref_sources=0x00598985` (generator always seeds g_MapGenRng from MapSeed+0x74 directly).

2. **Do not implement `Random__Next` as a simple LCG or Mersenne-Twister.** The algorithm is a lagged-Fibonacci XOR generator with state-array size 250 and lags R=0→250 (mod), S=103. `verified via get_assembly_context xref_sources=0x0065c780` (wrap threshold `CMP EDX, 0xFA` = 250 at `0x0065C7AB`).

3. **Do not apply an internal modulo in `next_u32()`.** The raw return is an unmasked uint32. All range reduction is caller-side floating-point. `verified via get_assembly_context xref_sources=0x0065c780` — no AND/modulo after `XOR ESI, EDX` before `RET`.

4. **Do not copy only 250 dwords when initializing `g_MapGenRng`.** The copy is 253 dwords (header + state), not 250. `verified via get_assembly_context xref_sources=0x00598985` (`MOV ECX, 0xFD` at `0x0059898A`).

5. **Do not treat `MapSeed+0x74` as a signed int.** It is loaded via `dword ptr` (unsigned) and clamped to `0..0xFFFF`. Using a signed type would produce negative seeds for values above 0x7FFF. `verified via get_assembly_context xref_sources=0x00598985` (`MOV EAX, dword ptr [EBP+0x74]` at `0x0059897B`).

### 5.3 Remaining Uncertainty

1. **[RESOLVED 2026-07-20] Feistel constants fully recorded.** The 8 consumed constants (4 per table) are dumped in §2.3: table 1 effective fetches `0x00839644..0x00839650`, table 2 effective fetches `0x00839694..0x008396A0` (instruction displacement `0x00839690` with EBX pre-incremented; the dword `0x48AAD7E4` at `0x00839690` itself is never consumed). `verified via disassemble_function 0x0065C6D0 + read_memory 0x00839644/0x00839690 2026-07-20`

2. **[RESOLVED 2026-07-20] `locked` byte initialization.** The earlier claim that `Random__Seed` does not write the `+0x0` locked byte was wrong: the function's tail unconditionally clears it (`0x0065C769 MOV EAX,[ESP+0x18]` reloads `this`; `0x0065C770 MOV byte ptr [EAX],0x0`). A freshly seeded struct is always unlocked regardless of prior stack garbage; the 253-dword copy into `g_MapGenRng` therefore always copies an unlocked header. `verified via disassemble_function 0x0065C6D0 2026-07-20`

---

## 6. Cross-Reference to Existing Docs

The prior `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md` §3 states:
> "The generator does not rely on the process global RNG for the generated terrain stream after the seed is known. At function entry it calls `FUN_0065C6D0(seed)` and copies `0xFD` dwords into global `DAT_00ABE890`."

This report **verifies and expands** that claim with:
- Full RNG algorithm identity (LFG XOR, R=250, S=103, state size 250 dwords)
- Struct layout (12-byte header + 1000-byte state = 1012 bytes total)
- `Random__Next` advance mechanics and return-range contract
- Five generator-phase call-site confirmations that all load `ECX = 0x00ABE890`

No stale or wrong claims found in the prior doc for the RNG seeding section; the expansion here adds precision, not corrections.

---

## 7. Summary of Verified Facts

1. `MapSeed+0x74` is a `uint32` read via `dword ptr`, clamped to `0..0xFFFF`, pushed to `Random__Seed`. `verified via get_assembly_context xref_sources=0x00598985`
2. `Random__Seed` loop counter is `0xFA` (250 iterations), filling state dwords at `this+0xC` through `this+0xC+249*4`. `verified via get_assembly_context xref_sources=0x0065c6d0`
3. `Random__Next` wraps r and s at threshold `0xFA` (250), returning raw XOR'd state as uint32 with no modulo. `verified via get_assembly_context xref_sources=0x0065c780`
4. `FUN_00598960` copies exactly 253 dwords (`MOV ECX, 0xFD`) from the seeded temp object to `g_MapGenRng @ 0x00ABE890`. `verified via get_assembly_context xref_sources=0x00598985`
5. All five sampled generator-phase `Random__Next` call sites load `ECX = 0x00ABE890` (g_MapGenRng), never g_MainRng (0x886B88) or Scen+0x218. `verified via get_assembly_context xref_sources=0x0058cafe,0x0058d787,0x0058d7d2,0x0059a4c7,0x0059c6df`
