# RNG MP Seed Handshake & Gameplay Instance — Ghidra Research Report

**Scope:** (a) The network handshake paths that write `DAT_00A8ED94` (the shared u32 RNG seed)
before `Init_Random_Number_System @ 0x0052FC20` runs in LAN/Internet modes.
(b) Confirmation of which RandomClass instance (`g_MainRng @ 0x00886B88` vs `Scen->Random`) the
live gameplay simulation draws from per frame.

**Prerequisite:** `RNG_SYSTEM_GHIDRA_REPORT.md` covers algorithm, layout, three instances,
seed pipeline, and single-player entropy source — do not re-derive those here.

**Confidence:** HIGH on all verified findings below.

**Active in YR:** Yes for all paths described (LAN=g_GameMode 3, Internet=g_GameMode 4).

---

## 1. Investigation Protocol

**Target question:**
- (a) Which function(s) write `DAT_00A8ED94` in MP mode, and what is the source of the seed
  value (host-generated random, network packet, INI, etc.)?
- (b) Do the per-frame gameplay paths (combat, scatter, ore, particles) draw from `g_MainRng`
  or from `Scen->Random`?

**Non-goals:**
- RNG algorithm, layout, RandomRanged helper — see RNG_SYSTEM_GHIDRA_REPORT.md.
- Sync checksum or desync-detection message paths (slot 1/4 scope).
- PerTickUpdate callee ordering (slot 5 scope).

**Evidence needed to mark COMPLETE:**
- ≥1 verified writer of `DAT_00A8ED94` that is called during MP session setup.
- Clear identification of the seed source in that writer (packet field, `rand()`, etc.).
- Confirmation that `g_MainRng` is the dominant gameplay draw instance
  (or explicit evidence that `Scen->Random` draws happen in sim-deterministic paths).

**Stop conditions:**
- Finding both the LAN path and Internet path writers, or confirming they share one path.
- Confirming g_MainRng vs Scen->Random split from HouseClass__Update and InfantryClass__Scatter.

---

## 2. Seed Handshake Writers — Complete Inventory

`get_xrefs_to 0x00A8ED94` returned 23 references. Filtering to WRITE sites outside
`FUN_0052FC20` (Init_Random_Number_System) and `Main_Game` replay-path:

| Address | Function | Write Source |
|---------|----------|--------------|
| `0x005b6ab6` | `FUN_005b67f0` | `*(param_1 + 0x92)` (packet field) |
| `0x005e4dc1` | `CDFileClass__Constructor @ 0x005e3d10` (mislabeled) | `CRT__atoi(first_strtok_token)` |
| `0x005b8b0a` | `CDFileClass__Constructor @ 0x005b82f0` (mislabeled) | `_rand()` |
| `0x005dec5c` | *(none — out-of-function code)* | `MOV [DAT_00A8ED94], EAX` (lobby/netshare cluster) |
| `0x005c4896` | *(none — out-of-function code)* | `MOV [DAT_00A8ED94], EDX` (lobby/netshare cluster) |

**Correction (2026-05-29):** The original table listed only the first three WRITE sites and
presented the inventory as complete; it omitted the two WRITE xrefs at `0x005dec5c` and
`0x005c4896`. Both are real, disassemblable code that does NOT fall inside any Ghidra-bounded
function (`get_function_by_address` returns "No function found" for each), which is why they
were missed by a function-scoped filter. Re-confirmed via `get_xrefs_to 0x00A8ED94` (both
flagged `[WRITE]`), `disassemble_bytes` (each disassembles cleanly), and `read_memory`:
`0x005dec5c` = `a3 94eda800` (`MOV moffs32[0x00A8ED94], EAX`), `0x005c4896` =
`89 15 94eda800` (`MOV [0x00A8ED94], EDX`). They sit in the lobby/netshare cluster
(neighbouring READs at `0x005c39da`, `0x005c48a1`).

The first three are mislabeled as `CDFileClass__Constructor` in Ghidra; they are lobby/session
dialog handlers. Verified via `decompile_function` at each entry address.

---

## 3. Path A — LAN "Received Game Options" Packet (packet type 0x65)

**Call chain** (verified via `decompile_function 0x005b6020`, `get_xrefs_to 0x005b6020`,
`get_xrefs_to 0x005b49b0`):

```
Main_Game @ 0x0052E1D5
  └─ FUN_005b49b0   [ModemGuest_Dialog]
       └─ FUN_005b6020   [packet handler, switch on packet_type]
            case 0x65 ("Received game options"):
              └─ FUN_005b67f0   [DecodeGameOptions_LAN]
                   DAT_00A8ED94 = *(param_1 + 0x92)
```

