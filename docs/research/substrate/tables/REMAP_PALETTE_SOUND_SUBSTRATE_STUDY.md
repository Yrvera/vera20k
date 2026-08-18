# Remap / Palette / Sound Tables — Substrate Study

**Date:** 2026-06-04
**Author lane:** substrate study — Remap/Palette/Sound tables (Stage 2 design + write)
**Scope:** The Priority→ColorScheme remap table + house-color extraction (palette), the VocClass
name→index resolution + per-sound config tables, and the positional-SFX volume/pan + flag tables.
This is a **study (research + design)**. No Rust was written or modified.
**Confidence convention (per claim):**
- **VERIFIED-THIS-SESSION** — read live out of Ghidra/binary this session, the exact MCP call is cited inline.
- **DOC-HIGH** — taken from a prior verified `docs/research/` report, not re-read this session (marked).
- **UNCHECKED** — not verified this session and the prior source is doc-only or absent → default verdict **DRIFT**.

**Burden of proof:** default verdict on any value/ordering/indexing/formula difference is **DRIFT**.
There is no "internal-only" escape for this family — palette is a per-frame render output, sound is a
per-event audio output, and the name→index resolution is deterministic gameplay-adjacent state (EVA cue
selection, credit-tick lists, sound priority). Downgrades require algebraic proof, a bit-identical
boundary-inclusive test, or exhaustive caller verification. None of the DRIFTs below clear that bar.

---

## (1) Active-YR responsibilities

Three player-visible output channels, all live in every stock YR skirmish:

1. **House color remap (palette).** A lobby color *priority* (0..8) maps through a 9-byte table to a
   *ColorScheme index*; that selects a loaded ColorScheme object whose embedded 256-entry RGB remap
   palette recolors that house's units/buildings/cursors/radar dots. From the scheme the engine extracts
   one base RGB triplet (`House+0x56F9..0x56FB`) and a normalized "bright" triplet (`House+0x56FC..0x56FE`)
   for highlights. Fires once per house at creation (`Create_Houses`) and on `Color=`/`TriggerAction`;
   the remap palette is consumed every frame an owned object renders. Player-visible: faction tint, radar
   dots, lobby swatch, placement/rally visuals.
2. **Sound name→index resolution + per-sound config.** `[AudioVisual]` keys and `[SoundList]`/per-sound
   sections parse once at rules load into VocClass entries; INI strings resolve to a stable 0-based
   VocClass index via linear name search. Player-audible: which `.aud` plays for every click/build/sell/
   weapon/credit-tick/EVA-adjacent cue.
3. **Positional SFX playback + spatial mix.** `VocClass__PlayAtPos` (~75 live callers) dispatches
   in-game SFX; `CalcVolumeAndPan` computes per-sound volume falloff, stereo pan, and SHROUD/visibility
   gating. Player-audible: loudness, L/R balance, whether enemy sounds in unexplored cells are silenced.

---

## (2) Full inventory (gamemd contract)

### 2.1 Palette / Remap

| Symbol | Address | Verified | What |
|---|---|---|---|
| Priority→ColorScheme table | `0x0083ed14` | **VERIFIED-THIS-SESSION** (`read_memory 0x0083ed14` → `03 0b 15 1d 0d 19 11 0f 05 00 00 00 ff ff ff ff`) | `char[9]` = `{3,11,21,29,13,25,17,15,5}`; priority 0..8 → scheme 3,11,21,29,13,25,17,15,5. Followed by padding `00 00 00` then `DAT_0083ed1c`. |
| `DAT_0083ed1c` (default-scheme fallback) | `0x0083ed1c` | **VERIFIED-THIS-SESSION** (in the same 16-byte read = `ff ff ff ff` = `0xFFFFFFFF`) | Returned when priority == `0xFFFFFFFE`. |
| `SessionClass__PriorityToColorScheme` | `0x0069A310` | **VERIFIED-THIS-SESSION** (`decompile_function 0x0069A310`) | `if p==0xFFFFFFFE return DAT_0083ed1c; if p<9 p=(uint)(char)(&table)[p]; return p;` — signed-byte cast; **p≥9 returns p unchanged (no clamp, no bounds error)**. |
| `g_ColorSchemeArray` | `DAT_00b054d4` | DOC-HIGH (HOUSECLASS / radar-dot reports) | Ptr to DynamicVector of ColorScheme*; runtime-populated. |
| ColorScheme object (0x310 bytes) | — | DOC-HIGH (radar-dot report §4–5) | `+0x04` 256×3-byte RGB remap palette; `+0x304` name (strcmp key); `+0x30C` converted-pixel-data block; `+0x310` 2nd match key; `+0x330` remap pixel index; `+0x314..` fixed remap-band indices. |
| `FindColorSchemeIndex` / `FUN_00474A90` | `0x0068cab0` / `0x00474A90` | DOC-HIGH | Linear search; match on name strcmp AND `scheme+0x310` key; returns index or previous/default. |
| `HouseClass__InitColor` | `0x50B840` | **VERIFIED-THIS-SESSION** (`decompile_function 0x50B840`) | `if House+0x16054 < 0 →5; scheme=array[idx]; if scheme==0 force idx=5` (debug "Forcing House %s [%s] to color WHITE")`;` read converted pixel via `(scheme+0x30C → +0x174) + (scheme+0x330)*stride` (stride 1 if `*(scheme+0x30C)+4==1` else 2), unpack via DD shift/loss (`g_DD_R/G/B Shift/Loss`) per channel `(raw>>shift)<<loss` → write 3 bytes `House+0x56F9/56FA/56FB`. |
| `HouseClass__ComputeRemap` | `0x50BA00` | **VERIFIED-THIS-SESSION** (`decompile_function 0x50BA00`) | `len=Sqrt_Approx(R²+G²+B²)`; `len==0 → bright=(255,255,255)` (all three set to the `_DAT_007eaa50` high-cap const); else per channel `v=channel*_DAT_007e5f78/len`, high-cap `if v>_DAT_007eaa50 → _DAT_007eaa50`, low-cutoff `if v<_DAT_007eaa48 → 0`; then a trailing `Sqrt_Approx(R²+G²+B²)` (FPU-stack idiom) and three `ftol` pop the normalized R/G/B to bytes `House+0x56FC/56FD/56FE`. **Note: the channel multiplier and high-cap are named globals `_DAT_007e5f78`/`_DAT_007eaa50` (presumed 255.0 but UNREAD), NOT a hardcoded literal 255.** |
| DD format globals `g_DD_RShift/RLoss/...` | near `0x008a0dd0` | DOC-HIGH | Runtime DirectDraw surface masks; static bytes zero. |
| House `ColorSchemeIndex` | `House+0x16054` (4 bytes) | DOC-HIGH | Index into scheme array. |
| House base RGB | `House+0x56F9..0x56FB` (3 bytes) | DOC-HIGH (radar-dot §5 + HOUSECLASS field map) | Extracted display-format RGB; shared consumer (radar dots, target lines, selection). |
| House bright RGB | `House+0x56FC..0x56FE` (3 bytes) | DOC-HIGH | Normalized highlight RGB. |

### 2.2 Sound

| Symbol | Address | Verified | What |
|---|---|---|---|
| `g_VocArray` / count | `DAT_00b1d37c` / `DAT_00b1d388` | **VERIFIED-THIS-SESSION** (referenced in `decompile 0x007514d0`, `0x00750920`) | Ptr to array of VocClass* and its count. |
| `VocClass__FindByName` | `0x007514d0` | **VERIFIED-THIS-SESSION** (`decompile_function 0x007514d0`) | `name==0→-1`; linear `0..count`; compare input vs (`*entry==0 ? "Invalid Voc" : GetName(entry)`) via `FUN_007c8d20` strcmp; **returns first match (lowest index)** else -1. |
| `VocClass__FindPtrByName` | `0x00751520` | DOC-HIGH (decode contract) | input == `<none>` sentinel → 0; else scan → VocClass* or 0. |
| `VocClass__FindIndexByPtr` | `0x007515c0` | DOC-HIGH | scan, match stored ptr, return index or -1. |
| `VocClass__GetName` | `0x00405170` | DOC-HIGH | returns `entry+0x6c`, or `"<no events>"` if `DAT_0087e2a0==0`. |
| `VocClass__PlayAtPos` | `0x00750920` | **VERIFIED-THIS-SESSION** (`decompile_function 0x00750920`) | `if DAT_008464ac==0 →0`; resolve VocClass only if `-1<index<count` else iVar2=0; loop-handle revalidate/stop; alloc SoundEvent + SetVolume/SetPan only if iVar2!=0; **silent no-op (no crash) on OOB index**. |
| `VocClass__ReadINI` | `0x00750440` | DOC-HIGH (decode contract) | Per-sound keys in order: `Sounds`,`Volume`(default `_DAT_008464b4`),`VShift`,`MinVolume`(default `_DAT_008464b8`),`Priority`(ParsePriority),`Attack`,`Decay`,`Control`(ParseControlFlag loop), type field @`DAT_00824314`(ParseTypeFlag loop),`Limit`(default `DAT_008464c4`),loop field @`DAT_00824238`,`Range`(default `DAT_008464c0`),`Delay`(2 ints),`FShift`(2 ints). |
| `CCINIClass__ReadSoundList` | `0x00525430` | **VERIFIED-THIS-SESSION** (`decompile_function 0x00525430`) | `ReadString`→`strtok` by delim `DAT_00817f70`; per nonempty token `FindPtrByName`→if 0 skip; else `FindIndexByPtr`→append **index** to DVC; empty key → empty DVC; order = INI order. |
| `AudioEventClass__ParseControlFlag` | `0x00406820` | DOC-HIGH; table re-read this session | walk `{name,bit}` @`0x008160c0` to NULL term; matched → `flags|=bit`; unknown → OR 0 (no-op). |
| `AudioEventClass__ParseTypeFlag` | `0x00406870` | DOC-HIGH | walk pairs @`0x00816048`; exclusion: new bit in 0x60 clears 0x60; new bit in 0xc00 clears 0xc00; then OR. |
| `VocClass__CalcVolumeAndPan` | `0x00750ac0` | **VERIFIED-THIS-SESSION** (`decompile_function 0x00750ac0`) | See §5. |

