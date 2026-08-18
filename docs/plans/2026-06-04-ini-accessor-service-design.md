# Slice 1 — INI Typed-Accessor Service — DESIGN SPEC (brainstorm output)

**Status:** DESIGN SPEC (brainstorm output). NOT an approved implementation plan. Doc work only — no `src/` touched this run.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Slot:** Engine-substrate program, load-time data substrate (the `rules/` layer that feeds every other substrate). Master TODO: `docs/plans/2026-05-29-core-engine-substrate-todo.md`. Companion rhythm: `docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md`.
**Source of truth for the parse contract:** `docs/research/INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (Pass-2 verify-and-expand, 2026-06-04 — entire CCINIClass accessor family re-decompiled live; GREEN). All P-statement and address citations below resolve to that study unless re-verified inline here.

> **Scope of "Slice 1."** The study (§8) lays out S0–S7. **Slice 1 = study-S1 (introduce the service, additive + shadow) plus its blocking gate study-S0 (pin ReadDouble precision) plus the study-S2 corpus equivalence harness.** Consumer flips (study-S3..S7) are explicitly **out of scope** for this slice — they are later slices, each per-system. This spec defines the *service surface and its proof harness*; it does not flip a single consumer.

---

## 1. Goal

Introduce one cohesive, tested **typed INI accessor service** in `rules/` that reproduces the gamemd CCINIClass `ReadX` parse contract **bit-for-bit** on the resolved value, sitting on top of the existing raw `IniFile`/`IniSection` store. After Slice 1, the engine *has* a gamemd-faithful `read_int`/`read_bool`/`read_double`/`read_string`/tuple/enum/transform surface and a corpus-equivalence harness that proves where it agrees with — and where it deliberately corrects — the current ad-hoc accessors. No consumer is changed yet (additive/shadow).

**Why this is parity-load-bearing despite being load-time:** the accessor family's *only* observable output is the resolved value (study §1). A wrong hex/bool/percent/atoi parse silently shifts a unit stat / damage multiplier / build time / foundation extent to the last decimal, and — unlike a tick bug — is invisible until you diff a stat. So the acceptance bar for this slice is **bit-identical parsed values**, not "compiles and runs."

## 2. Non-goals

- **Not** flipping any consumer (`get_i32`→`read_int`, etc.). That is study-S3..S7, later slices.
- **Not** deleting `get_i32`/`get_bool`/`get_percent`/`get_f32`. They stay until their consumers are repointed in a later slice; deleting them is study-S6.
- **Not** reproducing the CRC-hash store, lazy qsort, binary search, pointer-identity section cache, COM vtables, or fixed C buffers (study §3 INACTIVE). The `HashMap`-backed `IniSection` is the equivalent store; we reproduce *values*, not the storage mechanism.
- **Not** an INI *writer* (read-only client; study §2f).
- **Not** introducing any `sim/` / `render/` / `ui/` / `audio/` / `net/` dependency. The service depends only on `std`, `crate::rules::ini_parser`, and (for the documented fixed-conversion at the boundary) `crate::util` fixed-math. (#1 invariant.)
- **Not** changing `advance_tick` phase order, `SNAPSHOT_VERSION`, or any runtime hash *directly* (see §6 rollout — this slice is read-only w.r.t. the hash).

---

## 3. Current architecture this slice touches (verified this run)

Read live this run; line numbers cited are from the files as they exist now.

- **`src/rules/ini_parser.rs`** — the raw store ("INIClass" analog). `IniSection { name, entries: HashMap<String,String>, key_order }`; lowercase-keyed case-insensitive lookup. Typed reads are methods on `IniSection`:
  - `get` (`:61`), `get_i32` (`:70` — `self.get(key)?.trim().parse::<i32>().ok()`, **no hex, no atoi leniency, None on parse-fail**), `get_f32` (`:77` — plain parse, no percent), `get_light_f32` (`:86` — comma-stop quirk, the one place a gamemd parse quirk is already mirrored), `get_percent` (`:100` — single **trailing** `%` strip ÷100), `get_bool` (`:114` — **whole-word** match `yes/true/1` / `no/false/0`, None otherwise), `get_list` (`:128` — comma split + trim), `get_values` (`:142` — numbered-key registry).
  - `IniFile::from_str` (`:197`) — strips `;` inline comments (`:246`), trims via `str::trim` (Unicode whitespace), merges duplicate in-file sections (later key wins). `IniFile::merge` (`:304`) — md-over-base, additive, later-wins.
- **`src/rules/mod.rs`** — module list; `ini_parser` declared at `:27`. NEW modules slot in here.
- **`src/rules/foundation.rs`** — `FOUNDATION_TABLE: [FoundationDef; 22]` (`:17`), `DEFAULT_FOUNDATION_ID = 0` (`:15`). Default-to-id-0=`1x1` matches gamemd `FUN_00474DA0` `return 0` (study P10). This is the existing correct enum-by-name shape.
- **`src/rules/object_type.rs`** — inline `match value.trim().to_ascii_lowercase()` enum tables: `BuildCategory::from_ini` (`:59`), `PipScale::from_ini` (`:87`), `FactoryType::from_ini` (`:117`). These are the scattered enum-by-name re-impls a shared helper folds (later slice).
- **`src/rules/warhead_type.rs`** — `CellSpread` via `get_f32` (`:118`), `PercentAtMax` via `get_f32` then `×100` (`:121-124`), `Verses` via dedicated `parse_verses` (`:115`/`:197` — strips **trailing** `%`, `unwrap_or(100.0)` on junk), `ProneDamage` via `parse_prone_damage_basis_points` (`:212`, trailing-`%` `strip_suffix` at `:218`). These are the genuine `%`/double consumers (later slice; **Verses is NOT a `get_f32` 100×-wrong bug — that draft claim was refuted, study Reviewer note**).
- **`src/bin/extract-ini.rs`** — dumps `rules.ini`/`rulesmd.ini`/`art.ini`/`artmd.ini` + theaters into `ini/` (the corpus the study-S2 harness loads). No accessor logic; relevant only as the corpus source.

**The gap (study §4.3):** there is **no central typed accessor with gamemd semantics**; ~852 `get_*`/`.get(` call sites + the raw-parse re-impls each re-implement default/percent/hex/bool/enum logic. Zero hex parsing exists anywhere in `src/rules/` (study §Pass-2 A last row) → the R2 hex gap is total.

---

## 4. Proposed module / service boundary

### 4.1 Placement & ownership

```
src/rules/
  ini_parser.rs        // KEEP unchanged this slice: IniFile/IniSection raw store (the "INIClass" analog)
  ini_value.rs   (NEW) // the typed-accessor service: the "CCINIClass ReadX" analog
  ini_enum.rs    (NEW) // generic enum-by-name table helper (the FUN_00474DA0 round-trip)
```

- `IniFile`/`IniSection` stay the raw case-insensitive store. No CRC, no binary search — `HashMap` is the verified-equivalent (study §3 INACTIVE: identical OUTPUT for non-colliding stock keys; CRC-collision shadowing is a latent TS bug we must NOT reproduce).
- `ini_value.rs` owns the `ReadInt/ReadBool/ReadDouble/ReadString/Read3Int/ReadMinMax/ReadPoint/ReadRect/ReadColorRGB/ReadSpeed/ReadRange` semantics. **Default-on-*miss*, parsed-value-on-present** is the encoded invariant (study P4/P18) — the sharp divergence from today's `get_*(...).unwrap_or(default)` which falls to default on *parse-fail too*.
- `ini_enum.rs` owns one `enum_by_name(value, table, default_id)` matching `FUN_00474DA0` (study P10), reusable by Foundation / MovementZone / SpeedType / Layer / the `object_type` inline matches.

**Form decision (open, see §9):** methods on `IniSection` vs free functions taking `&IniSection`. Default recommendation: **methods on `IniSection`** in `ini_value.rs` via `impl IniSection` (Rust allows inherent-impl split across files in the same module tree), so the call shape mirrors today's `section.get_i32(...)` → `section.read_int(...)` and minimizes diff at flip time. Alternative considered: a wrapper `CcIni<'a>(&'a IniSection)` newtype that *only* exposes `read_*` (forcing consumers off the old methods); rejected for Slice 1 because it churns every call site at introduction time, violating "additive/shadow, no consumer changes."

### 4.2 Surface sketch (signatures — proposed, NOT written to the tree)

```rust
// ini_value.rs — gamemd CCINIClass ReadX-equivalent typed reads.
// INVARIANT (P4/P18): "present" = key exists (even if value is empty). A present key
// ALWAYS returns its parsed value (which for int may be atoi("")=0); `default` is
// returned ONLY when section/key is absent. This is NOT unwrap_or(default).
impl IniSection {
    /// ReadInt: '$xx' / 'xxh' (case-insensitive 'h') hex, else C-atoi leniency.
    /// Default ONLY on absent key. Present-but-nonnumeric -> atoi result (0), NOT default.
    fn read_int(&self, key: &str, default: i32) -> i32;                 // P1,P2,P3,P4,P18

