# Skirmish Cooperative Precall 0049B760 - Ghidra Research Report

**Address(es):** `0x0049B760`, caller `0x005C1D80`, related cooperative record helpers `0x0049B610`, `0x0049B720`, `0x0049C0D0`, `0x0049CAF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the `0x0049B760` helper reached by Cooperative mode's vtable `+0x14` Start-acceptance path, and whether that helper affects visible Skirmish Start acceptance, team/alliance values, or immediate session packing.  
**Non-Scope:** full cooperative campaign progression UI, full `MPCoopMD.ini` scenario sequence rules, and gameplay mission startup after shell exit.  
**Confidence:** High for the helper's writes and Start-acceptance effect; Medium for naming the `this+0x40` object as a cooperative progress/save record because that is inferred from adjacent `coopsave.ini` reads and field layout.  
**Active in YR:** Conditional. The path is active when the selected YR `MPModesMD.ini` mode category is `Cooperative`, the Skirmish node count is exactly `2`, and the Cooperative mode object's field `+0x40` is non-null. Evidence: `0x005C1D80..0x005C1DA0`; mode list includes `[Cooperative]` in `ini/mpmodesmd.ini:26..27`.

## 1. Overview

`0x0049B760` copies the first two Skirmish node names into a Cooperative-mode side record: node0 becomes a narrow string at record `+0x00`, node1 becomes a narrow string at record `+0x1C`. Null node pointers clear the corresponding destination string to an empty NUL byte.

In the Cooperative `+0x14` Start-acceptance method, this helper is called before the unconditional base accept method `0x005D6310`. Its return value is void, and the caller does not branch on any side effect from it. The visible Start accept/reject result is therefore unchanged by the helper in this slice.

## 2. Key Offsets

| Object / offset | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Cooperative mode object `+0x40` | pointer to cooperative progress/save record | caller loads `ECX=[ESI+0x40]` before `CALL 0x0049B760` | Conditional, Cooperative selected and pointer non-null |
| coop record `+0x00` | narrow player/name string for first node | `0x0049B760` copies converted `param_2` string to `param_1` | Conditional |
| coop record `+0x1C` | narrow player/name string for second node | `0x0049B760` copies converted `param_3` string to `param_1+0x1C` | Conditional |
| coop record `+0x14/+0x18` | house/color for first player from `coopsave.ini` | `0x0049C0D0` reads `House1`/`Color1` into those offsets | Conditional, when save/progress file parse succeeds |
| coop record `+0x30/+0x34` | house/color for second player from `coopsave.ini` | `0x0049C0D0` reads `House2`/`Color2` into those offsets | Conditional |
| coop record `+0x38` | current cooperative map index/progress | `0x0049C0D0` reads `CurrentMap` into `+0x38`; `0x0049CAF0` resets it to `0` for new campaign type | Conditional |
| coop record `+0x3C/+0x40` | allocated map-name pointer array and count | `0x0049C0D0` allocates `count * 4` at `+0x3C` and stores count at `+0x40`; `0x0049B720` frees entries | Conditional |
| coop record `+0x44` | campaign type / sequence index | `0x0049C0D0` reads `CampaignType` into `+0x44`; `0x0049CAF0` updates it | Conditional |
| coop record `+0x6C` | initialized/valid flag | `0x0049B610` initializes `0`; `0x0049C0D0` and `0x0049CAF0` set `1` on success | Conditional |

## 3. Core Logic

### Cooperative `+0x14` caller

The caller at `0x005C1D80` performs three gates before the helper call:

1. `DAT_00A8DA84 == 2`.
2. `this+0x40 != 0`.
3. `DAT_00A8DA78` supplies node pointers at indexes `0` and `1`.

If all gates pass, it calls `0x0049B760(this+0x40, node0, node1)`. It then pushes the original Start result-buffer argument and calls the base accept method `0x005D6310`. Active in YR: Conditional, selected Cooperative mode with exactly two nodes and a live coop record. Evidence: assembly `0x005C1D80..0x005C1DB2`.

If the node count is not exactly `2`, or the coop record pointer is null, the helper is skipped and the method still calls `0x005D6310`. Active in YR: Conditional. Evidence: branches from `0x005C1D8B` and `0x005C1D92` to `0x005C1DA5`.

### Helper behavior at `0x0049B760`

The helper is a two-string copier. It treats `param_2` and `param_3` as wide-string pointers, converts each via `0x007350C0`, computes a NUL-inclusive byte length with `SCASB`, and copies the bytes into fixed destinations using dword copies plus a `len & 3` byte tail. Active in YR: Conditional, only when called by Cooperative mode paths. Evidence: `0x0049B760..0x0049B7D2`; converter `0x007350C0`.

Null handling is explicit:

| Input | Destination effect | Evidence | Active in YR |
|---|---|---|---|
| `param_2 == 0` | writes `0` to record `+0x00` | `0x0049B768..0x0049B793` | Conditional |
| `param_2 != 0` | copies converted string to record `+0x00` | `0x0049B76D..0x0049B78F` | Conditional |
| `param_3 == 0` | writes `0` to record `+0x1C` | `0x0049B796..0x0049B7CB` | Conditional |
| `param_3 != 0` | copies converted string to record `+0x1C` | `0x0049B79E..0x0049B7C3` | Conditional |

The converter `0x007350C0` truncates/copies 16-bit characters into an eight-slot rotating static narrow buffer, max `0x400` bytes per slot, and returns null if the source pointer is null. Active in YR: Yes as the live conversion helper used by `0x0049B760`. Evidence: `0x007350C0`.

### What it does not do

`0x0049B760` does not read or write Skirmish node team/start fields such as node `+0x5B`, `+0x63`, or `+0x6B`. It only reads the input node pointer as a wide string source and writes into the coop record at `+0x00` and `+0x1C`. Active in YR: Conditional. Evidence: full helper body `0x0049B760..0x0049B7D2`.

`0x0049B760` cannot reject Start in this path. It returns `void`, and the caller immediately delegates to `0x005D6310`, whose body returns `1`. Active in YR: Conditional. Evidence: `0x005C1DA0..0x005C1DB2`; base accept method `0x005D6310`; Start caller branches only on the final AL at `0x006AD2D5`.

`0x0049B760` is before final offline Skirmish local-node packing, but it is not part of that packing block. The final local node is allocated later with size `0x85`, then receives name/start/team fields at `0x006AD647..0x006AD69C`; the helper does not participate in those writes. Active in YR: Yes for Start packing; Conditional for Cooperative helper. Evidence: Start caller `0x006AD2BA..0x006AD34B` then packing block `0x006AD647..0x006AD6F9`.

## 4. INI Keys / Data Files

| Path / key | Role | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini` `[Cooperative] 3=GUI:Cooperative, STT:ModeCooperative, MPCoopMD.ini, cooperative, false` | Makes Cooperative a standard YR selectable mode entry | `ini/mpmodesmd.ini:26..27`; mode loader evidence from parent report `0x005D7CE0` | Yes, when selected |
| `coopsave.ini` | Save/progress file read by adjacent cooperative record parser | `0x0049C0D0` constructs file using string `s_coopsave_ini_0081f0f8` | Conditional, if present/readable |
| `CurrentMap` | read into coop record `+0x38` | `0x0049C0D0` | Conditional |
| `CampaignType` | read into coop record `+0x44` | `0x0049C0D0` | Conditional |
| `House1`, `Color1`, `House2`, `Color2` | read into coop record player-side fields | `0x0049C0D0` | Conditional |
| `Map%d` | per-map string entries allocated into record `+0x3C` array | `0x0049C0D0`, format string `s_Map_d_0081f114` | Conditional |