### 2.3 Static tables (dumped this session)

**Control-flags table** @ `0x008160c0` — `{char* name, u32 bit}[8]` + NULL terminator
(**VERIFIED-THIS-SESSION** `read_memory 0x008160c0` len 72; name-pointer order `0x816148,0x816140,0x816138,0x81612c,0x816120,0x816118,0x816110,0x8161b8` then `00000000`):

| Order | Name | Bit |
|---|---|---|
| 0 | `ALL` | `0x04` |
| 1 | `LOOP` | `0x01` |
| 2 | `RANDOM` | `0x02` |
| 3 | `PREDELAY` | `0x08` |
| 4 | `INTERRUPT` | `0x10` |
| 5 | `ATTACK` | `0x20` |
| 6 | `DECAY` | `0x40` |
| 7 | `AMBIENT` | `0x80` |

> **Correction vs the AUDIO_CHANNEL doc §6 (DRIFT in the doc, not the binary):** `PREDELAY=0x08`,
> `INTERRUPT=0x10` — the doc had them swapped (`INTERRUPT 0x08 / PREDELAY 0x10`). The binary bytes are
> authoritative.

**Type-flags table** @ `0x00816048` — `{char* name, u32 bit}` to NULL term
(**VERIFIED-THIS-SESSION** `read_memory 0x00816048` len 120 + name-string reads `0x0081617c`/`0x008161b8`):
`AMBIENT`=0x1000 (name @0x8161b8, shared string with Control AMBIENT), `VIOLENT`=0x01, `MOVEMENT`=0x02,
`QUIET`=0x04, `LOUD`=0x08, `GLOBAL`=0x10, `SCREEN`=0x20, `LOCAL`=0x40, `PLAYER`=0x80,
(empty default @0x8161d4)=0x00, `GUN_SHY`=0x200, `NOISE_SHY`=0x100, `UNSHROUD`=0x400, `SHROUD`=0x800.
Byte order in table: AMBIENT(0x1000), VIOLENT, MOVEMENT, QUIET, LOUD, GLOBAL, SCREEN, LOCAL, PLAYER, (empty)0x00,
GUN_SHY, NOISE_SHY, UNSHROUD, SHROUD, then NULL term. Exclusion groups: `0x60` (SCREEN/LOCAL) and `0xc00`
(UNSHROUD/SHROUD) — last-wins within group.

**Priority table** @ `0x00816018` — `{char* name, u32 value}[5]` + `{NULL,2}` default-terminator
(**VERIFIED-THIS-SESSION** `read_memory 0x00816018` → values `0,1,2,3,4` then `00000000 02000000`):

| Name | Value |
|---|---|
| `LOWEST` | 0 | (name @0x8161e0) |
| `LOW` | 1 | (name @0x8161dc) |
| `NORMAL` | 2 | (name @0x8161d4) |
| `HIGH` | 3 | (name @0x8161cc) |
| `CRITICAL` | 4 | (name @0x8161c0) |
| (unknown token) | **2** (NORMAL, from terminator) | name-strings VERIFIED-THIS-SESSION `read_memory 0x008161c0`/`0x008161dc` |

**Sentinel strings:** `"Invalid Voc"` @`0x846574`, `"<no events>"` @`0x816204`, type-field key @`0x00824314`,
ReadSoundList delim @`0x00817f70`, Voc delim @`0x00846570`, `<none>` sentinel `DAT_00817474`.

### 2.4 RulesClass sound-index fields + 3 sound-list DVCs

DOC-HIGH (GLOBAL_SOUNDS report, not re-decompiled this session — **UNCHECKED-this-session**): 101 sound
keys stored as int VocClass indices in the RulesClass singleton (`DAT_008871e0`), e.g. `SellSound@0x6A4`,
`BaseUnderAttackSound@0x184`, `GUIMainButtonSound@0x188`, `ChronoInSound@0x218`; plus DVC lists
`CreditTicks@0x6CC`, `LightningSounds@0x734`, `IceCrackSounds@0x648`. `-1` = no sound. `ReadAudioVisual`
@ `0x006691e0` populates these. `DeploySound` is dead as a global (not read by ReadAudioVisual; per-type only).

---

## (3) Active vs legacy/dormant TS split

| Mechanism | Status | Reachable in stock YR? | Notes |
|---|---|---|---|
| Priority→ColorScheme table, PriorityToColorScheme, g_ColorSchemeArray, InitColor, ComputeRemap, FindColorSchemeIndex | **LIVE** | Yes — every house creation | Observable as faction tint + radar dots; callers `Create_Houses`, `MPlayer_Defeated`, `TriggerAction`. |
| 9th priority entry (8→scheme 5) | **LIVE** | Yes — observer/spectator slots | Scheme 5 = WHITE (also the forced fallback). |
| VocClass system (FindByName/PlayAtPos/ReadINI/CalcVolumeAndPan); Control/Type/Priority tables | **LIVE** | Yes — ~75 PlayAtPos callers | Weapon fire, locomotion, UI, radar, credits, lightning. |
| SHROUD type gating (0x800) in CalcVolumeAndPan | **LIVE** | Yes — cell-visibility, NOT FogOfWar | Reads `cell+300 & 0x18` (revealed/visible bits) — shroud audibility, present in stock YR; independent of the TS FogOfWar darkening (default off). |
| UNSHROUD type (0x400) | **LIVE** | Yes | Paired/mutually-exclusive with SHROUD. |
| NOISE_SHY (0x100) / GUN_SHY (0x200) | **UNCHECKED** | not traced this session | Table exists and parses; whether the suppression branch is reachable in stock YR was not traced → default **DRIFT/UNCHECKED**, do not assume dormant. |
| DeploySound as a global `[AudioVisual]` key | **DEAD as a global** | — | Not read by ReadAudioVisual; only per-type. |
| tunnel/subterranean | n/a | — | No intersection with this family. |

---

## (4) Compare vs current Rust — table-by-table

### 4.1 Palette / Remap

**4.1a — `PRIORITY_TO_SCHEME_INDEX` (the 9-byte table).**
`src/rules/color_scheme.rs:31` — `[3, 11, 21, 29, 13, 25, 17, 15, 5]`. **MATCH** the gamemd bytes
`03 0b 15 1d 0d 19 11 0f 05` (`read_memory 0x0083ed14`). This is the one table that is gamemd-exact.

**4.1b — `scheme_index_for_priority` out-of-range branch — DRIFT.**
`src/rules/color_scheme.rs:39-47`: for `priority` outside `0..9` and ≠ `-2`, returns `priority.max(0) as usize`.
gamemd `PriorityToColorScheme` (`decompile_function 0x0069A310`) returns `p` **unchanged** for `p≥9`
(no clamp). The two differ for any negative priority other than `-2`: Rust maps a negative `priority`
(e.g. `-1`, `-3`) to `0` (→ index 3 if it then re-entered the table — but here it returns `0` as the
scheme index directly), whereas gamemd, treating `p` as `uint`, takes a huge unsigned value `≥9` and
returns it unchanged. **Mismatch on every negative-non-`-2` input.** Also semantically: Rust uses `i32`,
gamemd uses `uint` — the `-2` sentinel only works because Rust special-cases it; gamemd compares
`0xFFFFFFFE`. Boundary `p=9` → gamemd returns 9; Rust returns `9.max(0)=9` (coincidentally equal here),
but `p=-1` → gamemd returns `0xFFFFFFFF`, Rust returns `0`. **DRIFT.**

**4.1c — House color RGB source — STRUCTURAL DRIFT (the biggest one).**
`src/rules/house_colors.rs` synthesizes house ramps from **invented** base RGB triplets
(`SCHEME_BASES`, line 61-71, e.g. Gold `(200,180,60)`, DarkBlue `(40,60,200)`) and a const-fn brightness
gradient `generate_ramp` (line 156-188, `brightness_100 = 140 - (i*110/15)`). gamemd does **none** of this:
- The 16-shade ramp is the ColorScheme object's embedded 256-entry RGB remap palette (`ColorScheme+0x04`),
  loaded from the scheme data, **not** a brightness gradient over a base color (DOC-HIGH, radar-dot §5).
- The house's single team-color RGB (`House+0x56F9..0x56FB`) is **extracted from the scheme's converted
  pixel** via `InitColor` (`scheme+0x30C`/`+0x330`, DD shift/loss), **not** `SCHEME_BASES[i]`.
- The "bright"/highlight RGB (`House+0x56FC..0x56FE`) comes from `ComputeRemap`'s
  `sqrt`-normalization, which the Rust port has no equivalent of at all.