    /// ReadBool: toupper(first char) in {'1','T','Y'}=true / {'0','F','N'}=false / else default.
    /// NB 'on'/'off' are NOT recognized (first char 'o') -> default.
    fn read_bool(&self, key: &str, default: bool) -> bool;             // P6,P18

    /// ReadDouble: sscanf "%f" through f32, widen (double)(float), then *0.01 iff value
    /// contains '%' ANYWHERE (not just trailing). Returns the gamemd double UN-truncated;
    /// the consumer truncates toward zero at ITS boundary (never .round() here). [S0-pinned]
    fn read_double(&self, key: &str, default: f64) -> f64;            // P7

    /// ReadString: trimmed (bytes <=0x20 both ends) value, or default on ABSENT key
    /// (present-empty -> ""). No C buffer cap in Rust; debug_assert at the per-accessor
    /// cap (smallest 32) to surface a corpus value that would have truncated. (P5/P18)
    fn read_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str; // P5,P18

    /// Comma tuples. All-defaults copied on MISS (P8). Each component parsed via the
    /// atoi-lenient int rule (P3), matching sscanf field-stop behavior.
    fn read_3int(&self, key: &str, default: [i32; 3]) -> [i32; 3];     // P8
    fn read_minmax(&self, key: &str, default: [i32; 2]) -> [i32; 2];   // P8
    fn read_point(&self, key: &str, default: (i32, i32)) -> (i32, i32);            // P9 (COMMA)
    fn read_rect(&self, key: &str, d: (i32,i32,i32,i32)) -> (i32,i32,i32,i32);     // P9 (COMMA)