## 5. Integration Points

`0x0049B760` is used not only in the Start `+0x14` method but also in adjacent Cooperative setup/progress refresh code around `0x005C1DC0`: it writes the two current node names before `0x0049C0D0` tries to read/validate cooperative save/progress data, and again after switching/allocating a campaign record. Active in YR: Conditional, Cooperative mode object with exactly two nodes. Evidence: calls at `0x005C1E0E` and `0x005C1EA6`.

This shows the player-name pair is the key used by the Cooperative progress record path. Inference: the Start-time precall refreshes the same identity strings immediately before launch acceptance, so later Cooperative campaign/save logic sees the current two player names. Confidence: Medium; the name-copy and `coopsave.ini` consumption are verified, but the later mission-load consumer after shell exit was out of scope.

## 6. Current Rust Implementation Status

The Rust Skirmish shell currently has no selected `MPModesMD.ini` game-mode object model and no Cooperative progress/save record equivalent. Start uses `SkirmishShellAction::StartGame` and `launch_settings` to build simple settings. Evidence: prior scan in `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`; local grep found Skirmish shell paths under `src/ui/skirmish_shell` and `src/app.rs`, but no `coopsave` or Cooperative progress implementation. Active in YR: not applicable to Rust status.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Cooperative `+0x14` gates | verified | `0x005C1D80..0x005C1DA0` | none for helper reachability |
| `0x0049B760` node0 copy | verified | `0x0049B768..0x0049B793` | exact max visible player-name length is from node construction, not this helper |
| `0x0049B760` node1 copy | verified | `0x0049B796..0x0049B7D2` | none for destination offset |
| wide-to-narrow converter | verified | `0x007350C0` | exact codepage/locale effects beyond low-byte truncation are not investigated |
| Start accept/reject effect | verified | `0x005C1DA5..0x005C1DB2`, `0x005D6310`, `0x006AD2D5` | none |
| team/alliance/session packing effect | verified-negative for this helper | `0x0049B760` body; packing block `0x006AD647..0x006AD6F9` | downstream Cooperative mission-load consumers are out of scope |
| `coopsave.ini` parser fields | touched-not-exhausted | `0x0049C0D0`, `0x0049CAF0`, `0x0049B610`, `0x0049B720` | full Cooperative campaign progression UI/save system |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What does `0x0049B760` write? It writes two converted node-name strings into coop record `+0x00` and `+0x1C`, or clears those strings when inputs are null. Evidence: `0x0049B760..0x0049B7D2`.