Every owned object's tint, every radar dot, every selection/target-line color in gamemd traces back to
these scheme-derived bytes. The Rust port produces visually *plausible* but **not bit-identical** colors.
**DRIFT** (unproven equivalence; the formulas are entirely different mechanisms). Cited: `house_colors.rs:61`,
`:156`; gamemd `0x50B840`, `0x50BA00` (DOC-HIGH) + radar-dot report §5.

**4.1d — Scheme count/indexing — DRIFT.**
`src/rules/house_colors.rs:42` `SCHEME_COUNT=9` with hand-picked names `gold,darkblue,darkred,green,
orange,purple,lightblue,brown,grey` (line 48-58). gamemd has a **runtime DynamicVector** of ColorScheme
objects (`g_ColorSchemeArray`), one per `[Colors]` entry, **doubled** (the `[Colors]` list maps to two
runtime schemes each — see `color_scheme.rs:88` which already documents the doubling for the loading bar).
The priority table indexes 3,11,21,29,... into that doubled runtime array, **not** into a 9-name table.
`house_colors.rs` cannot reach scheme index 11/21/29 at all — its `house_color_ramp` (line 95-102)
clamps anything `≥9` to Gold. So `house_colors.rs` is a **parallel, incompatible indexing scheme** that
does not consume the priority table's output. **DRIFT** (wrong indexing domain). Cited: `house_colors.rs:42`,
`:95`; gamemd `0x0069A310` + radar-dot §3.

**4.1e — `color_index_for_name` fuzzy matching — DRIFT.**
`src/rules/house_colors.rs:108-149` resolves a color name with substring heuristics
(`contains("blue")`, etc.) and an unconditional Gold fallback. gamemd `Color=` resolution
(`FUN_00474A90`, DOC-HIGH) does an **exact** scheme-name strcmp against `ColorScheme+0x304` (plus the
`+0x310 != 1` key) and on no-match returns the **previous/default scheme index**, not Gold-0. Different
match semantics (substring vs exact), different fallback (Gold vs previous). **DRIFT.** Cited:
`house_colors.rs:108`; gamemd `0x00474A90` (DOC-HIGH, radar-dot §4).

**4.1f — `house_colors.rs` is consumed by render, not the loading bar.**
Note the *intra-family* duplication: `src/rules/color_scheme.rs` already has the correct gamemd HSV→RGB
path + priority table + doubling for the loading bar, while `house_colors.rs` re-implements an
*incompatible* color story for unit/radar rendering. These two files disagree on what a "scheme" is.
See Retire List §7.

**4.1g — `Palette::with_house_colors` remap range — MATCH (structure), but feed is wrong.**
`src/assets/pal_file.rs:154-159` substitutes palette indices `16..32` with a 16-color ramp — this is the
correct gamemd remap-band range (DOC-HIGH; VXL/blitter reports confirm indices 16–31 are the house band).
The *mechanism* (16-entry substitution at 16..31) is right; the *data* fed in (synthesized ramp from
`house_colors.rs`) is the §4.1c DRIFT. The GPU mirror `src/render/palette_textures.rs:263-288`
(`build_house_ramp_bytes`) inherits the same wrong feed via `house_colors::house_color_ramp` (line 278).

**4.1h — 6-bit→8-bit palette scale — two formulas, both present.**
`src/assets/pal_file.rs:182` `from_bytes` uses `(v*255+31)/63`; `:187` `from_bytes_gamemd_ui` uses `v<<2`.
gamemd's UI/loading path uses `<<2` (the `from_bytes_gamemd_ui` variant has a passing boundary test,
`63→252`). The default `from_bytes` rounding `(v*255+31)/63` (`63→255`) is **not** the gamemd UI formula;
whether the *in-game* (non-UI) palette path uses `<<2` or `*255/63` was **not verified this session** →
**UNCHECKED**; if any in-game render path uses `from_bytes`, that is a DRIFT (max component 255 vs 252).
Cited: `pal_file.rs:182`, `:187`.

### 4.2 Sound

**4.2a — Name→index resolution — MISSING (structural DRIFT).**
gamemd `FindByName` (`decompile_function 0x007514d0`) assigns every parsed VocClass a **stable 0-based
index** = first-match position in `g_VocArray`, with `"Invalid Voc"` for null-named entries and **lowest-
index tie-break**. The Rust `SoundRegistry` (`src/rules/sound_ini.rs:46-49`) is a **`HashMap<String,
SoundEntry>` keyed by uppercase ID** — there is **no index**, no array order, no tie-break, no `"Invalid
Voc"` sentinel. Consumers that gamemd drives by VocClass index (RulesClass 101 sound fields, CreditTicks/
LightningSounds DVCs, ReadSoundList) cannot be reproduced from a name-keyed HashMap without re-deriving an
order. **DRIFT** (missing the entire index domain). Cited: `sound_ini.rs:46`; gamemd `0x007514d0`,
`0x00525430`.

**4.2b — Case sensitivity — DRIFT.**
gamemd name comparisons are **case-sensitive strcmp** (`FUN_007c8d20`, in `FindByName` `0x007514d0`).
The Rust registry lowercases/uppercases keys (`sound_ini.rs:128` `to_ascii_uppercase`, `:166` lookup
uppercases). gamemd treats `GISelect` and `giselect` as **different** sounds (the second would not match
and would keep the prior field value / return -1). **DRIFT** on any case-mismatched INI reference.
Cited: `sound_ini.rs:128`, `:166`; gamemd `0x007514d0`.

**4.2c — Control / Type / Priority flag tables — MISSING (DRIFT).**
The Rust `SoundEntry` (`sound_ini.rs:28-43`) stores only `id, sounds, volume, priority, range, min_volume`
— and `priority` is parsed as a raw `get_i32("Priority").unwrap_or(1)` (`sound_ini.rs:117`). gamemd
parses `Priority=` as a **string** through the priority **name table** (`LOWEST..CRITICAL`→0..4, unknown→2)
(`read_memory 0x00816018`), and parses `Control=`/type via the bit tables (`0x008160c0`/`0x00816048`).
The Rust port:
- has **no Control flags** (LOOP/RANDOM/ALL/PREDELAY/INTERRUPT/ATTACK/DECAY/AMBIENT) — so RANDOM vs
  sequential selection, looping, predelay, interrupt are all unmodeled;
- has **no Type flags** (SHROUD/UNSHROUD/GLOBAL/LOCAL/SCREEN/PLAYER/VIOLENT/MOVEMENT/QUIET/LOUD/AMBIENT/
  GUN_SHY/NOISE_SHY) — so the §5 CalcVolumeAndPan flag-driven branches cannot be selected at all;
- parses Priority as an **integer**, defaulting to `1` (not `2`/NORMAL), and never maps the name table.
**DRIFT** (missing tables; wrong Priority default `1` vs `2`; integer vs name parse). Cited:
`sound_ini.rs:28`, `:117`; gamemd `0x00816018`, `0x008160c0`, `0x00816048`.

**4.2d — `Sounds=` tokenization — partial DRIFT.**
`sound_ini.rs:96-107` splits on whitespace **and** commas, strips leading `$`/`#`, drops `;`-comment
tokens. gamemd `ReadINI` uses `strtok` with the Voc delimiter set (`@0x00846570`, DOC-HIGH) and
`AddSample` per token. The delimiter set was **not byte-dumped this session** (UNCHECKED) — if it is not
exactly "whitespace + comma", the token split differs. The `$`/`#` strip is plausibly correct (legacy
Westwood markers) but unverified against `AddSample` this session. **UNCHECKED → DRIFT** until the
delimiter bytes at `0x00846570` are read. Cited: `sound_ini.rs:96`; gamemd delim `0x00846570`.

**4.2e — Default Volume/Range/MinVolume — partial DRIFT.**
`sound_ini.rs:60-74` reads `[Defaults]` Volume(100)/Range(10)/MinVolume(0). gamemd defaults come from
globals `_DAT_008464b4` (Volume), `_DAT_008464c0` (Range), `_DAT_008464b8` (MinVolume), `DAT_008464c4`
(Limit) (DOC-HIGH, ReadINI contract) — these were **not read this session** (UNCHECKED), so the literal
fallbacks `100/10/0` are unverified-equal. Volume/MinVolume are also **doubles** in gamemd (`ReadINI`),
clamped/quantized to `u8` in Rust (`sound_ini.rs:64,74`) — precision loss. **DRIFT** (float→int) +
UNCHECKED default values. Cited: `sound_ini.rs:60`; gamemd globals `0x008464b4/c0/b8/c4`.

**4.2f — `RANGE_MULTIPLIER` — MATCH.**
`src/audio/sfx.rs:35` `RANGE_MULTIPLIER=60.0`; gamemd `CalcVolumeAndPan` (`decompile 0x00750ac0`)
`audioEventPtr = iVar6 * 0x3c` = ×60. **MATCH** on the multiplier value.

**4.2g — Spatial volume formula — DRIFT (several sub-points).**
`src/audio/sfx.rs:55-97 calc_spatial_volume` vs gamemd `CalcVolumeAndPan` (`decompile 0x00750ac0`):
- **Center reference:** Rust uses `camera + viewport*0.5` then `abs(sound - center)` (lines 65-71).
  gamemd uses `CoordsToClient2` then `abs(x), abs(y)` of the **client-space** result — a different
  projection (isometric `CoordsToClient2`, not a linear camera offset). **DRIFT.**
