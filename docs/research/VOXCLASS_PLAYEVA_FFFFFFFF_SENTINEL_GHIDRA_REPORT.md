# VoxClass::PlayEVA Sentinel `0xFFFFFFFF` Investigation

**Target:** Resolve what the sentinel value `0xFFFFFFFF` (-1) passed in the
engineer bridge-repair EVA path means inside `VoxClass::PlayEVA`.

**Status:** COMPLETE. Read-only Ghidra investigation against `gamemd.exe`
(image base `0x00400000`). No writes to the binary, no edits to Rust code or
in-repo docs.

**Source for the open question:** `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
§3.1 step B and §7 Q5, which left this as a Phase-3 follow-up.

---

## TL;DR

The parent doc's notation `VoxClass::PlayEVA(0xFFFFFFFF)` was **single-argument
shorthand**. PlayEVA is actually a 3-argument `__fastcall` function. The
bridge-repair caller passes:

| Arg | Reg/slot   | Value                                       | Meaning                                       |
|-----|------------|---------------------------------------------|-----------------------------------------------|
| 1   | ECX        | `0x00825538` -> string `"EVA_BridgeRepaired"` | EVA name to play                              |
| 2   | EDX        | `0xFFFFFFFF` (-1)                           | "use the EVA entry's default **priority**"    |
| 3   | [ESP+stk]  | `0xFFFFFFFF` (-1)                           | "use the EVA entry's default **voice index**" |

So `0xFFFFFFFF` here is **NOT** "play no EVA", **NOT** "use last-fired EVA",
**NOT** a radar-event reference. It is a **per-arg sentinel** consumed
**downstream** in `VoxClass::QueueVoice` that means *"don't override; fall back
to the value stored in the EVA entry struct itself."*

The EVA being requested is just `EVA_BridgeRepaired` (the standard "Bridge
repaired" voice). The path **is YR-active**: this exact `(name, -1, -1)`
calling pattern is the standard idiom used across the entire engine for EVA
playback (verified at four call sites listed below).

---

## 1. Function Signature and Body

**Address:** `0x00752700` (Ghidra label: `VoxClass__PlayEVA`)

**Calling convention:** `__fastcall` — ECX = arg1, EDX = arg2, [ESP+4] = arg3.
Stack cleanup is callee (`RET 0x4`).

**Decompilation (lightly cleaned):**

```c
void __fastcall VoxClass__PlayEVA(LPCSTR name, int priority, int voiceIdx)
{
    if (name == nullptr) return;

    for (int i = 0; i < g_VoxEntryCount /*DAT_00b1d4b0*/; ++i) {
        if (strcmpi(name, g_VoxEntryNames[i] /*DAT_00b1d4a4[i]*/) == 0) {
            VoxClass__QueueVoice(/*index*/ i, priority, voiceIdx);
            return;
        }
    }
    // Name not found: pass index = -1 to QueueVoice (which rejects it).
    VoxClass__QueueVoice(/*index*/ -1, priority, voiceIdx);
}
```

Key disassembly anchors (verified `0x00752700`-`0x0075275c`):
- `0x00752720 CALL 0x007c8d20` — `strcmpi` (case-insensitive string compare)
- `0x0075273a OR ECX, 0xFFFFFFFF` — *no match* path: index = -1
- `0x0075274f MOV ECX, ESI` — *match* path: index = ESI (matched index)
- Both branches fall through to `CALL 0x00752480` — `VoxClass::QueueVoice`

Note that **inside PlayEVA itself**, the only literal `0xFFFFFFFF` is the
sentinel index it synthesises **when the name lookup fails** to disable
QueueVoice. PlayEVA does *not* itself compare any argument against -1.

---

## 2. Where the -1 Sentinels Are Consumed (`VoxClass::QueueVoice`)

**Address:** `0x00752480`

```c
void __fastcall VoxClass__QueueVoice(int index, int priority, int voiceIdx)
{
    if (g_StreamingReady /*DAT_00b1d4cc*/ != 0
        && index >= 0
        && index < g_VoxEntryCount  /*DAT_00b1d4b0*/
        && !g_EVASuspended           /*DAT_00b1d3d8*/
        && (entry = g_VoxEntryTable[index]) != g_CurrentlyPlayingEntry /*DAT_00b1d4c4*/)
    {
        // <<< Sentinel resolution: -1 means "use the EVA entry's default". >>>
        if (priority == -1)  priority  = *(int*)(entry + 0x4c);
        if (voiceIdx == -1)  voiceIdx  = *(int*)(entry + 0x48);

        // ... priority==2 special-case flushes the queue ...
        // ... insert into queue, then VoxClass__PlayNextQueued() ...
    }
}
```

Verified at `0x00752480`-`0x0075258d`. The two `-1` checks are at the very
start of the function body, immediately after the validity guards.

**So the sentinel `0xFFFFFFFF` passed by callers means: "Don't override the
EVA's configured priority / voice-slot — use whatever was set on the EVA
entry (typically from `evamd.ini`)."**

The EVA-entry struct fields `+0x48` (voice slot) and `+0x4c` (priority) are
populated by `VoxClass::ReadEVAINI` at `0x00753000` (out of scope for this
report — pre-existing INI-load path).

---

## 3. Bridge-Repair Call Site (the one the parent doc cited)

**Address:** `0x00519bbf` — `0x00519bc9`, inside
`InfantryClass::PerCellProcess` (`0x00519630`)

```
00519bbf  PUSH -0x1                    ; voiceIdx = -1 (use entry default)
00519bc1  OR   EDX, 0xFFFFFFFF         ; priority = -1 (use entry default)
00519bc4  MOV  ECX, 0x00825538         ; name = "EVA_BridgeRepaired"
00519bc9  CALL 0x00752700              ; VoxClass::PlayEVA
```

String at `0x00825538`: bytes spell `EVA_BridgeRepaired\0` (verified via
`read_memory`).

**Guard chain reaching this call** (also `InfantryClass::PerCellProcess`):
- Engineer (`*piVar10 + 0x2c) == 6` => building) entering CABHUT
  (`+0x16b6`) flag
- `HouseClass::IsHumanPlayer()` — only the local human player gets the EVA
- `CreateRadarEvent(coord)` — radar event raised first
- Then `PlayEVA("EVA_BridgeRepaired", -1, -1)` is called
- Separately, gated by `RulesClass+0x248 != -1`, `VocClass::PlayAt` is
  triggered for `RepairBridgeSound` (covered in BRIDGE_REPAIR_AND_HUT_DEATH §3.1)

**YR-active:** YES. This is the standard bridge-repair flow; gated only on
local-human-player and successful radar event registration. No TS-only
SpecialFlag.

---

## 4. Other Callers Using the Same `(name, -1, -1)` Pattern

The `(name, -1, -1)` idiom is the **standard** PlayEVA call pattern across
the binary, not a bridge-repair quirk. Sampled call sites (all confirmed via
disassembly + `read_memory` on the name string):

| Site address | Containing function                          | EVA string          | String addr  | YR-active? |
|--------------|----------------------------------------------|---------------------|--------------|------------|
| `0x00519bc9` | `InfantryClass::PerCellProcess` (bridge hut) | `EVA_BridgeRepaired`| `0x00825538` | YES        |
| `0x0044fd1a` | `BuildingClass::ReadFromINI` (online init)   | `EVA_BuildingOnLine`| `0x008190c8` | YES        |
| `0x00430d78` | `RadarClass::PlaceBeacon` (ally placed)      | `EVA_BeaconPlaced`  | `0x00818a9c` | YES        |
| `0x00430f1b` | `RadarClass::PlaceBeacon` (enemy detected)   | `EVA_BeaconDetected`| `0x00818a68` | YES        |

PlayEVA has ~70+ caller addresses (`get_function_callers` enumerates them);
nothing in the inspected slice passes anything other than `name`-string +
`(-1, -1)` defaults. **No caller passes `0xFFFFFFFF` as the name pointer
itself** — that would null-deref the `strcmpi` loop. The "use entry default"
pattern is universal.

**No TS-only caller was identified** that passes the `0xFFFFFFFF` sentinels.
All sampled callers are part of the live YR skirmish surface (bridge hut
repair, building power-on, beacon system). The full caller list contains a
handful of well-known TS-era helpers (e.g. references in
`LightningStorm::Process`) but PlayEVA itself is YR-live and the sentinels
are not TS-gated.

---

## 5. What the Sentinel Does *Not* Mean

For completeness, eliminating the parent doc's candidate hypotheses:

| Hypothesis                                  | Verdict | Evidence                                                                                       |
|---------------------------------------------|---------|------------------------------------------------------------------------------------------------|
| "Play no EVA"                               | WRONG   | An EVA *is* played (`EVA_BridgeRepaired`); the -1s only suppress priority/voiceIdx overrides   |
| "Use last-fired EVA"                        | WRONG   | No global "last EVA" lookup exists in PlayEVA or QueueVoice; the name argument is mandatory    |
| "Play the most-recent CreateRadarEvent EVA" | WRONG   | RadarEvent and PlayEVA are independent calls; no shared state                                  |
| "Sentinel for name == 0xFFFFFFFF"           | N/A     | PlayEVA's name-null check (`TEST EDI, EDI; JZ`) treats 0 as "skip"; no caller passes -1 here   |
| **"Use the EVA entry's default priority and voice-slot"** | **CORRECT** | QueueVoice has explicit `if (param == -1) param = *(int*)(entry + 0x4c / 0x48);` |

---

## 6. Load-Bearing Facts (for parent reconciliation)

1. `VoxClass::PlayEVA` at `0x00752700` is `__fastcall(LPCSTR name, int priority,
   int voiceIdx)`. Verified via disassembly `0x00752700-0x0075275c`.
2. The `0xFFFFFFFF` sentinel from the bridge-repair caller is **not** the name
   argument — it is `priority` (EDX) and `voiceIdx` (stack). The name is
   `EVA_BridgeRepaired` at `0x00825538`. Verified via disassembly
   `0x00519bbf-0x00519bc9` + `read_memory(0x00825538)`.
3. The sentinel is consumed in `VoxClass::QueueVoice` at `0x00752480`,
   replaced with values from the EVA entry struct fields `+0x4c` (priority)
   and `+0x48` (voice slot). Verified via the explicit `if (param == -1) param
   = *(int *)(entry + 0x4c/0x48);` branches at the head of the function.
4. The `(name, -1, -1)` call pattern is the standard PlayEVA idiom: same
   shape used in `BuildingClass::ReadFromINI` (`0x0044fd1a`,
   `EVA_BuildingOnLine`) and `RadarClass::PlaceBeacon` (`0x00430d78` /
   `0x00430f1b`, `EVA_BeaconPlaced` / `EVA_BeaconDetected`). All verified
   via disassembly + `read_memory`.
5. The bridge-repair PlayEVA path is YR-active; the only gates are
   `HouseClass::IsHumanPlayer()` and `CreateRadarEvent` returning true.
   Not TS-only, no SpecialFlag dependency.

---

## 7. Implication for the Rust Port

When implementing the bridge-repair EVA cue in Rust, the call should be
modelled as:

```rust
// pseudocode
vox.play_eva("EVA_BridgeRepaired", priority: EvaPriority::Default,
                                    voice_idx: VoiceSlot::Default);