[RESOLVED] OQ-2 - Does the helper affect Start acceptance? No direct effect: it returns void, and Cooperative `+0x14` returns the result of `0x005D6310`, which returns `1`. Evidence: `0x005C1DA0..0x005C1DB2`, `0x005D6310`.

[RESOLVED] OQ-3 - Does the helper alter teams, alliances, start slots, or node role field `+0x6B`? No. The helper writes only coop record string destinations and does not write back to node records. Evidence: full body `0x0049B760..0x0049B7D2`.

[RESOLVED] OQ-4 - Why does the coop record contain these names? Adjacent cooperative record parser uses the two strings to build/read a `coopsave.ini` section and then fills progress fields such as `CurrentMap`, `CampaignType`, `House1/2`, `Color1/2`, and map list entries. Evidence: `0x0049C0D0`; calls around `0x005C1E0E` and `0x005C1EA6`.

[DEFERRED] OQ-5 - Which post-shell Cooperative mission-load code consumes the refreshed progress record? Category: out-of-scope. Reason: this slot only resolves the Start-acceptance precall and immediate session-packing effect; mission startup after shell exit requires a separate Cooperative campaign flow investigation.

## Sources

- Ghidra decompiled/read: `0x0049B760`, `0x007350C0`, `0x0049B610`, `0x0049B720`, `0x0049C0D0`, `0x0049CAF0`.
- Ghidra assembly/read: `0x005C1D80..0x005C1DB2`, `0x005C1DC0..0x005C1F0E`, `0x006AD2BA..0x006AD34B`, `0x006AD647..0x006AD6F9`.
- Prior report: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`.