- **Half-viewport subtraction is gated by LOCAL (0x40):** gamemd subtracts the half-viewport
  `(width*k, height*k)` **only when type flag `0x40` (LOCAL) is CLEAR** (`if ((uVar7 & 0x40)==0)`).
  Rust subtracts it **unconditionally** (lines 73-74). Since Rust has no type flags (§4.2c), it can never
  honor LOCAL. **DRIFT.**
- **Half-viewport scale:** gamemd uses `g_RadarViewportWidth * _DAT_007e5168` (a global factor `k`, not
  necessarily 0.5). Rust hard-codes `*0.5` (lines 73-74). The factor `_DAT_007e5168` was **not read this
  session** (UNCHECKED) — if `k≠0.5` this is a DRIFT. **UNCHECKED→DRIFT.**
- **y*2:** both double Y (Rust `:77`, gamemd `fVar2 = fVar2 + fVar2`). **MATCH** on that step.
- **distance = max(x,y):** both (`:80`, gamemd `if (fVar2 < x) fVar2 = x`). **MATCH.**
- **Falloff:** both `(range - dist)/range` (`:88`, gamemd `(fVar1 - fVar2)/fVar1`). **MATCH** in shape.
  Corrected ordering (verified `decompile 0x00750ac0`): gamemd **first doubles y** (`fVar2 = fVar2 + fVar2`),
  THEN guards `if (x < range_px && y_doubled < range_px && range_px > 0)`, THEN takes `max(x, y_doubled)`,
  THEN `(range_px - max)/range_px`. So the guard uses the *already-doubled* y (the doc's earlier worry
  about an "unscaled y" path divergence does not apply — both guard and max use the doubled value). The
  residual divergence is real but narrower: gamemd's three-way AND guard `(x<range && y<range)` differs
  from Rust's single `max(x,y) >= range → 0` (`:84`) — they agree because `max(x,y)<range ⟺ x<range AND
  y<range`, so this specific guard **is** algebraically equivalent. The remaining DRIFTs are the projection
  (CoordsToClient2 vs camera offset), the LOCAL/GLOBAL/SHROUD gates, the hard-coded 0.5/0.05, and the
  missing pan — not this guard. **Guard step: now MATCH; section overall still DRIFT for the other points.**
- **MinVolume floor gated by GLOBAL (0x10):** gamemd applies the MinVolume floor **only if type flag
  `0x10` (GLOBAL)** is set. Rust applies it **unconditionally** (lines 90-93). **DRIFT.**
- **Inaudibility threshold:** gamemd returns the silent sentinel `FLOAT_007e1748` (NOT literal 0) when
  `vol < _DAT_007e8ae8` (the test in the binary is `if (_DAT_007e8ae8 <= vol) {...return vol}` else return
  `FLOAT_007e1748`). Rust uses a hard-coded `MIN_VOLUME_CUTOFF = 0.05` (`:43`, `:96`) commented
  "approximately 5%". Both `_DAT_007e8ae8` and `FLOAT_007e1748` were **not byte-read this session**
  (UNCHECKED values) — "approximately" is a self-declared DRIFT. **DRIFT.**
- **SHROUD gate (0x800) entirely missing:** gamemd silences a sound whose source cell == listener cell OR
  whose cell is not revealed/visible (`cell+300 & 0x18 == 0`) when type flag `0x800` set. Rust has **no
  shroud audibility gate at all**. **DRIFT** (enemy sounds in unexplored cells would play). 
- **Pan:** gamemd computes a pan value (`*out_pan`, ftol'd from signed screen-X before abs) and returns
  it; Rust `calc_spatial_volume` returns **only a scalar volume** — **no pan output**. The whole stereo
  pan channel is unmodeled. **DRIFT.**
- **float vs fixed:** both use `f32`/float here (gamemd is genuinely float in the mixer; this is
  presentation-layer, so float is acceptable per CLAUDE.md), but the *values* still must match.

**4.2h — PlayAtPos OOB sentinel — informational MATCH-able.**
gamemd `PlayAtPos` (`decompile 0x00750920`) silently no-ops on disabled sound system or OOB index
(no crash). The Rust SFX dispatch (`sfx.rs:168` `play_sound` returns `false` on unresolved id) matches the
**silent no-op** contract in spirit, but resolves by **name/HashMap**, not by validated **index<count**
(§4.2a). The "return value reflects loop-handle path" nuance is unmodeled (no loop handles in Rust).

**4.2i — Random sample selection — DRIFT.**
gamemd RANDOM is a **Control flag** that, when set, picks a sample via the engine RNG; without RANDOM the
samples play in sequence/first. Rust always picks `random_counter % len` (`sfx.rs:179`, a non-RNG counter)
for **every** multi-sample entry, ignoring the Control flag and not consuming the deterministic RNG.
**DRIFT** (selection policy + RNG source). Cited: `sfx.rs:179`; gamemd Control table `0x008160c0`.

---

## (5) Gamemd-native behavior contract (the spec the substrate must reproduce)

### 5.1 PriorityToColorScheme  (`0x0069A310`, VERIFIED-THIS-SESSION)
- Input `p` is **`uint`**. Output `uint`.
- `p == 0xFFFFFFFE` → return `DAT_0083ed1c` (`0xFFFFFFFF` in stock).
- `0 ≤ p ≤ 8` → return `(int)(signed char)table[p]` from `{3,11,21,29,13,25,17,15,5}`. (Signed-byte cast —
  stock bytes are all positive so identical, but a `>0x7F` byte would sign-extend; preserve the cast.)
- `p ≥ 9` (and ≠ `0xFFFFFFFE`) → return `p` **unchanged** (no clamp, no table read). Boundary `p=9 → 9`.

### 5.2 InitColor (index→base RGB)  (`0x50B840`, VERIFIED-THIS-SESSION `decompile_function 0x50B840`)
- `House+0x16054 < 0` → force `5`.
- `scheme = g_ColorSchemeArray[idx]`. If `scheme == 0` → debug "Forcing House %s [%s] to color WHITE",
  set `idx=5`, use `array[5]`.
- Read converted pixel at `(scheme+0x30C → +0x174) + (scheme+0x330)*stride`, stride `1` if
  `scheme+0x30C+4 == 1` else `2`. Extract R,G,B by `(raw >> DD_xShift) << DD_xLoss` per channel → 3 bytes
  `House+0x56F9/56FA/56FB`. Boundary: idx≥count is undefined unless the caller pre-mapped via
  PriorityToColorScheme; gamemd relies on the forced-5 path only when the **ptr is 0**, not for OOB idx —
  the Rust replacement must mirror PriorityToColorScheme feeding InitColor.

### 5.3 ComputeRemap (base→bright RGB)  (`0x50BA00`, VERIFIED-THIS-SESSION `decompile_function 0x50BA00`)
- `len = Sqrt_Approx(R² + G² + B²)`. If `len == 0` → bright = all three set to `_DAT_007eaa50` (the high-cap
  const, i.e. `(255,255,255)` if that const is 255.0).
- Else per channel `v = channel*_DAT_007e5f78/len`; if `v > _DAT_007eaa50` → `_DAT_007eaa50`; if
  `v < _DAT_007eaa48` (low cutoff) → 0. After clamping, a trailing `Sqrt_Approx(R²+G²+B²)` is computed (its
  result discarded into x87 FPU state) and three `ftol` pop the clamped R/G/B values to bytes
  `House+0x56FC/56FD/56FE`. Boundary: a single nonzero channel normalizes to the cap on its axis.
- **Correction vs prior DOC-HIGH text:** the channel multiplier (`_DAT_007e5f78`) and the high-cap
  (`_DAT_007eaa50`) are **named globals, NOT a hardcoded literal 255** — they were not byte-read this
  session (presumed 255.0; gated follow-up read needed before any exact bright-RGB test asserts numbers).

### 5.4 Voc name→index resolution  (`0x007514d0` VERIFIED; `0x00751520`/`0x007515c0` DOC-HIGH)
- `FindByName(name)`: `name==0 → -1`; linear scan `0..count-1`; per entry compare `name` vs
  (`entry+0 == 0 ? "Invalid Voc" : entry+0x6c`) by **case-sensitive strcmp**; return **first match
  (lowest index)** else -1.
- Caller keep-previous-on-fail (ReadAudioVisual, DOC-HIGH): empty string or `-1` → field keeps prior
  value, NOT set to -1.
- `FindPtrByName`: input == `<none>` sentinel → 0 (NULL); else scan → VocClass* or 0.
- `FindIndexByPtr`: scan, match on stored pointer equality, return index or -1.

### 5.5 ReadSoundList (DVC of indices)  (`0x00525430`, VERIFIED-THIS-SESSION)
- `ReadString` then `strtok` by delim (single set @ `0x00817f70`). Per nonempty token:
  `FindPtrByName → if NULL skip silently; else FindIndexByPtr → append index to DVC`. Empty/missing key →
  empty DVC (count 0). Order preserved = INI order. **CreditTicks contract:** needs count ≥ 2 for credit
  ticks to play (`[0]`=up, `[1]`=down).

### 5.6 Control / Type / Priority flag parsing  (tables VERIFIED-THIS-SESSION; parsers DOC-HIGH)
- Tokens parsed **left-to-right**; each matched by **case-sensitive strcmp** against its table to a NULL-
  name terminator.
- **Control:** `flags |= bit` (no exclusion). Unknown token → OR 0 (silently ignored, no error).
  Bits: `ALL=0x04, LOOP=0x01, RANDOM=0x02, PREDELAY=0x08, INTERRUPT=0x10, ATTACK=0x20, DECAY=0x40,
  AMBIENT=0x80`.
- **Type:** apply exclusion first (`0x60` group SCREEN/LOCAL; `0xc00` group UNSHROUD/SHROUD — last-wins
  within group), then `flags |= bit`. Bits per §2.3.
- **Priority:** unknown → default value **2** (NORMAL). LOWEST=0..CRITICAL=4.

### 5.7 PlayAtPos  (`0x00750920`, VERIFIED-THIS-SESSION)
- If `DAT_008464ac == 0` (sound system disabled) → return 0, no sound.
- Resolve VocClass only if `-1 < index < count`; index `-1` or `≥count` → no sound, **silent no-op, no
  crash**. This is the canonical "no sound" sentinel a Rust port must reproduce (silent, not panic).

### 5.8 CalcVolumeAndPan  (`0x00750ac0`, VERIFIED-THIS-SESSION)
- `range_px = GetRange() * 0x3C (=60)`.
- If type flag `0x800` (SHROUD): per coord cell via `(c + (c>>31 & 0xff)) >> 8` (sign-correct toward
  zero); if source cell == listener cell (`DAT_00b1d310/0312`) → silent threshold `FLOAT_007e1748`; if
  `MapCell+300 & 0x18 == 0` → silent.
- Screen coords via `CoordsToClient2`; take `abs(x), abs(y)`.
- If type flag `0x40` (LOCAL) **CLEAR**: subtract half-viewport `(g_RadarViewportWidth*_DAT_007e5168,
  g_RadarViewportHeight*_DAT_007e5168)`; clamp each `≥0`.
- `y *= 2` (isometric). `distance = max(x,y)`. If `x<range_px AND y<range_px AND range_px>0`:
  `vol = (range_px - distance)/range_px`; else 0.
- If type flag `0x10` (GLOBAL) and `vol < MinVolume` → `vol = MinVolume`.
- If `_DAT_007e8ae8 <= vol` → write `*out_pan` (ftol'd) and return `vol`. Else return the inaudible
  sentinel **`FLOAT_007e1748`** (NOT a literal 0 — this is also the same value used for the SHROUD/source-
  cell silent returns and as the per-axis floor; it is the engine's "silent" float constant, value UNREAD
  this session — presumed 0.0 but a gated follow-up read is needed to confirm it is exactly 0.0).
- Units: Range INI is cells; ×60 = the px scale used directly against screen-space pixel deltas. Pan
  derives from the **signed** screen-X before abs (abs is for distance only).

---

## (6) Designed Rust-native substrate boundary

**Verdict:** split this family into **two pure, deterministic substrate services** — one for remap/palette
data (consumed by render/), one for the Voc table + flag parsing (consumed by audio/, and the deterministic
index part by sim-adjacent rules consumers). The presentation math (GPU shader remap, rodio mixing) stays
in render/ and audio/ and is **not** part of the substrate. Crucially: **sim/ never depends on render/ or
audio/**, so the substrate lives in `rules/` (parsed/embedded data + deterministic name→index), and the
render/audio layers consume it.

### 6.1 `rules::tables::color_scheme_substrate` (NEW, render-facing data)

Owns the gamemd color-scheme pipeline as pure data.

```text
src/rules/tables/color_scheme_substrate.rs
```

API (signatures, Rust-native, no C++ class port):

```rust
/// The 9-byte priority→scheme table, embedded verbatim from the gamemd dump.
pub const PRIORITY_TO_SCHEME: [u8; 9] = [3, 11, 21, 29, 13, 25, 17, 15, 5];
pub const PRIORITY_DEFAULT_SCHEME: u32 = 0xFFFF_FFFF; // DAT_0083ed1c
pub const PRIORITY_RANDOM_SENTINEL: u32 = 0xFFFF_FFFE;