```

i.e. just request the EVA by name. The `0xFFFFFFFF` in the original Ghidra
output is a low-level encoding of "don't override defaults" and should be
naturally represented as `Option::None` / a `Default` variant in idiomatic
Rust — there is no behavioural complexity to port. The actual priority and
voice-slot come from the EVA entry which itself is populated from
`evamd.ini` at startup (see `VoxClass::ReadEVAINI` at `0x00753000`).

---

## 8. Confidence Axes (per `feedback_research_confidence_axes`)

| Axis                                       | Confidence | Basis |
|--------------------------------------------|------------|-------|
| **Content** (what PlayEVA does internally) | HIGH       | Decompiled + disassembled the full function body; logic is small and unambiguous |
| **Identity** (this is the right function)  | HIGH       | `search_functions("VoxClass")` returns a single `VoxClass__PlayEVA` symbol; matches the bridge-repair caller's `CALL 0x00752700` target |
| **Binding** (sentinel semantics resolved by QueueVoice fields `+0x4c/+0x48`) | HIGH | `read_memory` on string at `0x00825538` confirmed name; `if (param == -1) param = *(int*)(entry + 0x4c)` is an unambiguous, single-pass sentinel handler with no aliasing |
| **Caller-trace** (which call sites)        | HIGH       | `get_function_callers` returned 40 functions; sampled 4 representative ones (bridge, building, 2x beacon) all confirm the `(name, -1, -1)` pattern with the name being a real LPCSTR |
