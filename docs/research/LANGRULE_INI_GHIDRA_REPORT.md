# LANGRULE.INI — load path, consumed keys, parity verdict

**Status:** VERIFIED-from-binary (loader + retail-disk extraction), 2026-06-10.
**Scope:** RC-6 follow-up from the ScenarioClass/RulesClass substrate contract
(`docs/contracts/2026-06-10-scenarioclass-rulesclass-engine-substrate-implementation-contract.md`).

## Verdict

**LANGRULE.INI is a full secondary gameplay-rules layer, NOT a localization
file — but stock retail ships none, so it has zero parity impact.** For
stock-gamemd parity it is **out of scope**, and the Rust engine's current
omission (no `langrule` reference anywhere in `src/`) is correct.

## What it is

LANGRULE.INI is parsed through the **exact same** master section processor as
`rulesmd.ini` (`RulesClass::Process`, `0x00668BF0`) — it can override any
gameplay value (`[General]`, `[CombatDamage]`, `[Radiation]`, `[AudioVisual]`,
difficulty tables, and every type-class list). It is the same extra-rules-layer
mechanism modders use; it is not CSF/string data.

## Loader (verified)

- String `"LANGRULE.INI"` lives at `0x00826228` (single ASCII occurrence); two
  xref sites consume it.
- **Site 1 — `0x006686C0`** (outer rules driver; called by
  `ScenarioClass::Full_Init 0x00686b20`; carries a stale
  `CDFileClass__Constructor` Ghidra label that does not match its body).
  Verified via `decompile_function 0x006686C0`. Sequence: clear type arrays →
  read `[Maximums]Players` → `RulesClass::Process` over rules+rulesmd →
  construct `CCFileClass("LANGRULE.INI")` → `FUN_00473c50` availability probe
  (`decompile_function 0x00473c50`, checks vtable `Is_Available`+open) → **if
  present**, load into a stack `CCINIClass` and `RulesClass::Process` it → then
  the map-rules pass (gated by `g_GameMode != 0 && DAT_00a8b23c != 0`).
- **Site 2 — `0x0052cd70`** (the RULE*MD/ART*MD expansion rules-load path;
  `decompile_function 0x0052cd70`) — same idiom, loads LANGRULE.INI right
  before AIMD.INI.

**Merge order: base rules → rulesmd → LANGRULE → map `[overrides]`.** LANGRULE
overrides base rules; the map then overrides both.

## Consumed keys

There is **no dedicated key list**. LANGRULE inherits the entire
`RulesClass::Process` section set (every section rulesmd supports). All of it is
gameplay-affecting; none is treated as localization text.

## Retail presence (verified absent)

LANGRULE.INI / LANGRULEMD.INI are **absent** from this retail install. Verified
empirically with the project's own MIX parser (`AssetManager` +
`load_all_disk_mixes()`, mounting `language.mix`/`langmd.mix`/`expandmd01.mix`):
lookups for `langrule.ini` (hash `0xB3C17994`), `LANGRULE.INI`, and
`langrulemd.ini` (`0x585AA09F`) all returned not-found. Stock YR never reads it
— the path is live but the file does not exist. (LANGRULE/LANGRULEMD are
TS/RA2-era localization-rules hooks; in stock YR the file is simply missing.)

## If support is ever wanted (not now)

A faithful loader would parse LANGRULE.INI through the same path as
`rulesmd.ini` and merge it **after base rules, before map `[overrides]`**
(`rules → rulesmd → LANGRULE → map`). It is not localization data and would
override sim-visible values. No action for stock parity.