**Mechanism** (verified via `decompile_function 0x005b67f0`):

The function named `FUN_005b67f0` ("Decoding game options" — log string
`s_Decoding_game_options_0082c7a4`) receives a packet struct as `param_1`.
It copies `*(int *)(param_1 + 0x92)` directly into `DAT_00A8ED94`. This is
a **raw 4-byte field from a network packet**, not locally generated.

Other session settings copied in the same block: `DAT_00A8B25C` (starting credits),
`DAT_00822CF4` (tech level), `DAT_00A8B270` (unit count), `DAT_00A8B268` (game speed),
flags for bases/crates/fog/bridge-destroy/harvester-truce/short-game/superweapons/build-off-ally.
The seed field at +0x92 is in this same game-options struct.

**Active in YR:** Yes. This path is gated on `case 0x65` in the LAN packet handler called
from `Main_Game @ 0x0052E1D5 → FUN_005b49b0 → FUN_005b6020`. The log string
"Received game options" and `s_D__ra2mdpost_ModemGst_cpp_0082c534` source path confirms
this is the standard WOL/LAN multiplayer lobby (ModemGst.cpp).

**Who sets the seed on the host side:** Not fully traced (out of scope), but the format
structure of the "game options" packet is built on the host side. The counterpart writer
is the host's "send game options" path.

---

## 4. Path B — Internet (WOL/Online) "Decoding game options" Packet

**Call chain** (verified via `decompile_function 0x005e3d10`):

The function at `0x005e3d10` is a large WOL/Internet game-setup handler (log string
`s_Decoding_game_options__s_008318bc`). It reaches a block:

```c
iVar10 = CRT__strtok(pcVar6, &DAT_00817f70);
if (iVar10 != 0) {
    DAT_00a8ed94 = CRT__atoi(iVar10);   // <-- seed from first token of options string
}
// … further tokens: DAT_00a8eb60, DAT_00a8b268, DAT_00a8b25c, DAT_00a8b270, flags…
```

Verified at instruction `0x005e4dc1` inside `FUN_005e3d10` via `decompile_function 0x005e3d10`.

The seed is the **first `\0`-delimited token** of a text options string received over the
WOL (Westwood Online) connection. It is parsed with `CRT__strtok + CRT__atoi`. The format
string `s__d__d__d__d__d__d__d__d__d__d__d_00831360` (used by the host-side options-string
**encoder** `FUN_005dbb60`) confirms the options string is `%d %d %d %d …` — the seed
is serialised as a decimal integer, first field.

**Correction (2026-05-29):** `FUN_005dbb60` is the host-side options-string **encoder/serializer**,
not merely a "logger." Re-confirmed via `decompile_function 0x005dbb60`: it calls
`FUN_007c8ef4(param_1, s__d..._00831360, DAT_00a8ed94, DAT_00a8eb60, DAT_00a8b268,
DAT_00a8b25c, DAT_00a8b270, …)`, i.e. it sprintf-formats the options string into `param_1`
with `DAT_00A8ED94` (the seed) as the **first `%d` field**, followed by the same session
settings the Path B parser reads back in the same order. This is the counterpart to the
Path B decoder — the encoder referenced as "not traced" in §11.2 below.

**Active in YR:** Yes — this path is active when `g_GameMode == 4` (Internet). The function
at `0x005e3d10` is gated on packet type checks and WOL connection state. The `if (g_GameMode
== 4 && (DAT_00b77dc1 != '\0' || DAT_00b77e28 != 0))` early-exit guard in this function
affects NAT traversal packets ('Z' prefix), not the options-string path.

---

## 5. Path C — Modem Host "InitDialog" (`_rand()` local seed generation)

**Location** (verified via `decompile_function 0x005b82f0`):

Inside the large modem host init-dialog function `0x005b82f0` ("ModemHost_InitDialog"),
after setting up the lobby UI:

```c
uVar6 = Random__RandomRanged(1, 0x7fff);   // draws from g_MainRng (address 0x886B88)
FUN_007cb49d(uVar6);                        // seeds CRT _srand() with result
DAT_00a8ed94 = _rand();                    // host seed = CRT rand() output
```

Log string `s_ModemHost_InitDialog_exit__0082c974` confirms this is the **host** of a
LAN/modem game. The host generates the seed locally using `_rand()` (seeded from
`Random__RandomRanged`), stores it in `DAT_00A8ED94`, then broadcasts it to guests
in the "game options" packet (Path A).