/// Exact PriorityToColorScheme: uint in, uint out. p>=9 (≠sentinel) returns p.
pub fn priority_to_scheme(p: u32) -> u32;

/// One loaded color scheme: name + the 256-entry RGB remap palette + the
/// converted-pixel remap index. Built from [Colors]+scheme data, NOT synthesized.
pub struct ColorScheme { pub name: String, pub remap_palette: [[u8;3];256], pub remap_index: u16, pub match_key: i32 }

/// The runtime doubled scheme list (DynamicVector analogue): Vec<ColorScheme>.
pub struct ColorSchemeTable { schemes: Vec<ColorScheme> }
impl ColorSchemeTable {
    pub fn from_ini(colors: &IniFile, /* scheme palette source */) -> Self; // builds the doubled list
    pub fn get(&self, idx: usize) -> Option<&ColorScheme>;
    pub fn find_by_name(&self, name: &str) -> Option<usize>; // exact strcmp + match_key, previous/default on miss
    pub fn len(&self) -> usize;
}

/// InitColor: idx → base RGB (House+0x56F9..56FB), forced-5 on null scheme.
pub fn extract_base_rgb(table: &ColorSchemeTable, idx: i32, dd: &DdFormat) -> [u8;3];
/// ComputeRemap: base RGB → bright RGB (House+0x56FC..56FE).
pub fn compute_bright_rgb(base: [u8;3], low_cutoff: u8) -> [u8;3];
```

- **Where it lives:** `rules/` — pure data, depends only on `rules/ini_parser` and `assets/pal_file`
  (Color). No sim/render/audio deps inward; render/ consumes it.
- **Who owns the data:** `ColorSchemeTable` is owned by the rules bundle (built once at load). The
  per-house extracted `[u8;3]` base/bright RGB are owned by the house record (sim or game-state),
  computed once at house creation via `extract_base_rgb`/`compute_bright_rgb`.
- **Construction source:** the **9-byte priority table** is an **embedded const** from the gamemd dump
  (`read_memory 0x0083ed14`) — it is not in any INI. The **scheme palettes** are loaded scheme data
  (the doubled `[Colors]` list); the `DdFormat` shift/loss come from the active surface descriptor
  (render-supplied at extract time — passed in, not depended on).
- **Determinism:** all integer math (the `sqrt` in ComputeRemap must be a fixed/integer isqrt to stay
  lockstep-safe if house color ever feeds a hash; it feeds render only today, but keep it integer).

### 6.2 `rules::tables::voc_substrate` (NEW, audio-facing + deterministic index)

Owns the VocClass table, name→index resolution, and the three flag tables as pure data.

```text
src/rules/tables/voc_substrate.rs
```

API:

```rust
bitflags! { pub struct VocControl: u32 { /* ALL=0x04, LOOP=0x01, RANDOM=0x02, PREDELAY=0x08, INTERRUPT=0x10, ATTACK=0x20, DECAY=0x40, AMBIENT=0x80 */ } }
bitflags! { pub struct VocType: u32   { /* per §2.3 */ } }
#[repr(u8)] pub enum VocPriority { Lowest=0, Low=1, Normal=2, High=3, Critical=4 }

pub struct VocEntry {
    pub name: String, pub samples: Vec<String>,
    pub volume: f64, pub min_volume: f64, pub range: i32, pub limit: i32,
    pub priority: VocPriority, pub control: VocControl, pub ty: VocType,
    pub vshift: i32, pub attack: i32, pub decay: i32, pub delay: (i32,i32), pub fshift: (i32,i32),
}

/// The ordered VocClass table — array order IS the index domain.
pub struct VocTable { entries: Vec<VocEntry> } // Vec, not HashMap — index is load-bearing
impl VocTable {
    pub fn from_ini(soundmd: &IniFile, sound: &IniFile) -> Self; // YR-first, preserves [SoundList] order
    /// FindByName: case-sensitive, first-match (lowest index), "Invalid Voc" for empty-named, else -1.
    pub fn find_by_name(&self, name: &str) -> i32;
    pub fn get(&self, index: usize) -> Option<&VocEntry>;
    pub fn read_sound_list(&self, raw: &str) -> Vec<i32>; // strtok delim, skip-NULL, indices, INI order
    pub fn len(&self) -> usize;
}