    /// ReadColorRGB: COMMA "%d,%d,%d" -> [u8;3]. Per-component is sscanf %d (NOT atoi-lenient,
    /// NO $/h hex) -> stops at first non-digit. Default RGB on miss/parse-fail. (P21)
    fn read_color_rgb(&self, key: &str, default: [u8; 3]) -> [u8; 3];  // P21

    /// ReadSpeed (TRANSFORM): read_int(-1); -1 -> default; else min(v,100),
    /// (v<<8)/100 round-toward-zero, min(result,255). e.g. 100->255, 50->128, 7->17. (P19)
    fn read_speed(&self, key: &str, default: i32) -> i32;             // P19

    /// ReadRange (TRANSFORM): read_double(-1.0); == -1.0 -> default; else ftol
    /// truncate-toward-zero to i32 (5.9 -> 5, never rounds). (P20)
    fn read_range(&self, key: &str, default: i32) -> i32;            // P20
}

/// C-atoi-equivalent leading-numeric parse: optional sign + leading digits, stop at
/// first non-digit; "5cells"->5, "abc"->0, ""->0, "  7 "->7, "-50"->-50. (P3)
fn atoi_lenient(s: &str) -> i32;

// ini_enum.rs — the FUN_00474DA0 round-trip helper.
pub struct EnumByName { pub name: &'static str, pub id: i32 }
/// ReadString-into-buf-with-default-name then WHOLE-STRING case-insensitive compare
/// against the table; matched id else default_id. (P10) substring does NOT match.
pub fn enum_by_name(value: &str, table: &[EnumByName], default_id: i32) -> i32;
```

### 4.3 What stays / what moves (boundary discipline)

- `read_double` returns an `f64` mirroring gamemd `(double)(float)x [×0.01]`. The **single** `f64`→`SimFixed` conversion stays in the existing `util` fixed-math (`sim_from_f32`-equivalent), so every parse→fixed path goes through ONE pinned conversion (determinism). **No `f32`/`f64` enters `sim/`** — only the converted `SimFixed`. `f32`/`f64` is legitimate here strictly as the *parse-boundary* type (gamemd uses `double`/`float` at exactly this boundary).
- Per-key **defaults stay at call sites** (they ARE the per-field semantics) — the service does NOT centralize 852 defaults. Moving only the *parse* (not the default) into `read_*` is what kills the P4 "parse-fail → unwrap_or" drift without a giant default registry.

---

## 5. gamemd behavior contract this slice must reproduce

Every row below is a TESTABLE invariant; the study's P-numbers and addresses are the verified source. (Study §5 + §Pass-2 A. Default verdict = DRIFT until a test proves bit-identity.)

| Contract | Exact rule | Study ref |
|---|---|---|
| **Hex `$` prefix** | value `$1A`→26, `$FF`→255, `$0`→0 (`sscanf "$%x"`, fmt `0x00825BB8`). | P1 |
| **Hex `h` suffix** | tolower(last char)=='h' → `sscanf "%xh"` (fmt `0x00825BB4`): `1Ah`→26, `FFh`→255, `0FFH`→255. ASCII tolower (`FUN_007caff4`) → case-insensitive. | P2 |
| **atoi leniency** | non-hex via C `atoi`: `100`→100, `-50`→-50, `5cells`→5, `  7 `→7, `abc`→0, ``→0. gamemd NEVER returns "None" from a present value. | P3 |
| **Default only on ABSENT key** | present key → parsed value (int may be atoi=0); default only on null/absent section/key. **Divergence from `unwrap_or(default)`.** | P4, P18 |
| **String trim + default-on-miss** | strtrim strips bytes ≤0x20 BOTH ends (lead break `0x20<byte`; trail zero while `byte<0x21`, `0x00727CF0`); default on absent; present-empty → `""`. | P5, P18 |
| **Bool first-char** | `toupper(value[0])` ∈ {`1`,`T`,`Y`}→true, ∈ {`0`,`F`,`N`}→false, else default (`0x005295F0`). `off`→default (NOT false). | P6 |
| **Double `%`→×0.01, single-precision** | `sscanf "%f"` (fmt `0x00825BD8`) into float → `(double)(float)` → `× 0.01` (`0x007E3808`) iff `strchr(value,'%')` matches `%` ANYWHERE. `50%`→0.5, `12.5%`→0.125, `0.5`→0.5, `7`→7.0. **No ftol here** — value returned un-truncated. | P7 |
| **3Int / MinMax** | COMMA `"%d,%d,%d"` / `"%d,%d"`; all-defaults on miss. | P8 |
| **Point/Size/Rect — COMMA** | `"%d,%d"` / `"%d,%d,%d,%d"` (verified COMMA, fmt `0x0081C000` / `0x00825BBC`; the draft's "space" claim was WRONG). Reuse comma tokenization, NOT a space split. ReadRect seeds `"0,0,0,0"` so missing fields keep default component. | P9 |
| **ColorRGB triplet** | COMMA `"%d,%d,%d"` → `[u8;3]`; per-component plain `%d` (NOT atoi-lenient, NO hex). | P21 |
| **ReadSpeed transform** | `read_int(-1)`; -1→default; else `min(v,100)`, `(v<<8)/100` round-toward-zero, `min(,255)`. `100→255`, `50→128`, `7→17`, `0→0`. | P19 |
| **ReadRange transform** | `read_double(-1.0)`; `==-1.0`→default; else ftol truncate-**toward-zero** (`5.9→5`). NB: do NOT use `util::sim_to_i32`/`to_num::<i32>()` — that floors toward −∞ and diverges on negatives (ledger #18). ftol gate `0x007c5f00`, CW `0x00822d80`=`0x0E7F`. | P20 |
| **Enum-by-name** | ReadString(default = `table[default_idx].name`) → WHOLE-STRING case-insensitive (`_stricmp`, `0x007c8d20`) compare → matched id else table default (Foundation→0=`1x1`, MovementZone→-1, Action→0). | P10 |
| **Case-insensitive section/key** | `[General]`==`[GENERAL]`; `Cost`==`COST`. (Already true in Rust `HashMap`.) | P13 |
| **Merge: YR patches base** | `rules.ini` then `rulesmd.ini` on top (later-wins, additive); same `art`/`artmd`. (Already correct — `app_init_helpers` base-then-md, study §9.) | P14 |
| **Verses precision SPLIT** | `Verses` does **NOT** route through `read_double`; it strtok-splits then per-token full-f64 `strtod` (no `%`) or `atoi*0.01` in double (has `%`). Generic `read_double` carries f32-narrowed precision; Verses carries double. (Slice 1 only documents the split; the `parse_verses` fold is study-S5.) | P7 fold-in, §Pass-2 D |

---

## 6. Shadow-first rollout shape

This is **load-time** substrate, so the Mission/Radio "shadow → invert → authoritative → SNAPSHOT_VERSION bump → parity harness" rhythm adapts (study §8 preamble): there is no per-tick hash for the parser itself. The state-hash relevance is **indirect** — a changed parsed value changes a unit stat which changes `state_hash`. So the analog of "shadow" is: *add the new accessor, prove it equals the old accessor on the entire stock corpus (or document the intended divergence), and change nothing downstream.*

**Slice-1 phases (all additive / read-only w.r.t. runtime hash):**

1. **S0 — BLOCKING gate: pin ReadDouble→SimFixed precision (research+test, no consumer change).** Decide the Rust path that is bit-identical after `SimFixed` conversion: gamemd computes `(double)(float)sscanf("%f")` **then** `×0.01` in `double`. The candidate Rust path is `s.parse::<f32>() as f64` then `*0.01_f64` then the single `sim_from_*` conversion. **Acceptance gate:** `test_read_double_precision_matches_gamemd` over a boundary table {`0`,`1`,`100%`,`50%`,`12.5%`,`0.016`,`.9`, a representative `Verses` token, plus a NEGATIVE and a `%`-with-decimal} — each row's resulting `SimFixed` must equal the value computed via the pinned path. **Until S0 is green, no percent/Verses consumer may flip in any later slice.** (DRIFT acknowledged, gated.)
2. **S1 — introduce the service (additive, shadow).** Add `ini_value.rs` + `ini_enum.rs` with the §4.2 surface. **No consumer changes.** Unit acceptance tests (study §8 S1): `test_read_int_hex`, `test_read_int_atoi_leniency`, `test_read_bool_first_char`, `test_read_double_percent` (after S0), `test_read_string_trim_default`, `test_read_point_comma` + `read_rect`, `test_read_color_rgb`, `test_read_speed_clamp`, `test_read_range_truncates`, `test_enum_by_name`, plus present-empty/absent cases (P18).
3. **S2 — corpus equivalence harness ("the shadow assert").** A test that loads stock `rulesmd.ini` + `artmd.ini` (from `ini/`, produced by `extract-ini`) and, for every key currently read via `get_i32/get_bool/get_percent/get_f32`, asserts `read_*` either (a) equals the old accessor's value, or (b) **documents an intended divergence** (hex / first-char-bool / `%`-anywhere / atoi-leniency) with the gamemd-correct expected value. **Acceptance:** `test_ini_accessor_corpus_parity` — enumerates every divergence row; each row is either a cited gamemd-correct fix or zero. **No silent diffs.** Also runs the P5 buffer-cap scan: flag any enum/zone/action value > 31 chars or any 128-cap list value > 127 chars (currently UNCHECKED; surfaced not triaged).

**What is read-only vs hash-relevant in this slice:**
- **Read-only / NOT hash-relevant directly:** the entire Slice-1 deliverable. The service is *added*; nothing consumes it yet. `state_hash`, `SNAPSHOT_VERSION`, and the deterministic-replay parity harness are **untouched** by Slice 1.
- **Where SNAPSHOT_VERSION / the global parity harness apply:** later slices (study-S6). When a consumer flips and the new parse *corrects a real drift* (e.g. a stock hex value that previously read `None`→call-site default), the corrected stat changes a stock-skirmish replay's `state_hash` → that later slice re-baselines the parity harness with a one-line cited reason per changed value and bumps `SNAPSHOT_VERSION` if a serialized stat layout changes. **Slice 1 deliberately stops before that boundary** so the proof harness (S2) can quantify exactly which keys will move before any of them do.

**Why this ordering:** the corpus harness (S2) is the safety net that tells us, *before* a single consumer flips, the complete list of keys whose value will change and by how much. That list is the input to every later flip slice's parity-harness re-baseline.

---

## 7. Ad-hoc Rust to retire (enumerated; flipped in LATER slices, not Slice 1)

Slice 1 does **not** edit these — it builds the replacement and proves equivalence. Listed so the later flip slices have an exact target set (study §7, file:symbol verified this run):

- `src/rules/ini_parser.rs:get_i32` (`:70`) — no hex, no atoi leniency, default-on-parse-fail → `read_int` (P1–P4). *(study-S4)*
- `src/rules/ini_parser.rs:get_bool` (`:114`) — whole-word match → `read_bool` first-char (P6). *(study-S4)*
- `src/rules/ini_parser.rs:get_percent` (`:100`) — single trailing `%` → fold into `read_double` (`%`-anywhere, P7). *(study-S5)*
- `src/rules/ini_parser.rs:get_f32` (`:77`) — plain parse, no percent. Callers `warhead_type.rs:118 CellSpread`, `:121 PercentAtMax` → `read_double` (atoi/leniency + `%` parity). *(study-S5)*
- `src/rules/ini_parser.rs:get_light_f32` (`:86`) — comma-stop quirk; KEEP the behavior but re-express as a documented `read_*` variant (gamemd `%f` stops at comma). *(study-S5, lowest priority)*
- `src/rules/warhead_type.rs:parse_verses` (`:197`) — trailing-only `%`, `unwrap_or(100.0)` on junk → fold into the comma-tokenize + double path so `%`-anywhere / atoi-leniency edges match (Verses stays **full f64**, NOT the f32-narrowed generic path). *(study-S5)*
- `src/rules/warhead_type.rs:parse_prone_damage_basis_points` (`:212`, trailing `strip_suffix('%')` at `:218`) — fold the `%` handling onto the shared double path. *(study-S5)*
- `src/rules/terrain_rules.rs:354` (`trim_end_matches('%')`, study §7) — trailing-only → `read_double`. *(study-S5)*
- `src/rules/object_type.rs:BuildCategory::from_ini` (`:59`), `PipScale::from_ini` (`:87`), `FactoryType::from_ini` (`:117`) — inline `match lowercased` → `enum_by_name` + per-enum `&[EnumByName]` tables (P10). *(study-S3)*
- `src/rules/foundation.rs:FOUNDATION_TABLE` (`:17`) — already gamemd-correct (default id 0); re-express over the shared `EnumByName` helper so all enum-by-name share one path. Behavior-preserving, lowest priority. *(study-S3)*
- `Speed=` / `Range=` / `MinimumRange=` consumers — any reading these as a raw int/double instead of via `read_speed` (P19) / `read_range` (P20) is DRIFT; audit `rules/*_type.rs` and repoint. *(study-S4/S5)*
- The long tail: ~85–192 raw `parse::`/`from_str_radix`/`strip_suffix`/`to_lowercase` sites across `rules/*.rs` (heaviest `weapon_type.rs`, `projectile_type.rs`, `warhead_type.rs`) — audit per-system; route int/bool/double/percent/enum reads through the service; leave already-clean derived math alone. *(per-system, never one bulk change — CLAUDE.md change-management)*

---

## 8. Tiny-detail ledger (must NOT drift)

Each is an observable that a sloppy port would silently get wrong:

1. **Default on ABSENT, parsed-value on PRESENT** (incl. present-empty). NOT `unwrap_or` on parse-fail. (P4/P18)
2. **`$` prefix AND `h` suffix hex**, `h` case-insensitive; `atoi` fallback for everything else (leading-numeric, stop at non-digit, `abc`→0). (P1/P2/P3)
3. **Bool = first char only**, T/Y/1 vs F/N/0; `on`/`off`→default (NOT false). (P6)
4. **`%`-ANYWHERE → ×0.01**, not just trailing; via `strchr` byte `0x25`. (P7)
5. **ReadDouble single-precision round-trip**: `(double)(float)x` THEN ×0.01 in double — last-ULP-sensitive; pin before any percent consumer flips. (P7/S0)
6. **ReadDouble returns UN-truncated**; truncation toward zero happens at the *consumer* boundary, never `.round()`, never truncate-at-read. (P7/P20)
7. **strtrim strips bytes ≤0x20 BOTH ends** (space + all ASCII control), not Unicode whitespace. (P5)
8. **Point/Size/Rect are COMMA-delimited** (`"%d,%d"` / `"%d,%d,%d,%d"`), NOT space. (P9)
9. **ReadColorRGB per-component is plain `%d`** — NOT atoi-lenient, NO `$`/`h` hex, stops at first non-digit. (P21)
10. **ReadSpeed transform**: clamp 100 → `(v<<8)/100` round-toward-zero → clamp 255 (`100→255`, `7→17`). The transform IS the value. (P19)
11. **ReadRange transform**: ftol truncate-toward-zero (`5.9→5`). (P20)
12. **Enum match is WHOLE-STRING case-insensitive**, substring does NOT match; per-table default id (Foundation 0, MovementZone -1, Action 0). (P10)
13. **Verses precision split**: Verses stays full-f64; generic `read_double` is f32-narrowed. Do NOT collapse them. (P7 fold-in)
14. **Merge order base-then-md, later-wins, additive** (already correct — do not regress). (P14)
15. **`;` inline comment strip at LOAD**, distinct from `%`; a trailing `%` with no `;` survives to ReadDouble. (P16, already in `from_str`)
16. **All-defaults copied on miss for tuples** (not partial). (P8)
17. **Per-accessor buffer truncation** (smallest cap 32 for enum/zone/action) — debug-assert guard in Rust; UNCHECKED across full corpus, surface in S2 scan. (P5)
18. **ftol truncates toward ZERO, not toward −∞** (P20). The existing `util/fixed_math.rs:71 sim_to_i32` uses `to_num::<i32>()` which discards fractional bits toward **−∞** (verified `fixed-1.31.0/src/macros_from_to.rs:98-101`; its doc comment "rounds toward zero" is WRONG). gamemd ftol (RC=11) truncates toward zero. Diverges on **negatives** (`-5.9`: floor=`-6` vs ftol=`-5`). `Range`/`MinimumRange` are non-negative in stock so harmless there, but `read_range` must NOT be built on `sim_to_i32` — truncate toward zero explicitly. `read_speed`'s `(v<<8)/100` is fine (Rust i32 `/` already truncates toward zero). Likewise the f64→SimFixed data-load conversion uses **round-to-nearest-ties-even** (`from_num`), NOT truncation — pin it in S0. (Correction 2/1)

---

## 9. Open questions / assumptions for design review

1. **Surface form — methods vs newtype wrapper.** Recommended: `impl IniSection` methods in `ini_value.rs` (mirrors `get_i32`→`read_int`, minimal flip diff). Alternative: a `CcIni<'a>` newtype that hides the old methods to force migration. **Decision needed** — affects every later flip slice's diff shape.
2. **S0 pinned conversion path — reframed (see Correction 1).** `SimFixed = I16F16` is **coarser** than f32 (16 frac bits, `util/fixed_math.rs:23`), so there is no "bit-identical to gamemd" target — gamemd keeps the raw double. The real load-bearing fact: `fixed::from_num(float)` **rounds to nearest, ties-to-even** (`macros_from_to.rs:33-36`), not truncate. The gate is whether the f32-path and f64-path land on the same 16.16 value after that rounding (they should, because 16.16 quantization is coarser than the f32-vs-f64 gap for the stock domain — but PROVE it with the boundary table incl. a value near a 1/65536 tie). The S0 test is the gate, not more Ghidra. *(study §F)*
3. **`atoi` semantics on edge inputs** — confirm the Rust `atoi_lenient` matches C `atoi` for: leading `+`, `0x`-prefixed (C `atoi` does NOT treat `0x` as hex — `0x1A`→0 via atoi, but ReadInt's `$`-branch is separate; a value `0x1A` would hit the atoi fallback → 0, NOT 26). **Need a test row to lock this** (does any stock key use a `0x`-prefixed int? If so this matters).
4. **Present-empty for transform accessors** — `Speed=` present-empty → `read_int` returns atoi("")=0 (NOT the -1 sentinel) → `read_speed` returns `(0<<8)/100`=0, not the caller default. This is correct per P4/P18 + P19, but means a present-empty `Speed=`/`Range=` silently resolves to 0 instead of the call-site default. **Promoted to an S2 corpus assertion** (Correction 4): scan stock `rulesmd.ini`/`artmd.ini` for any present-empty `Speed=`/`Range=`/`MinimumRange=` and surface it.
5. **Scope confirmation** — this spec treats "Slice 1" as study-{S0,S1,S2} (service + proof harness, zero consumer flips). If the user intends Slice 1 to also flip the enum consumers (study-S3, output-equivalent, lowest-risk), say so — it would extend acceptance to the S3 tests but keep the slice hash-neutral (enum output is unchanged).
6. **Buffer-cap policy** — debug-assert (panic in debug, silent in release) vs hard error vs silent truncate-to-match-gamemd. Recommendation: debug-assert + the S2 corpus scan to prove no stock value trips it; do NOT silently truncate (that reproduces a latent bug for non-stock/modded data). **Decision needed.**

---

## 10. Acceptance summary (Slice 1 done = all green)

- `cargo test -p vera20k` green with the new `ini_value` / `ini_enum` unit tests (study §8 S1 list).
- `test_read_double_precision_matches_gamemd` (S0) green — see Correction 1: the gate is the **16.16 round-to-nearest-ties-even quantization** of gamemd's reference `(double)(float)x [×0.01]` double, NOT "bit-identical to gamemd" (gamemd has no 16.16 value). Assert (a) `sim_from_f64(ref) == sim_from_f32(ref as f32)` for the 16.16 consumers over the boundary table (proves f32-vs-f64 path choice is quantization-irrelevant for the stock domain) and (b) the chosen path equals `from_num(ref_double)`. Include negative and `%`-with-decimal rows.
- `test_ini_accessor_corpus_parity` (S2) green — every divergence vs the old accessors over stock `rulesmd.ini`+`artmd.ini` is either zero or a cited gamemd-correct fix; no silent diffs; P5 cap scan reports no over-length stock value (or surfaces the exact ones).
- **No consumer flipped, `SNAPSHOT_VERSION` unchanged, runtime `state_hash` untouched** — the slice is additive/shadow by construction.

---

## Design-review corrections (adversarial review, 2026-06-04)

**Verdict: YELLOW.** Parity claims are well-grounded in the study (all cited file:line and P-numbers re-verified against source this run — see below); the tiny-detail ledger is covered; layering and rollout shape are correct. Two real correctness traps in the S0/transform precision story must be fixed before write-plan, and three framing/scope issues need tightening. None block the slice; all are doc-level.

### Source re-verification (this run — grep/Read of current tree)
- `ini_parser.rs`: `get_i32:70`, `get_f32:77`, `get_light_f32:86`, `get_percent:100` (`strip_suffix('%')` :102), `get_bool:114` (whole-word), `get_list:128`, `get_values:142`, `merge:304` — all **confirmed at the cited lines** (`Read ini_parser.rs:55-154`).
- `warhead_type.rs`: `Verses` via `parse_verses` (`:115`/`:197`), `CellSpread` `get_f32:118`, `PercentAtMax` `get_f32:121-124`, `parse_prone_damage_basis_points:212` + `strip_suffix('%'):218` — **confirmed** (`Read warhead_type.rs:110-233`).
- `object_type.rs`: `BuildCategory::from_ini:59`, `PipScale::from_ini:87`, `FactoryType::from_ini:117` inline matches — **confirmed** (`Read object_type.rs:55-124`).
- `foundation.rs`: `DEFAULT_FOUNDATION_ID = 0` `:15`, `FOUNDATION_TABLE: [_;22]` `:17`, id 0 = `1x1` — **confirmed** (`Read foundation.rs:1-45`).
- `terrain_rules.rs:354` `trim_end_matches('%')` — **confirmed** (`Read terrain_rules.rs:348-357`).
- `mod.rs:27` `pub mod ini_parser;` — **confirmed**; no `ini_value`/`ini_enum` exists (`ls src/rules/` + grep). Greenfield, as claimed.
- `ini_parser_tests.rs`: only `test_get_percent:209`; **no** hex/first-char-bool/atoi-leniency test — coverage-gap claim **confirmed** (`Grep`).
- `Speed=`/`Range=` retire targets are **REAL** (`Grep "Speed"/"Range"` in `rules/`): `object_type.rs:870 speed: get_i32("Speed")`, `weapon_type.rs:189 speed: get_i32("Speed")` (no `(v<<8)/100` transform → P19 DRIFT); `weapon_type.rs:185 Range: get_f32`, `:199 MinimumRange: get_f32`, `superweapon_type.rs:142 range: get_f32` (no ftol transform → P20 DRIFT).
- `SNAPSHOT_VERSION = 17` at `snapshot.rs:24` — confirmed untouched-by-this-slice is correct.

### CORRECTION 1 — S0's "bit-identical after SimFixed conversion" is mis-framed (and the framing hides the real risk).
`SimFixed = I16F16` = **16 fractional bits** (`util/fixed_math.rs:23`, `:7-9`; precision 1/65536 ≈ 0.0000153). gamemd's `ReadDouble` carries **f32-mantissa** precision (~24 bits). There is **no gamemd `SimFixed` value to be "bit-identical" to** — gamemd keeps the raw `double`. So S0 is NOT "match gamemd's fixed bits"; it is "pick the Rust load path whose 16.16 result is the closest correct quantization of gamemd's parsed double." Because 16.16 is *coarser* than f32, the f32-vs-f64 last-ULP question the doc worries about is mostly **washed out by the 16.16 quantization** — but the **rounding mode of the quantization is the real load-bearing fact** and the doc never pins it:
- `fixed::from_num(f32/f64)` **rounds to nearest, ties-to-even** (verified: `fixed-1.31.0/src/macros_from_to.rs:33-36`, `:175-178`), NOT truncate-toward-zero.
- The PASS-2 fact "float→int = _ftol2 TRUNCATE-toward-zero" applies to **ftol consumers** (ReadRange, P20), **not** to the `f64→SimFixed` data-load conversion (which has no gamemd analog).
**Rewrite S0's acceptance** as: enumerate the boundary table, compute gamemd's `(double)(float)x [×0.01]` reference double, then assert `sim_from_f64(reference) == sim_from_f32(reference as f32)` for the consumers that go through 16.16 (proving the f32-vs-f64 path choice is quantization-irrelevant for the stock domain) AND that the chosen path's 16.16 value equals `from_num(reference_double)`. The gate is the **round-to-nearest-even quantization**, not "bit-identical to gamemd."

### CORRECTION 2 — ReadRange (P20) truncate-toward-zero ≠ the existing `sim_to_i32` helper (toward −∞). DRIFT trap.
`sim_to_i32` (`util/fixed_math.rs:71-73`) uses `to_num::<i32>()`, which **discards fractional bits toward −∞** (verified: `macros_from_to.rs:98-101` "rounds towards −∞") — its own doc comment ("rounds toward zero") is **WRONG**. gamemd's ftol (RC=11) truncates **toward zero**. They differ for **negatives**: `-5.9` → `sim_to_i32` = `-6`, ftol = `-5`. `Range`/`MinimumRange` are non-negative in stock data so it is harmless *there*, but: (a) the write-plan must NOT reach for `sim_to_i32` to implement `read_range`'s ftol — it must truncate toward zero explicitly; (b) `read_speed`'s `(v<<8)/100` "round-toward-zero division" has the same sign trap (Rust `/` on i32 already truncates toward zero, so that one is fine — but say so). Added to the tiny-detail ledger as #18.

### CORRECTION 3 — "single f64→SimFixed conversion path" overstates reality; most INI doubles never become SimFixed.
The doc's §4.3 / §6.3 "every parse→fixed path goes through ONE pinned conversion" is aspirational, not current: `PercentAtMax`→`u8` (`warhead_type.rs:123` `(v*100).round() as u8`), `Verses`→`Vec<u8>` (0–200, `parse_verses:207`), `ProneDamage`→`u32` basis points (`:232`). **`Verses` is stored as `u8`, NOT full f64** — so the doc's tiny-detail #13 ("Verses stays full-f64; do NOT collapse") describes the *gamemd* contract (double[11]) but the *current Rust port already lossily narrows it to u8*. That is a pre-existing DRIFT the study's S5 must address; Slice 1 only documents the split, but the design should **not imply the Rust f64→SimFixed path is the single funnel** — it is one of several lossy boundaries, each needing its own pinned quantization in its flip slice. Reworded below.

### CORRECTION 4 — Scope/framing nits (no fix needed, flagged for write-plan).
- **OQ3 (`0x`-atoi):** confirmed correct as written — C `atoi("0x1A")`=0; the `$`/`h` hex branches are separate, so `0x1A` hits the atoi fallback → 0, not 26. Needs an S1 test row `test_read_int_0x_prefix_is_zero`. No stock key audited for `0x`-prefix reliance yet → keep as open question, add the corpus check to S2.
- **OQ4 (present-empty `Speed=`):** the `read_int("")→atoi("")→0` then `read_speed→(0<<8)/100→0` chain is correct per P4/P18/P19, but means a **present-empty `Speed=` yields 0, not the -1 absent-sentinel default**. S2 must scan stock for any present-empty `Speed=`/`Range=` (would silently become 0 vs the call-site default). Promote from assumption to an S2 corpus assertion.
- **`read_speed` applicability:** P19's `(v<<8)/100` transform is the locomotion-Speed accessor. `weapon_type.rs:189`/`object_type.rs:870` read `Speed=` as a plain int — **before** flipping them to `read_speed`, the write-plan must confirm each call site's `Speed=` is the transformed kind (vehicle locomotion) vs a raw bullet speed; do NOT blanket-apply the transform. The §7 "audit, don't bulk" wording covers this — keep it explicit.

### Inline fixes applied
- §8 ledger: **added #18** (ftol toward-zero ≠ `sim_to_i32` toward −∞; `sim_to_i32` doc comment is wrong; negatives diverge).
- §5 P20 row + §10: S0 acceptance reworded from "bit-identical after SimFixed conversion" to the quantization-rounding gate (Correction 1).
- §9 OQ list: OQ2 reworded to the round-to-nearest-even framing; OQ4 promoted to an S2 corpus assertion.