**Active in YR:** Yes — this is the LAN host path. Confirmed by call from `FUN_005b82f0`
which is the modem/LAN host dialog.

---

## 6. DAT_00A8B8B8 Gate Confirmation

`get_xrefs_to 0x00A8B8B8` returned 9 references. Key write is
`FUN_00652230 @ 0x00652335` (verified via `decompile_function 0x00652230`).
This function is the event-queue overflow handler — when the queue fills it sets
`DAT_00A8B8B8 = 0` to reset connection state, NOT to set it non-zero.

The non-zero set of `DAT_00A8B8B8` comes from `FUN_006475f0` (the main per-frame network
driver, verified via `decompile_function 0x006475f0`): it reads
`iVar9 = DAT_00a8b8b8` at the top of the frame loop to gate the "connection active" block.
Multiple sites set `DAT_00A8B8B8 = 0` on desync/timeout/sign-off. The flag is set
non-zero by the network session-connect flow (not fully traced here).

The gate in `Init_Random_Number_System @ 0x0052FC20`:
```c
if (DAT_00A8B8B8 == 0 && (DAT_00A8D5F8 & 2) == 0 && DAT_00AA0444 == 0) { /* SP entropy */ }
```
means: if `DAT_00A8B8B8 != 0` (MP active), **skip entropy gather** and use whatever is
already in `DAT_00A8ED94`. This seed was set by Path A, B, or C before `Init_Random_Number_System` ran.
Verified via `decompile_function 0x0052FC20` (RNG_SYSTEM_GHIDRA_REPORT.md §4.1).

---

## 7. Replay Recording Path

`FUN_00531960` (verified via `decompile_function 0x00531960`):
```c
(*vtable_read)(param_1, &DAT_00822CF4, 4);
(*vtable_read)(param_1, &DAT_00A8ED94, 4);   // <-- read seed from recording
```
This is the recording-load ("Loaded recording values for scenario") path — reads `DAT_00A8ED94`
as the second 4-byte field. Consistent with `RNG_SYSTEM_GHIDRA_REPORT.md §4.3`.

---

## 8. Gameplay Instance Confirmation (Target b)

**Primary claim**: `g_MainRng @ 0x00886B88` is the dominant per-frame sim draw instance.
`Scen->Random` is used only in a narrow subset of calls.

**Verified evidence from `HouseClass__Update @ 0x004F8440`** (decompiled this session):

```asm
// At 0x004F887D (previously documented):
MOV ECX, 0x886B88
CALL Random__RandomRanged    // RandomRanged(0, 1) — MP cursor RNG-eat
MOV ECX, 0x886B88
CALL Random__RandomRanged    // RandomRanged(0, 2) — cell state roll
```

Both calls in `HouseClass__Update` confirmed as `ECX = 0x00886B88` = `g_MainRng`.
Same pattern for combat/warhead/particles per `RNG_SYSTEM_GHIDRA_REPORT.md §3.1`.

**Scen->Random callers** (`Scen + 0x218`, dynamic address via `[0x00A8B230] + 0x218`):
Confirmed from prior session: `InfantryClass__Scatter @ 0x0051D2AC` and
`0x0051D36D` (scatter direction `RandomRanged(0, 4)`), and the conditional cell-state
roll in `HouseClass__Update @ 0x004F88FA` (distinct from the `g_MainRng` rolls above).

**Split summary:**

| System | RNG instance | Evidence |
|--------|-------------|---------|
| Warhead scatter/damage | `g_MainRng` | RNG_SYSTEM_GHIDRA_REPORT §3.1 |
| Sound variant selection | `g_MainRng` | RNG_SYSTEM_GHIDRA_REPORT §3.1 |
| Combat/particle/ore/lightning | `g_MainRng` | RNG_SYSTEM_GHIDRA_REPORT §3.1 |
| House AI cursor-sync MP eat | `g_MainRng` | decompile_function 0x004F8440 |
| Infantry scatter direction | `Scen->Random` | RNG_SYSTEM_GHIDRA_REPORT §3.2 |
| HouseClass::Update cell-state | `Scen->Random` | RNG_SYSTEM_GHIDRA_REPORT §3.2 |
| Map gen terrain | `g_MapGenRng` | RNG_SYSTEM_GHIDRA_REPORT §3.3 |