// Pure parsers (left-to-right, case-sensitive, NULL-term tables embedded as consts):
pub fn parse_control(tokens: &str) -> VocControl; // OR, unknown→noop
pub fn parse_type(tokens: &str) -> VocType;       // exclusion 0x60/0xc00 last-wins, then OR
pub fn parse_priority(tok: &str) -> VocPriority;  // unknown → Normal(2)
```

- **Where it lives:** `rules/` — depends only on `rules/ini_parser`. audio/ consumes `VocTable` +
  `VocEntry`; the deterministic `find_by_name`/`read_sound_list` index outputs are also consumed by the
  rules/RulesClass sound-field resolver (the 101 fields + 3 DVCs).
- **Who owns the data:** `VocTable` owned by the rules bundle (built once). RulesClass sound-index fields
  are `i32` resolved through `find_by_name` at rules load (keep-previous-on-fail per §5.4).
- **Construction source:** Voc entries from `soundmd.ini`/`sound.ini` (INI-parsed, YR-first); the three
  flag/priority tables are **embedded consts** from the gamemd dump (`0x008160c0`, `0x00816048`,
  `0x00816018`). The Voc/ReadSoundList delimiters (`0x00846570`, `0x00817f70`) embedded as consts once
  byte-read.
- **Determinism:** `Vec` order = the gamemd index domain; `find_by_name` returns the same index gamemd
  would; `read_sound_list` preserves INI order and skips unresolved tokens identically.

### 6.3 What stays in render/ and audio/ (NOT substrate)

- `render::palette_textures` continues to upload the remap palette to the GPU, but **fed from
  `ColorScheme.remap_palette`** (§6.1) instead of synthesized ramps. The shader-time `16..32` substitution
  is correct already (§4.1g).
- `audio::sfx` continues to do rodio mixing/decode, but **reads `VocEntry`** (control/type/priority,
  volume as f64, range) and calls a `CalcVolumeAndPan` port that honors the type flags. The
  volume/pan formula (§5.8) is render/audio-presentation math; it lives in audio/ but must reproduce the
  contract exactly (including SHROUD gate, LOCAL gate, GLOBAL MinVolume, pan output, the
  `_DAT_007e8ae8`/`_DAT_007e5168`/`_DAT_007e1748` globals once read).

---

## (7) Retire list — ad hoc / duplicated / approximated Rust to replace

| Rust artifact | file:line | Why retire | Replaced by |
|---|---|---|---|
| `SCHEME_BASES` invented base RGB | `src/rules/house_colors.rs:61-71` | Synthesized, not gamemd scheme data | `ColorScheme.remap_palette` (§6.1) |
| `generate_ramp` brightness gradient | `src/rules/house_colors.rs:156-188` | Wrong mechanism (brightness curve vs scheme palette + InitColor/ComputeRemap) | `extract_base_rgb` + `compute_bright_rgb` + scheme palette |
| `SCHEME_NAMES` 9-name table | `src/rules/house_colors.rs:48-58` | Wrong indexing domain (9 names vs doubled runtime DVC) | `ColorSchemeTable` |
| `house_color_ramp` (clamps idx≥9→Gold) | `src/rules/house_colors.rs:95-102` | Cannot reach scheme idx 11/21/29 | `ColorSchemeTable::get` |
| `color_index_for_name` substring fuzzy match | `src/rules/house_colors.rs:108-149` | Substring + Gold fallback vs exact strcmp + previous-default | `ColorSchemeTable::find_by_name` |
| `generate_ramp_from_base` (tiberium reuse) | `src/rules/house_colors.rs:192-194` | Same wrong gradient mechanism; reassess tiberium separately | (out-of-family; flag, don't silently retire) |
| `scheme_index_for_priority` `priority.max(0)` clamp | `src/rules/color_scheme.rs:39-47` | p≥9 must return p unchanged; `-2` must be `uint` 0xFFFFFFFE compare | `priority_to_scheme(u32)` (§6.1) |
| `SoundRegistry` HashMap (no index) | `src/rules/sound_ini.rs:46-178` | Missing the index domain; case-insensitive keys | `VocTable` (Vec, ordered) (§6.2) |
| `SoundEntry` (6 fields, int priority default 1) | `src/rules/sound_ini.rs:28-43`, priority parse `:117` | Missing control/type flags; wrong Priority default & integer parse | `VocEntry` + flag parsers (§6.2) |
| `Sounds=` ad hoc tokenizer | `src/rules/sound_ini.rs:96-107` | Delimiter set unverified vs gamemd strtok | `VocTable::from_ini` strtok (delim const) |
| `calc_spatial_volume` (no flags/pan/shroud, hard-coded 0.5 & 0.05) | `src/audio/sfx.rs:55-97` | Missing LOCAL/GLOBAL/SHROUD gates, pan, real thresholds | `CalcVolumeAndPan` port reading `VocEntry` (§6.3) |
| `MIN_VOLUME_CUTOFF=0.05` "approximately" | `src/audio/sfx.rs:43` | Self-declared approximation | `_DAT_007e8ae8` const (once read) |
| `random_counter % len` sample pick | `src/audio/sfx.rs:179,216,253,352` | Ignores RANDOM control flag; non-RNG counter | RANDOM-flag-gated RNG selection |

**Intra-family duplication called out:** `color_scheme.rs` (correct gamemd HSV/priority/doubling for the
loading bar) and `house_colors.rs` (incompatible synthesized scheme story for unit/radar render) are **two
disagreeing definitions of "color scheme"**. The substitute `ColorSchemeTable` (§6.1) should subsume both;
`color_scheme.rs`'s priority table + doubling logic is the keeper, `house_colors.rs`'s ramp synthesis is
the discard.

---

## (8) Migration slices + acceptance tests

Ordered, each independently shippable. **Pure-data-parity** slices (P) vs **stateful** slices (S) marked.
Acceptance tests are **exact-equality vs the gamemd dump**, boundary-inclusive.

**Slice 1 (P) — Priority table + PriorityToColorScheme exact port.**
Add `priority_to_scheme(u32)` + embedded const. Acceptance `priority_to_scheme_exact`: for every input in
`{0..=8, 9, 10, 0xFFFFFFFE, 0xFFFFFFFF, 0, 0x7FFFFFFF, 0x80000000}` assert exact equality with the gamemd
contract — `0..8 → {3,11,21,29,13,25,17,15,5}`, `9→9`, `10→10`, `0xFFFFFFFE→0xFFFFFFFF`,
`0xFFFFFFFF→0xFFFFFFFF` (≥9 passthrough), `0x80000000→0x80000000`. (Catches the §4.1b `max(0)` DRIFT.)

**Slice 2 (P) — Control/Type/Priority flag tables + parsers.**
Embed the three tables; add `parse_control/parse_type/parse_priority`. Acceptance:
- `control_bits_exact`: each name → its bit (`ALL=0x04,LOOP=0x01,RANDOM=0x02,PREDELAY=0x08,INTERRUPT=0x10,
  ATTACK=0x20,DECAY=0x40,AMBIENT=0x80`); `"FOO"`→no-op (0); `"loop"` (lowercase)→0 (case-sensitive miss).
- `type_exclusion`: `"SCREEN LOCAL"` → only LOCAL (0x40); `"UNSHROUD SHROUD"` → only SHROUD (0x800);
  `"SHROUD UNSHROUD"` → only UNSHROUD (0x400) (last-wins).
- `priority_table`: `LOWEST→0..CRITICAL→4`; `""`/`"BOGUS"`→2 (NORMAL).

**Slice 3 (P) — VocTable ordered name→index resolution.**
Replace `SoundRegistry` with `VocTable` (Vec). Acceptance:
- `find_by_name_first_match_lowest_index`: with two entries sharing a name, `find_by_name` returns the
  lower index.
- `find_by_name_case_sensitive`: `"GISelect"` matches, `"giselect"` → -1.
- `find_by_name_missing`: unknown → -1; `null`/empty input → -1.
- `invalid_voc_sentinel`: an empty-named entry compares against `"Invalid Voc"`.
- `index_stability`: parsing the retail `[SoundList]` yields the same index for a fixed sample across runs.

**Slice 4 (P) — ReadSoundList → Vec<index>.**
Add `read_sound_list`. Acceptance `read_sound_list_order_skip`: `"A,,B,UNKNOWN,C"` → `[idx(A), idx(B),
idx(C)]` (empty + unknown skipped, INI order preserved); empty key → `[]`; **CreditTicks count≥2** assertion
on the retail credit-tick list.

**Slice 5 (S) — RulesClass sound-index field resolution (keep-previous-on-fail).**
Resolve the 101 sound fields + 3 DVCs via `find_by_name`/`read_sound_list`. Acceptance
`keep_previous_on_fail`: a field with a missing/empty INI value retains its prior (default) value, NOT -1
(per §5.4). (Stateful: depends on field defaults; verify against GLOBAL_SOUNDS once that doc's 101 keys are
re-confirmed — currently UNCHECKED, so this slice is gated on re-verifying `0x006691e0`.)

**Slice 6 (P) — ColorSchemeTable from scheme data + doubling + find_by_name.**
Build the doubled runtime scheme list; exact `find_by_name` (strcmp + match_key, previous-default on miss).
Acceptance:
- `priority_indexes_doubled_table`: priority 0..7 → scheme idx 3,11,21,29,13,25,17,15 must resolve to a
  valid scheme in the doubled list (idx in range); priority 8 → 5 (white).
- `color_name_exact_match`: exact scheme name → its index; unknown name → previous/default (NOT Gold-0).

**Slice 7 (S) — InitColor + ComputeRemap house RGB extraction.**
Replace `generate_ramp`/`SCHEME_BASES`. Compute per-house base RGB (`extract_base_rgb`) + bright RGB
(`compute_bright_rgb`). Acceptance:
- `compute_bright_zero_len`: base `(0,0,0)` → bright `(255,255,255)`.
- `compute_bright_single_axis`: base `(K,0,0)` (K>0) → bright `(255,0,0)` (single nonzero normalizes to
  255 on its axis).
- `extract_base_forced_white`: null/`<0` idx → scheme 5 (white) path.
- `extract_base_exact_vs_dump`: for a fixed scheme idx and a known `DdFormat`, the 3 extracted bytes equal
  the gamemd `InitColor` output (requires a fixture pixel + DD shift/loss; cite the live read when added).
  (Stateful: depends on the live `DdFormat` globals — gated on reading `g_DD_*` at runtime.)

**Slice 8 (P→S) — CalcVolumeAndPan flag-aware port + pan output.**
Rework `audio::sfx` to read `VocEntry` and honor flags. Acceptance:
- `range_multiplier`: `range_px == range_cells * 60`.
- `local_flag_skips_viewport_subtraction`: LOCAL set → no half-viewport subtraction.
- `global_flag_minvolume_floor`: GLOBAL set + `vol<MinVolume` → `MinVolume`; GLOBAL clear → no floor.
- `shroud_gate_silences`: SHROUD set + source cell == listener cell → silent; SHROUD set + cell not
  `&0x18` → silent; SHROUD clear → audible.
- `pan_sign_from_signed_x`: pan derives from signed screen-X (left vs right gives opposite-sign pan).
- `inaudible_threshold`: `vol < _DAT_007e8ae8` → 0 (exact threshold once read).
  (The half-viewport factor `_DAT_007e5168`, threshold `_DAT_007e8ae8`, silent value `_DAT_007e1748` must
  be read live before this slice's exact tests can assert numbers — gated.)

**Slice 9 (P) — RANDOM-flag-gated, RNG-sourced sample selection.**
Acceptance `random_selection_gated`: RANDOM set → selection consumes the deterministic RNG; RANDOM clear →
first/sequential (no RNG consumption). (Determinism: must use the lockstep RNG, not `random_counter`.)

**Gating note:** Slices 5, 7 (exact bytes), 8 (exact numbers) depend on globals/docs **not read this
session** (`0x006691e0`, `g_DD_*`, `_DAT_007e5168/007e8ae8/007e1748`, `0x008464b4/c0/b8/c4`,
`0x00846570`, and — added by the 2026-06-04 adversarial pass — the ComputeRemap multiplier/high-cap
`_DAT_007e5f78`/`_DAT_007eaa50`). Those are a bounded follow-up Ghidra read pass before those tests can
assert exact values; the structural/pure slices (1–4, 6, 9) are unblocked now.

---

## Anchors & Evidence

| Address / symbol | Ghidra call cited (this session unless noted) | Doc cross-ref |
|---|---|---|
| Priority table `0x0083ed14` (+`0x0083ed1c`) | `read_memory 0x0083ed14` → `03 0b 15 1d 0d 19 11 0f 05 00 00 00 ff ff ff ff` | LOBBY_SESSION_HOUSE_CREATION; radar-dot §3 |
| `PriorityToColorScheme 0x0069A310` | `decompile_function 0x0069A310` | radar-dot §3 |
| Control table `0x008160c0` | `read_memory 0x008160c0` (72 bytes; name ptr order 148/140/138/12c/120/118/110/1b8) | AUDIO_CHANNEL §6 (doc had PREDELAY/INTERRUPT swapped) |
| Priority table `0x00816018` | `read_memory 0x00816018` → values 0,1,2,3,4 + NULL→2 | AUDIO_CHANNEL |
| Type table `0x00816048` | DOC-HIGH (bit values) | AUDIO_CHANNEL §7 |
| `FindByName 0x007514d0` | `decompile_function 0x007514d0` | decode contract |
| `ReadSoundList 0x00525430` | `decompile_function 0x00525430` | decode contract |
| `PlayAtPos 0x00750920` | `decompile_function 0x00750920` | decode contract |
| `CalcVolumeAndPan 0x00750ac0` | `decompile_function 0x00750ac0` | AUDIO_CHANNEL |
| `InitColor 0x50B840`, `ComputeRemap 0x50BA00` | DOC-HIGH | radar-dot §5; HOUSECLASS field map |
| `FindColorSchemeIndex 0x0068cab0` / `FUN_00474A90` | DOC-HIGH | radar-dot §4 |
| House RGB `+0x56F9..56FB` / bright `+0x56FC..56FE` | DOC-HIGH | radar-dot §1,§5; HOUSECLASS |
| `ReadINI 0x00750440`, `ReadAudioVisual 0x006691e0` | DOC-HIGH (UNCHECKED-this-session) | GLOBAL_SOUNDS |

---

## DRIFT Ledger

Severity = player-visibility × trigger-frequency (one-sentence trigger clause each).

| Rust file:line | Current | gamemd-correct | Severity (trigger frequency) |
|---|---|---|---|
| `src/rules/house_colors.rs:61-188` | Synthesized base RGB + brightness-gradient ramps | ColorScheme remap palette (`+0x04`) + InitColor pixel extract + ComputeRemap | **HIGH** — every owned object's tint + every radar dot, every frame of every match. |
| `src/rules/house_colors.rs:42-102` | 9-name scheme table, idx≥9→Gold | Doubled runtime DVC indexed by 3,11,21,29… | **HIGH** — the priority table's outputs (11/21/29) are unreachable, so player colors land on the wrong scheme; every match. |
| `src/rules/color_scheme.rs:39-47` | `priority.max(0)` clamp; i32 sentinel | `uint`; p≥9 returns p unchanged; `-2`==0xFFFFFFFE | **LOW** — only diverges on negative-non-`-2` priorities, which stock lobby slots don't produce; rare. |
| `src/audio/sfx.rs:55-97` | No LOCAL/GLOBAL/SHROUD gates, no pan, hard-coded 0.5 & 0.05 | Flag-gated viewport subtraction, MinVolume floor, shroud silence, pan output, real globals | **HIGH** — every positional SFX (~75 callers); enemy sounds audible through shroud + no stereo pan, audible every engagement. |
| `src/rules/sound_ini.rs:46-178` | HashMap, case-insensitive, no index | Ordered Vec, case-sensitive, stable 0-based index, "Invalid Voc" | **MEDIUM** — index-driven consumers (CreditTicks, Lightning, 101 Rules fields) can't be reproduced; fires on credit ticks/lightning/sell every match. |
| `src/rules/sound_ini.rs:28-43,:117` | No Control/Type flags; Priority int default 1 | Control/Type bit tables; Priority name table default 2 | **MEDIUM** — RANDOM/LOOP/INTERRUPT/priority eviction all unmodeled; affects sound variety + ducking every match. |
| `src/audio/sfx.rs:179` | `random_counter % len` (non-RNG) | RANDOM-flag-gated engine-RNG selection | **MEDIUM** — multi-sample sounds (unit voices, weapon reports) pick wrong sample + don't consume lockstep RNG; every multi-sample play. |
| `src/rules/sound_ini.rs:64,74` (float→u8) | Volume/MinVolume clamped to u8 | gamemd doubles | **LOW** — sub-unit volume precision loss, audible only on fine MinVolume floors; rare. |
| `src/assets/pal_file.rs:182` `from_bytes` `(v*255+31)/63` | Rounds 63→255 | UNCHECKED whether in-game path uses `<<2` (63→252) | **UNCHECKED→LOW** — possible 3-level brightness offset if any render path uses `from_bytes`; needs the in-game palette-conversion read. |
| `src/rules/sound_ini.rs:96-107` (tokenizer) | whitespace+comma split | strtok with delim set `0x00846570` (unread) | **UNCHECKED→LOW** — token split could differ if delim set ≠ ws+comma; needs the delim byte read. |
| NOISE_SHY (0x100)/GUN_SHY (0x200) active effect | n/a (unmodeled) | suppression branch reachability not traced | **UNCHECKED** — default DRIFT; trace `0x00750ac0`/mixer callers before assuming dormant. |

---

## Cross-family hook for synthesis

`House+0x56F9..0x56FB` (base RGB) and `+0x56FC..0x56FE` (bright RGB) are a **shared consumer surface**: the
same bytes feed this family's render remap **and** the radar/minimap object-dot packing
(`HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR_GHIDRA_REPORT.md` §1,§5 — `owner_dot_color` /
`src/render/minimap_helpers.rs`). The substitute `extract_base_rgb`/`compute_bright_rgb` (§6.1) must store
those bytes once on the house record and both the unit-remap GPU path and the radar-dot pack path must read
them — the synthesis stage must reconcile this family's house-color substrate with the radar-dot substrate
so they don't re-derive RGB by two different routes. Second hook: the DD shift/loss globals (`g_DD_*`) are
shared by InitColor extraction and radar-dot packing — one `DdFormat` source.

---

## Verification Log (adversarial re-check, 2026-06-04)

Adversarial pass: every load-bearing claim treated as wrong until the live binary proved it. Read-only
Ghidra MCP. Each claim → VERIFIED / WRONG (+correction) / UNVERIFIABLE with the call cited.

### Palette / Remap
- **Priority→ColorScheme table bytes `0x0083ed14`** → **VERIFIED**. `read_memory 0x0083ed14` len 16 =
  `03 0b 15 1d 0d 19 11 0f 05 00 00 00 ff ff ff ff` → `{3,11,21,29,13,25,17,15,5}` then `DAT_0083ed1c=0xFFFFFFFF`. Exact.
- **`PriorityToColorScheme 0x0069A310` semantics** → **VERIFIED**. `decompile_function 0x0069A310`:
  `if(p==0xfffffffe) return DAT_0083ed1c; if(p<9) p=(uint)(char)(&DAT_0083ed14)[p]; return p;` — signed-byte
  cast, `p≥9` passthrough (no clamp), `0xFFFFFFFE` sentinel. Confirms §4.1b/§5.1 and the `priority.max(0)` DRIFT.
- **PriorityToColorScheme callers (Create_Houses / MPlayer_Defeated / TriggerAction)** → **VERIFIED**.
  `get_function_callers 0x0069A310` = Create_Houses(0x687f10), MPlayer_Defeated(0x4fc0b0), TriggerAction__Execute(0x6dd8b0)
  (+3 others FUN_0048d1e0/0055e420/00642bb0). Confirms §3.
- **InitColor `0x50B840` (was DOC-HIGH → now VERIFIED)** → **VERIFIED with refinement**.
  `decompile_function 0x50B840`: forced-5 on `<0` idx and on null scheme ptr (debug "Forcing House %s [%s]
  to color WHITE"); pixel read `(scheme+0x30C→+0x174)+(scheme+0x330)*stride`, stride 1 if `*(scheme+0x30C)+4==1`
  else 2; channel extract `(raw>>g_DD_xShift)<<g_DD_xLoss` → `House+0x56F9/56FA/56FB`. Confirms the §4.1c HIGH
  DRIFT (RGB from scheme pixel, not synthesized base). DD globals are named `g_DD_R/G/B Shift/Loss`. Doc row updated.
- **ComputeRemap `0x50BA00` (was DOC-HIGH → now VERIFIED)** → **WRONG (corrected) on the constants**.
  `decompile_function 0x50BA00`: the channel multiplier is the named global **`_DAT_007e5f78`** and the high-cap
  is **`_DAT_007eaa50`** — NOT the literal `255` the doc asserted in §2.1/§5.3. Low-cutoff `_DAT_007eaa48` is
  correct. Also a trailing `Sqrt_Approx(R²+G²+B²)` + 3 ftol (x87 FPU-stack idiom) writes the clamped triplet
  to `House+0x56FC/56FD/56FE` — the doc omitted this. Output contract (normalized bright RGB) unchanged, but
  the literal-255 claim was wrong → corrected; `_DAT_007e5f78`/`_DAT_007eaa50` byte values added to the
  gated-globals follow-up list (any exact bright-RGB test must read them first).
- **`g_ColorSchemeArray` indexed `idx*4`** → **VERIFIED** (incidental). InitColor reads
  `*(g_ColorSchemeArray + idx*4)`, confirming the ptr-array-of-ColorScheme* layout in §2.1.

### Sound
- **`FindByName 0x007514d0`** → **VERIFIED**. `decompile_function 0x007514d0`: `name==0→-1`; linear
  `0..DAT_00b1d388`; per entry `*(int*)entry==0 ? "Invalid Voc"(0x846574) : GetName`; strcmp `FUN_007c8d20`;
  first-match (lowest index) return, else -1. Confirms §4.2a/§4.2b/§5.4 and the case-sensitive + missing-index DRIFTs.
- **`g_VocArray / count` (`DAT_00b1d37c` / `DAT_00b1d388`)** → **VERIFIED** (referenced in both
  `decompile 0x007514d0` and `decompile 0x00750920`: array base `DAT_00b1d37c + i*4`, count `DAT_00b1d388`).
- **`PlayAtPos 0x00750920`** → **VERIFIED**. `decompile_function 0x00750920`: `if(DAT_008464ac==0) return 0`;
  resolve only if `-1 < param_1 < DAT_00b1d388`, else iVar2=0; SetVolume/SetPan only if iVar2!=0; silent
  no-op on OOB. Confirms §4.2h/§5.7. (Note: SetVolume+SetPan calls confirm a pan output path exists in the
  live pipeline, reinforcing the §4.2g "no pan in Rust" DRIFT.)
- **`ReadSoundList 0x00525430`** → **VERIFIED**. `decompile_function 0x00525430`: ReadString (key/section
  delim `DAT_00889f64`) then `strtok(local_80, &DAT_00817f70)`; per nonempty token `FindPtrByName→if 0 skip;
  else FindIndexByPtr→append index` (with DVC capacity-grow gating); empty key → empty DVC; INI order
  preserved. Confirms §4.2a/§5.5. (Refinement: the strtok delim is `DAT_00817f70`; the *ReadString* delim is
  `DAT_00889f64` — distinct from the Voc `Sounds=` delim `0x00846570` still UNREAD for §4.2d.)
- **`CalcVolumeAndPan 0x00750ac0`** → **VERIFIED with one correction**. `decompile_function 0x00750ac0`
  confirms: ×0x3c range, SHROUD(0x800) gate with sign-correct cell `(c+(c>>31&0xff))>>8` + source==listener
  + `cell+300 & 0x18` test, CoordsToClient2+abs, LOCAL(0x40)-clear half-viewport subtract
  `g_RadarViewportWidth*_DAT_007e5168`, `y*=2`, three-way guard `(x<range && y<range && range>0)` then
  `max` then `(range-max)/range`, GLOBAL(0x10) MinVolume floor. **CORRECTION:** the inaudible/silent return
  is the sentinel **`FLOAT_007e1748`**, NOT literal 0 (test is `if(_DAT_007e8ae8 <= vol) return vol; else
  return FLOAT_007e1748`); `FLOAT_007e1748` is also the per-axis floor and the SHROUD silent value. Doc
  §5.8/§4.2g updated. Also corrected §4.2g falloff: gamemd doubles y *before* the guard, so the guard uses
  the doubled value and the Rust `max(x,y)>=range→0` guard step IS equivalent to gamemd's `(x<range &&
  y<range)` — that one sub-point downgrades from DRIFT to MATCH; the section stays DRIFT for projection,
  flag gates, hard-coded 0.5/0.05, and missing pan.
- **Control-flags table `0x008160c0` (bytes + name strings)** → **VERIFIED**. `read_memory 0x008160c0`
  len 72 (ptr order 148/140/138/12c/120/118/110/1b8, bits 4/1/2/8/16/32/64/128) + `read_memory 0x00816110`
  name strings: DECAY=0x40, ATTACK=0x20, INTERRUPT=0x10, PREDELAY=0x08, RANDOM=0x02, LOOP=0x01, ALL=0x04,
  AMBIENT(@0x8161b8)=0x80. The §2.3 PREDELAY=0x08 / INTERRUPT=0x10 correction (vs AUDIO_CHANNEL §6 swap) is
  confirmed bit-for-bit.
- **Type-flags table `0x00816048` (was DOC-HIGH → now VERIFIED)** → **VERIFIED**. `read_memory 0x00816048`
  len 120 + name strings `read_memory 0x0081617c`/`0x008161b8`: AMBIENT=0x1000, VIOLENT=0x01, MOVEMENT=0x02,
  QUIET=0x04, LOUD=0x08, GLOBAL=0x10, SCREEN=0x20, LOCAL=0x40, PLAYER=0x80, (empty)=0x00, GUN_SHY=0x200,
  NOISE_SHY=0x100, UNSHROUD=0x400, SHROUD=0x800, NULL term. Exclusion groups 0x60 / 0xc00 consistent. Doc row upgraded.
- **Priority table `0x00816018` (bytes + name strings)** → **VERIFIED**. `read_memory 0x00816018` len 48 =
  values 0,1,2,3,4 then NULL→2; name strings `read_memory 0x008161c0`/`0x008161dc`: LOWEST=0(@0x8161e0),
  LOW=1(@0x8161dc), NORMAL=2(@0x8161d4), HIGH=3(@0x8161cc), CRITICAL=4(@0x8161c0). Confirms §2.3/§5.6 and
  the §4.2c "Priority parsed int default 1 vs name-table default 2" DRIFT.

### UNVERIFIABLE this session (default DRIFT/UNCHECKED — unchanged, gated follow-up read)
- `FindColorSchemeIndex 0x0068cab0` / `FUN_00474A90` (§4.1e exact-vs-substring match semantics) — still DOC-HIGH; not decompiled this pass.
- `FindPtrByName 0x00751520`, `FindIndexByPtr 0x007515c0`, `GetName 0x00405170` — DOC-HIGH; not re-read.
- `ReadINI 0x00750440`, `ReadAudioVisual 0x006691e0` (the 101 RulesClass sound fields, slice 5) — UNCHECKED; not decompiled.
- Voc `Sounds=` delim bytes `0x00846570` (§4.2d) — UNREAD.
- Default globals `_DAT_008464b4/c0/b8/c4` (§4.2e), `_DAT_007e5168`/`_DAT_007e8ae8`/`FLOAT_007e1748`/`_DAT_007e5f78`/`_DAT_007eaa50`/`_DAT_007eaa48` values — UNREAD.
- `g_DD_*` runtime shift/loss values (slice 7 exact bytes) — UNREAD (static bytes presumed zero at rest).
- `pal_file.rs` in-game palette-conversion path `<<2` vs `*255/63` (§4.1h) — not traced.
- NOISE_SHY/GUN_SHY suppression-branch reachability (§3) — not traced.

### Net
No load-bearing claim was refuted in a way that overturns a stage-2 recommendation. Two precision
corrections (ComputeRemap constants are named globals not literal 255; CalcVolumeAndPan silent return is
`FLOAT_007e1748` not 0) and one sub-point downgrade (§4.2g falloff guard step is MATCH, not DRIFT) — none
change the substrate boundary, the retire list, or any DRIFT-ledger severity. The five "gated" globals/docs
are unchanged; the ComputeRemap multiplier/cap (`_DAT_007e5f78`/`_DAT_007eaa50`) are **added** to slice 7's
gated-read list (bright-RGB exact tests cannot assert numbers until those two are read).