**Conclusion for target (b):** `g_MainRng` is the canonical simulation draw stream for
the vast majority of determinism-critical systems. `Scen->Random` is a secondary stream
used by scatter direction and a narrow HouseClass condition. Both are seeded from the same
`DAT_00A8ED94` at game start and diverge independently.

---

## 9. Implementation Handoff

### 9.1 Rust SimRng seeding contract for lockstep

**Verified behavior:** In a YR MP session (LAN or Internet), the host generates or receives
a `u32` seed, broadcasts it in the "game options" packet (as a decimal integer string for
Internet or as a packet struct field for LAN), and all clients store this identical seed in
`DAT_00A8ED94`. `Init_Random_Number_System` then seeds both `g_MainRng` and `Scen->Random`
from this value using `RandomClass_Seed` (the R(250,103) XOR-LFG seeder with the 16-dword
mixing tables). All clients start with byte-identical RNG state.

**Rust delta:** `SimRng` uses xorshift64\* seeded by a hardcoded constant. For lockstep
MP correctness the Rust engine needs: (a) a `u32` seed established before game init via
network handshake (one host generates it, all clients receive it); (b) that seed fed into
the `RandomClass_Seed`-equivalent initialization once; (c) the resulting state replaces the
default constant seed. The algorithm mismatch (xorshift64\* vs R(250,103)) is a pre-existing
known delta; the new handoff is the **seeding contract**, not the algorithm.

**Affected surface:** `SimRng` initialization in `src/sim/world/mod.rs` (line ~71 default
seed), the MP session setup path (currently unimplemented net layer), and any test that
asserts specific RNG outputs for a given seed — those tests must use a deterministic seed
injected via the same handshake path.

**Acceptance scenario:** Start two headless clients with identical `u32` seed. After 300
frames, both `SimRng` states must be identical. With different seeds, they must diverge.

**Proposed test name:** `test_mp_sibling_rng_state_matches_after_seed_sync`

**Risk:** HIGH — any MP session where seed is not shared produces deterministic drift from
frame 1. All per-frame RNG consumption (scatter, damage jitter, particle spawn) will diverge.

### 9.2 Two-stream design

**Verified behavior:** `g_MainRng` and `Scen->Random` start in identical state but diverge
independently as the game runs. `Scen->Random` is serialised with `ScenarioClass` (save/load),
`g_MainRng` is not. Infantry scatter and a narrow HouseClass roll draw from `Scen->Random`,
not `g_MainRng`.

**Rust delta:** Rust has one `SimRng`. For save/load parity and exact scatter-direction
parity, a second `SimRng` (`ScenRng`) must be introduced tracking `Scen->Random`.
Callers must be split: scatter draws from `ScenRng`, everything else from `SimRng`.

**Affected surface:** `src/sim/world/mod.rs` (SimRng field), `src/sim/components.rs`
or equivalent (scatter direction), `src/sim/world/world_spawn.rs` (seeding both at start).

**Acceptance scenario:** `test_infantry_scatter_uses_scen_rng_not_main_rng` — inject
a known seed, fire a scatter draw, confirm `ScenRng` advanced and `SimRng` did not.

**Risk:** MEDIUM — incorrect stream assignment produces deterministic scatter drift between
save-restored and non-restored sessions.

### 9.3 Host-seed generation (LAN)

**Verified behavior:** LAN host generates seed as `_rand()` seeded from
`Random__RandomRanged(1, 0x7FFF)` drawn from `g_MainRng`. This is the host-local seed
source before the game starts (when `g_MainRng` is uninitialized — it likely has BSS/zero
state, making `Random__RandomRanged(1, 0x7FFF)` return its `low` guard and
`_srand(1)` → `_rand()` returns a fixed value on every host). Practically this is
equivalent to a reproducible-but-different-per-build host seed; the mechanism is
pre-game-init RNG use to bootstrap `_srand`.

**Rust delta:** Rust MP host should generate a `u32` seed before game init and broadcast
it; the exact generation algorithm (`_rand()` seeded by a pre-game RNG draw) need not be
replicated — any non-zero `u32` that is the same on all clients works. The parity
requirement is that the seed is **identical on all clients**, not that the host uses a
specific RNG algorithm to generate it.

**Affected surface:** Net session-init path (currently unimplemented).

**Acceptance scenario:** `test_host_seed_matches_guest_seed_at_game_start`.

**Risk:** LOW — host seed generation algorithm is not observable externally; only the
seed value reaching all clients matters.

---

## 10. Negative Facts / Do Not Do

1. **Do not implement a separate "seed scramble" step.** No scramble/encryption of the
   seed was found between host generation and guest receipt. Seed travels verbatim.
   (verified via `FUN_005b67f0`: direct `*(param_1 + 0x92)` copy, no XOR/hash)

2. **Do not assume `DAT_00A8B8B8` is set non-zero during the handshake.** The non-zero
   set location was not found in this session; only zero-write sites were verified. The
   gate in `Init_Random_Number_System` uses `DAT_00A8B8B8 != 0` to skip entropy, but the
   setter is in the session-connect flow (deeper network init, not traced here).

3. **Do not use `g_MapGenRng` for any per-frame sim draw.** It is initialised separately
   from `DAT_00A8ED94` and never consumed during a normal YR tick.
   (verified via `RNG_SYSTEM_GHIDRA_REPORT.md §3.3`)

4. **Do not route infantry scatter through `g_MainRng`.** Scatter direction
   (`RandomRanged(0, 4)`) uses `Scen->Random` (`[0x00A8B230] + 0x218`).
   (verified via `decompile_function 0x0051D0D0`, `RNG_SYSTEM_GHIDRA_REPORT §3.2`)

5. **Do not treat `_rand()` seed generation on the host as parity-critical.** The mechanism
   (`Random__RandomRanged → _srand → _rand`) is pre-game-init plumbing; the parity bar is
   that the resulting `u32` is broadcast and received identically, not that the host used
   this specific derivation. The Rust host can use any secure/deterministic `u32` generator.

---

## 11. Remaining Uncertainty

1. **Who sets `DAT_00A8B8B8` non-zero** during MP session connect. The zero-writers
   (FUN_00652230, FUN_006475f0) were found but the positive-setter was not traced.
   This does not affect the seed contract (seed is already set by Paths A/B/C before
   Init_Random_Number_System runs), but understanding the exact MP state machine
   sequence would complete the picture.

2. **WOL Internet host-side options-string builder — RESOLVED (2026-05-29).** The host
   encodes the seed as the first decimal field of the options string. The encoder is
   `FUN_005dbb60`: it calls `FUN_007c8ef4(param_1, s__d..._00831360, DAT_00a8ed94, …)`,
   sprintf-formatting `DAT_00A8ED94` as the first `%d` field of the `%d %d %d …` options
   string (verified via `decompile_function 0x005dbb60`). This is the exact counterpart to
   the Path B `CRT__strtok + CRT__atoi` decoder. The prior "not traced" note is superseded;
   see §4.

3. **Whether `Scen->Random` is serialised in saves.** The claim in RNG_SYSTEM_GHIDRA_REPORT
   §3.2 is inferred from "ScenarioClass save state". Not directly verified via
   `ScenarioClass::Save/Load` decompilation this session.

---

## 12. Sources

**Ghidra calls this session:**
- `get_xrefs_to 0x00A8ED94` — all WRITE sites enumerated
- `decompile_function 0x005b67f0` — LAN "Decoding game options" writer (`FUN_005b67f0`)
- `decompile_function 0x005b6020` — LAN packet handler (switch case 0x65)
- `decompile_function 0x005b49b0` — ModemGuest_Dialog loop
- `decompile_function 0x005b82f0` — ModemHost_InitDialog (host `_rand()` path)
- `decompile_function 0x005e3d10` — WOL/Internet "Decoding game options" (Path B)
- `decompile_function 0x005dbb60` — Options-string logger (DAT_00A8ED94 read for log)
- `decompile_function 0x00531960` — Recording-load path (reads DAT_00A8ED94 from file)
- `decompile_function 0x00652230` — Event-queue overflow handler (DAT_00A8B8B8 zero-write)
- `decompile_function 0x006475f0` — Per-frame network driver (DAT_00A8B8B8 reads/zero-writes)
- `get_xrefs_to 0x005b67f0` — LAN decode-options callers
- `get_xrefs_to 0x005b6020` — packet-handler callers
- `get_xrefs_to 0x005b49b0` — guest-dialog callers
- `get_xrefs_to 0x00A8B8B8` — all write/read sites for MP gate flag
- `decompile_function 0x004F8440` — HouseClass__Update (g_MainRng confirmation)

**Prior docs:**
- `RNG_SYSTEM_GHIDRA_REPORT.md` — algorithm, layout, three instances, seed pipeline (authoritative)
- `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md` — RandomRanged helper

**Rust files:**
- `src/sim/rng.rs` — xorshift64\* SimRng (algorithm delta noted, out of scope here)
- `src/sim/world/mod.rs` — SimRng field and default seed `0x5EED_CAFE_D15E_A5E5`
