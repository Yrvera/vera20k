---
name: verify-doc-swarm
description: "Project-scoped for VERA20k/RA2 Yuri's Revenge only. Use when the user invokes '/verify-doc-swarm' (with any flags), asks to orchestrate parallel research-doc audits, wants the highest-leverage stale or never-audited docs checked against gamemd.exe, or asks to reconcile multiple /verify-doc GREEN/YELLOW/RED results. Default mode is detection-only: at most 3 read-only subagents per wave, each running /verify-doc on one research doc, then ranks staleness, WRONG/MISLEADING findings, cross-doc contradictions, and correction-fact bundles; never edits docs. Also use, in '--fix' mode, when the user asks for a corpus/area/list doc-FIX swarm, says 'audit and correct these docs', or wants drifted research docs repaired in place: one doc per worker with disjoint write ownership, workers keep Ghidra read-only, at most 3 workers per wave, and the parent reconciles, spot-checks, and reports the verified in-place corrections."
---

# Verify Doc Swarm

Coordinate up to 3 parallel `/verify-doc` audits per wave for research documents in
`<main-checkout>/docs/research/`, then reconcile the bounded
GREEN/YELLOW/RED summaries into a ranked staleness, contradiction, and correction report.

Two modes:

- **DETECT (default):** detection-only. Workers audit; they never edit the audited doc or
  write replacement prose. Output is a ranked findings report plus a correction-fact queue
  that makes a later patch pass easy.
- **FIX (`--fix`):** audit **and correct**. Each worker owns exactly one doc, repairs the
  WRONG/STALE/MISLEADING claims it verified against the live binary — after diagnosing why
  each one drifted — and edits the doc in place with an inline Ghidra-call citation per
  correction. Disjoint ownership is what makes concurrent correction safe.

**Documentation truth bar:** a verified mismatch in fields, offsets, operators, ordering,
bytes, or pixels (off-by-one, `<` vs `<=`, offset off by 4, clamp `0xFF` vs `0x100`, wrong
order-of-operations, TS-legacy framed as live YR) is WRONG — never downgraded to "minor" or
"internal-only" — and in FIX mode gets corrected. The audit verdict is separate from current
skirmish fix priority.

**Label-adversarial Ghidra rule:** local Ghidra names, labels, comments, xref labels, and
decompiler-assigned symbol names are navigation hints only; this project may contain
polluted or stale labels from earlier scripts. Treat address plus verified role as stronger
than label name, surface label drift clusters as a root-cause pattern, and (FIX mode)
correct docs only from body/callsites/bytes/data refs/active-YR reachability — never from a
current display label alone.

## Iron Laws (FIX mode)

```
THE BINARY IS GROUND TRUTH. THE DOC IS A HYPOTHESIS. THE FIX IS A NEW CLAIM.
```
A correction can be wrong too. Every edit carries its binary evidence inline so the next
audit can re-check the fix the way it re-checks the original. An unsourced correction is
worse than the original drift — it launders a guess into apparent ground truth.

```
DIAGNOSE THE WHY, NOT JUST THE WHAT
```
Docs drift for mechanical reasons (see the root-cause taxonomy). Naming the root cause
tells you whether the fix is safe (a verified address shift is patchable; a
"probably-equivalent mechanism" is not) and predicts which sibling docs carry the same rot —
the swarm's marginal value over fixing docs one at a time.

```
DISJOINT OWNERSHIP OR NO SWARM
```
Concurrent *writes* are safe only because no two agents touch the same file. Dedupe the
queue, assign one doc per agent, never let an agent wander into a sibling doc.

```
STRUCTURAL-RED IS A STOP, NOT A REWRITE
```
The fix step repairs isolated, verified claims. A doc wrong at the foundation is left
untouched and routed to `/re-investigate` — patching its surface produces a doc that passes
a skim but still misleads.

```
TINY MISMATCHES COUNT — THEY ARE THE WHOLE POINT
```
The drift this swarm exists to catch is the silent kind nobody re-reads docs to find. An
agent that returns a fat doc with zero WRONG is suspect — spot-check it.

## Hard Gates

Both modes:

- **Dispatch at most 3 subagents at once.** Split larger queues into sequential waves of ≤3.
  The cap is a deliberate Ghidra-MCP and review-load bound.
- Use subagents only after the user explicitly invokes `/verify-doc-swarm` or otherwise asks
  for parallel/delegated doc audits or doc fixes.
- **Every worker's Ghidra MCP access is READ-ONLY.** Forbid `rename_function`,
  `rename_function_by_address`, `rename_data`, `rename_variable`, `create_label`,
  `create_function`, `add_struct_field`, `modify_struct_field`, `set_decompiler_comment`,
  `set_plate_comment`, `set_disassembly_comment`, `save_program`, and any other mutating
  operation. Concurrent Ghidra mutations race on global state; 25 read-only decompile
  clients are well inside the MCP server's concurrency budget, 3 mutating clients are not.
- Workers must not write `.rs` files, touch INI files, create "corrected copies" at new
  paths, edit sibling research docs, or modify any other location. Contradictions noticed in
  sibling docs are reported for the parent's reconciliation, never chased or edited.
- The parent may write only `docs/research/.audit-swarm-claims.md` and
  `docs/research/.audit-coverage-index.md`. The parent never edits audited docs; in FIX mode
  corrections happen only inside the owning worker (single exception: the parent reverting
  or repairing a spot-check failure it caught itself, noted in the report).
- **Write-integrity check before and after each wave.** The only files that may have
  changed: `AUDIT_LOG.md`, `.audit-swarm-claims.md`, `.audit-coverage-index.md`, and (FIX
  mode) the wave's owned docs. Any other doc edit, any `.rs`/INI edit, or any Ghidra
  mutation outside the explicit post-wave parent label-sync phase is a contract violation —
  stop and report before continuing.
- If 2 or more slots in a wave fail (more than 1/3 of a full wave), stop before
  reconciliation or the next wave and ask the user whether to retry failed slots, review
  successes only, or abort.
- Treat GREEN on a large/high-risk doc with suspicion until spot-checked.
- Every returned finding must distinguish verified binary evidence from inference, and every
  YR-activity assertion must say `Active in YR: Yes / No / Conditional` with evidence or
  default-gate context.
- **Structural-RED is not patched in place.** If a doc is wrong at the foundation (wrong
  primary function, wrong struct base/layout family, TS-legacy framed as YR across multiple
  sections), the worker makes NO edits, marks it RED with `needs_reinvestigate: true`, and
  recommends a bounded `/re-investigate` target. Do not let this swarm silently become a
  re-investigation.

DETECT mode only:

- Each worker may write **exactly one** file: append one line to
  `docs/research/AUDIT_LOG.md`, matching `/verify-doc` rules. It must not modify the audited
  doc.
- Reconciliation never applies doc rewrites or code fixes.

FIX mode only:

- **One doc per worker — disjoint write ownership, no exceptions.** Two workers must never
  be assigned the same doc, and a worker may edit ONLY its one owned doc. If the scan or the
  user list contains the same doc twice, dedupe before dispatch.
- Each worker may write **exactly two** files: (a) its one owned research doc (in-place
  corrections) and (b) one appended line to `docs/research/AUDIT_LOG.md`.
- **Correct only verified-binary-backed claims.** A worker may edit a claim ONLY when it has
  the correct value directly from the live binary this session, with the exact Ghidra MCP
  call recorded. Inference, "probably," YRpp labels, or a sibling doc's value are NOT
  grounds to edit. Unverifiable findings stay flagged in the report, not patched.
- **Diagnose before correcting.** Every edit names the root cause of the error (taxonomy
  below). "It's wrong, here's the right value" is not enough.
- **Ghidra label sync, if requested, is parent-only and serial.** Workers only report
  rename/label/comment candidates with the verifying Ghidra calls. The parent may apply them
  after all waves only when the user explicitly opted in (`--sync-ghidra-labels`), the
  mutating tools are available, and the parent has re-verified the exact address/target.
  Never mutate Ghidra while workers are running.
- Do NOT stage, commit, or push. Corrected docs are left in the worktree for the user to
  review with `git diff`.

If you catch yourself about to violate any of these, STOP and surface it to the user.

## Status Rubric

Use the `/verify-doc` status meanings consistently across all slots:

- `GREEN`: zero WRONG, zero MISLEADING, and less than 5% UNVERIFIABLE among audited
  load-bearing claims. Safe to rely on for the audited scope.
- `YELLOW`: isolated wrong/stale/unverifiable claims, or a partial audit, but the doc
  remains usable with the listed caveats.
- `RED`: structural errors such as wrong primary function, wrong struct base/layout family,
  TS-legacy framed as standard YR, or broad pseudocode that would mislead implementation
  work. Recommend targeted rewrite or `/re-investigate`.

Tiny mismatches are never downgraded: off-by-one, `<` vs `<=`, signedness, wrong offset,
wrong clamp, wrong default value, or wrong order-of-operations is `WRONG`.

## Invocation Modes

- `/verify-doc-swarm`: scan candidate docs, propose up to 3 targets, wait for confirmation.
- `/verify-doc-swarm <d1>, <d2>, ...`: validate explicit docs, then wait for confirmation.
- `/verify-doc-swarm --area <area>`: scan only one area, propose up to 3 targets, wait for
  confirmation.
- `/verify-doc-swarm --all`: corpus sweep — scan everything, propose a multi-wave plan
  (waves of 3, HIGH tier first), await one confirmation covering the whole run.
- `/verify-doc-swarm --dry-run`: scan/validate and print the dispatch plan, then stop.
- `/verify-doc-swarm --refresh-index`: rebuild the audit coverage index before ranking.
- `/verify-doc-swarm --patch-plan` (DETECT mode): after the audit swarm, group findings into
  correction-fact bundles for a later patch pass. Still does not edit docs or generate
  replacement prose.
- `/verify-doc-swarm --fix`: FIX mode. Combine with an explicit list, `--area`, or `--all`.
  Workers correct their owned doc in place. Without `--fix`, the swarm is detection-only.
- `/verify-doc-swarm --fix --sync-ghidra-labels`: FIX-mode modifier. After all doc-fix waves
  complete, collect worker-reported label candidates, re-verify them in the parent, and
  apply Ghidra renames/labels/comments serially if mutating tools are available.

Valid areas include `combat`, `movement`, `locomotion`, `pathfinding`, `vision`, `shroud`,
`power`, `production`, `placement`, `harvester`, `miner`, `building`, `infantry`, `unit`,
`aircraft`, `sidebar`, `shell`, `audio`, `animation`, `weapon`, `superweapon`, `terrain`,
`map`, `ai`, `radio`, `timer`, `core-engine`, `ini-parsing`, `bridges`, `chrono`, and
`skirmish-ui`. Treat area names as aliases where the corpus uses them: for example
`harvester` and `miner` both include `miner/` docs and refinery-dock reports.

If an explicit list has more than 3 docs, split it into sequential waves. If the user
supplies a system name like `garrison`, resolve it to concrete research docs before
dispatching.

## Step 0: Preflight

Before scanning or proposing slots:

1. Confirm `<main-checkout>/docs/research/` exists.
2. Confirm `AUDIT_LOG.md` exists or can be created by `/verify-doc` rules.
3. Confirm Ghidra MCP read-only access is available. If unavailable, stop and report the
   blocker.
4. If `--sync-ghidra-labels` is requested, confirm the relevant mutating Ghidra tools are
   available before promising label sync. If unavailable, run the fix swarm normally and
   report the label-candidate queue without applying it.
5. Confirm Ghidra MCP answers from a subagent. If the bridge is down, run `/ghidra-up`
   before dispatch, or offer serial `/verify-doc` audits instead of pretending to swarm.
6. Read `.audit-swarm-claims.md` if present and identify active `claimed` rows less than 4
   hours old. Do not propose the same doc while a fresh claim exists unless the user
   explicitly overrides.
7. Capture a cheap write-integrity baseline for `docs/research/`, repo `src/`, repo `ini/`,
   and the Ghidra mutating-tools ban. `git status --short` / `git diff --stat` plus doc
   mtimes is enough; this catches unexpected writes after dispatch, not pre-existing dirty
   state.
8. Use the research-index MCP as the first candidate/source map (preferred over raw grep at
   the 2000+ doc scale): `research_map` (filter by `system`) for a system/topic inventory,
   `research_validate` for freshness/staleness signals, `research_search` +
   `research_related` to enumerate a cluster. Use the repo-local CLI only for a missing MCP
   capability:

   ```text
   python tools/research_index/validate.py --system <system> "<topic>" --limit 12
   python tools/research_index/map.py --system <system> "<topic>" --limit 12
   ```

   If the exact system is unclear or returns zero docs, rerun without `--system`, then use
   the inferred exact system (for example `skirmish-ui`, `bridges`, `miner`, `chrono`). Use
   stale/unknown docs, missing links, and contradiction signals as candidate-ranking inputs.
   The index chooses better targets; it does not replace per-doc `/verify-doc` verification.

## Step 1: Build Or Read The Audit Index

For auto, scoped, `--all`, and refresh modes (skip for explicit lists), use
`docs/research/.audit-coverage-index.md`:

- If absent, rebuild it.
- If present and modified within 7 days, use it and tell the user the cache date.
- If older than 7 days or `--refresh-index` was passed, rebuild it and tell the user.

When rebuilding, recursively walk every `*GHIDRA_REPORT.md` and other RE/research docs in
`docs/research/`, including subdirectories such as `miner/`, `units/`, and `traces/`.
Exclude plans, design specs, generated correction-only files such as
`*_VERIFY_DOC_AMENDMENTS.md`, and patch-cluster notes unless explicitly requested. For each
doc, extract:

- Last audit: most recent `AUDIT_LOG.md` entry for the filename, with date and
  GREEN/YELLOW/RED status. No entry means never audited.
- Patch status: most recent PATCHED/PATCHED-to-GREEN/PATCHED-to-YELLOW line for the
  filename, if any. Treat a doc patched after its last YELLOW/RED audit as
  `POST_PATCH_NEEDS_VERIFY` unless a later GREEN audit exists.
- Doc mtime: if the doc was edited after its last audit, it is post-audit drift.
- Cross-reference count: grep the rest of `docs/research/` for the filename, and grep repo
  `src/` for citations across `sim`, `rules`, `map`, `assets`, `render`, `sidebar`, `ui`,
  and `audio`.
- Surface-area proxy: count address citations like `0x00XXXXXX`, `+0x` offsets, `FUN_`
  references, function-name references, vtable-slot mentions, and INI key mappings.
- TS-legacy density: count callouts like `inherited from Tiberian Sun`, `SpecialFlags`,
  `FogOfWar`, `Tunnel`, `Subterranean`, and `Veins`.
- Implementation-load proxy: count references to `src/`, `Implementation Handoff`,
  `Current Rust`, `Required implementation effect`, and proposed test names.
- Recent edit signal: edited in the last 14 days and last audit predates the edit.

Write one flat Markdown bullet per doc:

```markdown
- NEVER_AUDITED | RADIO_PROTOCOL_GHIDRA_REPORT.md | mtime=2026-04-12 | xref=6 | offsets=38 | ts-density=2
- POST_AUDIT_DRIFT | BUILDING_POWER_GHIDRA_REPORT.md | last=2026-04-20 GREEN | mtime=2026-05-15 | xref=4 | offsets=29
- POST_PATCH_NEEDS_VERIFY | POWER_SYSTEM_GHIDRA_REPORT.md | last=2026-05-20 YELLOW | patched=2026-05-20 | xref=5 | offsets=52
```

Categories, highest leverage first: `STALE_RED`, `POST_PATCH_NEEDS_VERIFY`,
`POST_AUDIT_DRIFT`, `NEVER_AUDITED`, `STALE_YELLOW` (last audit YELLOW, older than 30 days),
`STALE_GREEN_90D` (last audit GREEN, older than 90 days), `FRESH_GREEN` (GREEN within 30
days).

Rank candidates:

- HIGH: `STALE_RED`, `POST_PATCH_NEEDS_VERIFY`, `POST_AUDIT_DRIFT` with xref >= 3,
  `NEVER_AUDITED` with xref >= 5, implementation-load >= 3, or ts-density >= 3.
- MEDIUM: `NEVER_AUDITED` with xref 1-4, `STALE_YELLOW`, or `STALE_GREEN_90D` with
  xref >= 5.
- LOW: isolated `STALE_GREEN_90D` or xref 0-1 docs.

Break ties by highest xref count, highest implementation-load, highest offset/address count,
then newest mtime when last audit predates it. Prefer canonical report docs over
amendment-only docs; prefer docs with implementation handoffs when the user is preparing
implementation work.

Building waves:

- **Default / `--area`:** take the top ≤3 by tier. That is wave 1. Propose it and stop;
  don't pre-plan further waves unless the user asks.
- **`--all`:** order the entire ranked corpus and split into waves of 3 (HIGH, then MEDIUM,
  then LOW). Present the full wave count and wave-1 contents; the user approves the run,
  then each wave gets a one-line "dispatching wave N/M" notice but not a fresh full
  confirmation.
- **Spread within a wave** when not `--area`: spread picks across 3-5 subsystems so a single
  root cause doesn't dominate a wave.
- **Dedupe.** A doc appears in at most one slot across the whole run.

## Step 1.5: Validate Explicit Targets

For each explicit target:

1. Resolve it to a concrete file under `docs/research/`. If ambiguous, list candidates and
   ask.
2. Confirm it is a research/RE doc, not a plan or design spec. Ask whether to drop
   questionable slots.
3. Grep `AUDIT_LOG.md` for an entry from today. If found, ask whether to skip, re-audit, or
   scope to a subsection.
4. If the doc was edited in the last 6 hours, a parallel session may be rewriting it —
   confirm before auditing a moving target.
5. If the target is an amendment, patch-cluster note, or audit-index file, ask whether the
   user meant the canonical source report instead.
6. Dedupe. If the list exceeds 3, split into waves and show the plan.

## Step 2: Propose And Confirm

Print a table with slot, doc, category, last audit, patch status, xref, offsets,
implementation-load, and tier, plus the mode line:

```markdown
Proposed swarm dispatch — wave 1 of M (N slots, mode: DETECT | FIX):

| Slot | Doc | Category | Last Audit | Patch | xref | offsets | impl | Tier |
|------|-----|----------|------------|-------|------|---------|------|------|
| 1 | MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md | STALE_RED | 2025-12-03 RED | none | 11 | 51 | 4 | HIGH |

Mode: DETECT (audit only, no doc edits) | FIX (agents edit their owned doc in place).
Ghidra label sync: OFF by default; ON only when --sync-ghidra-labels was requested and
mutating tools are available. Workers always remain read-only.
Confirm to dispatch, or list slot numbers to drop/replace.
```

**Wait for explicit confirmation such as "go" before Step 3.** A previous confirmation in
the same thread does not count for a new swarm. For `--dry-run`, stop after printing the
plan. For `--all`, the "go" authorizes the whole multi-wave run.

## Step 3: Append Work Claims

Append (never overwrite) to `docs/research/.audit-swarm-claims.md`. Create it if missing
with:

```markdown
# Verify-Doc-Swarm Work Claims

Active and historical claims from /verify-doc-swarm invocations. Prevents parallel
sessions from duplicating audit work. Append-only; stale claims (>4h `claimed`)
may be re-claimed but rows are not deleted.
```

Append one block per wave, naming the mode:

```markdown
## Swarm 2026-05-28T14:22 wave 1/3 (session abc123, mode FIX)

- 2026-05-28T14:22 - w1-slot-1 - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md - claimed
```

Statuses are append-only events: `claimed`, `done` (returned + `AUDIT_LOG.md` updated + doc
edited if FIX mode), `failed`. Do not edit or delete old rows; record completion by
appending a new row. A `claimed` row older than 4 hours is stale and may be re-claimed by
appending a new `claimed` row:

```markdown
- 2026-05-28T15:03 - w1-slot-1 - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md - done (RED, no edits, needs_reinvestigate)
- 2026-05-28T15:03 - w1-slot-2 - RADIO_PROTOCOL_GHIDRA_REPORT.md - done (YELLOW→corrected, 3 fixes)
```

## Step 4: Dispatch A Wave (≤3 Subagents)

Use parallel Agent tool calls with `subagent_type: general-purpose` — workers write files
(the audit-log line, and in FIX mode their owned doc), so read-only Explore agents will not
work. Spawn every slot before waiting so the wave runs concurrently. Do not create
user-visible background tasks for internal swarm work. Do not set an unavailable model
override; inherit the active model unless the user explicitly requested a supported model.

### Worker prompt template

Self-contained (workers do not auto-load this skill). Substitute `{{DOC_PATH}}`,
`{{DOC_FILENAME}}`, `{{SLOT}}`, and `{{MODE}}` (`DETECT` or `CORRECT`):

```
You are agent {{SLOT}} of a /verify-doc-swarm wave. Your single owned document is:

  {{DOC_PATH}}

You OWN this file exclusively. No other agent will touch it. Mode: {{MODE}}.

PHASE 1 — AUDIT (always). Run the /verify-doc procedure against gamemd.exe:
- Pick the ~15-25 load-bearing claims (addresses, struct offsets, formulas, thresholds,
  operators, orderings, vtable slots, INI mappings, "Active in YR" assertions, cross-refs)
  unless the doc is smaller; name any deliberately scoped-out sections.
- Verify each against the LIVE binary via Ghidra MCP. Read literally: operators
  character-by-character, constants digit-by-digit, offsets byte-by-byte.
- Classify each: CONFIRMED / WRONG / STALE / UNVERIFIABLE / MISLEADING.
- Tiny mismatches are WRONG, never "minor": off-by-one, `<` vs `<=`, signedness, offset
  off by 4, clamp 0xFF vs 0x100, wrong order-of-operations, pre- vs post-increment.
- For every "Active in YR" claim: re-trace the gating flag, read its default in
  ini/rulesmd.ini + the binary, trace live YR caller paths. TS-legacy framed as live YR
  is MISLEADING.
- Vtable claims: verify owner via COL→TypeDescriptor mangled name, verify slot at
  vtable+N*4, decompile the slot and confirm behavior. Ghidra's display label is NOT
  evidence; the mangled name wins.

PHASE 2 — DIAGNOSE (always, for each WRONG/STALE/MISLEADING finding). Classify: verified
binary fact vs inference, player-visible impact, likely downstream docs, and patchable vs
needs /re-investigate. Name the ROOT CAUSE from this taxonomy (or state a more specific
one):
  - GHIDRA_ADDRESS_SHIFT — re-analysis moved the address/label; semantics unchanged.
  - RTTI_LABEL_DRIFT — labeler rerun renamed a function/vtable slot since the doc was written.
  - PARAM1_TYPE_MISREAD — author read param_1 as int vs int* (×4 offset error), or vice versa.
  - OFFSET_RETYPED_WRONG — a constant/offset transcribed from memory, off by a small delta.
  - OPERATOR_OR_ORDER_DRIFT — `<`/`<=`, signedness, or A-before-B order misdocumented.
  - TS_LEGACY_AS_YR — a TS-only/flag-gated path documented as active in standard YR.
  - STRUCT_FAMILY_CASCADE — a base-class size change shifted many offsets by a constant.
  - INFERENCE_HARDENED — a "we infer / probably" guess written as a verified fact.
The root cause decides safety: a verified ADDRESS_SHIFT/RETYPE/OPERATOR fix is patchable;
an INFERENCE or "probably-equivalent mechanism" is NOT — flag it, do not edit.

PHASE 2.5 — GHIDRA LABEL CANDIDATES (always, report-only). If verification proves a
Ghidra function/data label/comment is stale or misleading, emit a parent-action candidate:
address, current display label/comment, proposed name/comment, candidate kind
(`rename_function`, `rename_data`, `create_label`, `set_comment`, etc.), root cause, and
the exact Ghidra calls that prove it. Do NOT apply it. Your Ghidra access stays read-only;
the parent decides later whether to mutate Ghidra serially.

PHASE 3 — CORRECT (only if MODE is CORRECT; skip entirely if DETECT).
- Edit ONLY {{DOC_PATH}}. Fix each WRONG/STALE/MISLEADING claim whose correct value you
  read directly from the binary THIS session.
- For each edit, replace the wrong text with the verified-correct text and append an inline
  citation in the form the doc already uses, e.g.:
    `(corrected 2026-05-28: was 0x16B3; binary shows vtable+0x16B7 via
     decompile_function 0x00521A40 — RTTI_LABEL_DRIFT)`
  Keep the doc's existing voice and structure; do not reflow unrelated prose.
- Do NOT invent. If you could not read the correct value from the binary, do NOT edit that
  claim — leave it and flag it UNVERIFIABLE in your return.
- STRUCTURAL-RED STOP: if the doc is wrong at the foundation (wrong primary function, wrong
  struct base/layout family, TS-legacy framed as YR across multiple sections), make NO
  edits, set needs_reinvestigate: true, and recommend a bounded /re-investigate target
  instead of rebuilding the doc inside this audit.

HARD CONSTRAINTS (non-negotiable):
- Ghidra MCP is READ-ONLY. NO rename_function, rename_function_by_address, rename_data,
  rename_variable, create_label, create_function, add_struct_field, modify_struct_field,
  set_decompiler_comment, set_plate_comment, set_disassembly_comment, save_program, or any
  other mutating tool. Decompile, read xrefs, read memory — that's all. You may only
  REPORT Ghidra label/rename/comment candidates for the parent.
- File-write budget: in DETECT mode you may write EXACTLY ONE file — one appended line to
  <main-checkout>/docs/research/AUDIT_LOG.md; do NOT modify the
  audited doc. In CORRECT mode you may write EXACTLY TWO files — {{DOC_PATH}} (in-place
  corrections) and that one AUDIT_LOG.md line. Touch NOTHING else: no other research doc,
  no "corrected copy" at a new path, no .rs file, no INI file, no other location.
- Do NOT chase, audit, or edit sibling docs. Note contradictions for the parent.
- Distinguish verified binary fact from inference in every finding.
- Scope is ONE doc. Do not expand.

RETURN-VALUE CONTRACT (mandatory, ≤150 lines):
1. YAML front matter:
   ```yaml
   slot: {{SLOT}}
   doc: {{DOC_FILENAME}}
   status_before: GREEN | YELLOW | RED
   status_after: GREEN | YELLOW | RED      # equals status_before in DETECT mode
   mode: DETECT | CORRECT
   result: COMPLETE | PARTIAL | FAILED
   doc_edited: true | false
   edits_applied: N
   audit_log_appended: true | false
   load_bearing_audited: N
   confirmed: X
   wrong: Y
   stale: Z
   unverifiable: W
   misleading: V
   needs_reinvestigate: true | false
   ghidra_label_candidates: N
   ```
2. The exact AUDIT_LOG.md line you appended, verbatim.
3. Top 5 most-impactful WRONG/MISLEADING findings with evidence: Ghidra address, offset,
   exact quoted doc text, exact binary value — plus the Phase-2 classification and
   root-cause tag for each.
4. CORRECT mode — edits applied: one line each with section/quote anchor, was→now,
   root-cause tag, and the Ghidra MCP call that verified the new value. Then findings NOT
   corrected: each WRONG/MISLEADING left unpatched with why (UNVERIFIABLE, structural-RED,
   inference), one line each.
   DETECT mode — correction facts when safe: section or quote anchor, exact verified
   binary fact, and evidence. Do not write replacement prose and do not edit the doc.
5. Sibling-doc contradictions noticed in passing (do not chase) — for the parent.
6. Ghidra label/rename/comment candidates (do not apply): one line each with address,
   current label/comment, proposed label/comment, candidate kind, root-cause tag, and the
   proving Ghidra MCP call(s). Write `None` if none.
7. Subsections deliberately scoped out (honesty about coverage).
8. One-line status: COMPLETE | PARTIAL (reason) | FAILED (reason).

Do NOT return: full claim tables, raw decompilation/assembly, raw INI dumps, narrative doc
recaps, or replacement prose beyond the one-line was→now anchors.

If you hit a hard blocker (Ghidra MCP outage, doc unreadable, target isn't a research doc,
scope too broad), append a FAILED line to AUDIT_LOG.md noting why, return status FAILED,
and stop. Do not retry indefinitely. Do not edit the doc on a FAILED audit.

REMINDERS:
- A `>` vs `>=` swap is WRONG. A 0x68 documented as 0x67 is WRONG. A field at 0x44
  documented as 0x40 is WRONG (and probably means the whole layout drifted — check the
  STRUCT_FAMILY_CASCADE root cause before patching one field).
- "Active in YR: Yes" on code gated by SpecialFlags & 0x1000 (FogOfWar) is MISLEADING.
- Vtable slot indices shift when the RTTI labeler reruns — re-confirm via the mangled name.
- Your correction is a new claim. Cite the binary, or do not make it.
```

After dispatching the wave, **wait for all N returns** before the per-wave integrity check.
Do not reconcile until every slot is complete, partial, or failed.

## Step 5: Collect Wave Returns, Integrity Check, Handle Failures

For each return:

1. Verify the returned `AUDIT_LOG.md` line was actually appended (grep date+filename).
2. FIX mode: if `doc_edited: true`, confirm via `git diff --stat` that ONLY that owned doc
   changed for this slot — never a sibling.
3. Append a `done`/`failed` row to `.audit-swarm-claims.md` (new row, don't rewrite).
4. Tally success/failure counts, GREEN/YELLOW/RED mix (FIX mode: status_before→status_after
   flips), edits_applied, needs_reinvestigate count, and `ghidra_label_candidates` count.
5. Run the write-integrity check vs the Step-0 baseline (allowed files per the Hard Gates).
   Any violation: STOP, report, do not start the next wave or reconciliation.

Treat malformed returns as PARTIAL: over 150 lines, missing YAML front matter, missing
audit-log line, missing status, missing edits/findings list, missing label-candidate
section, or missing scoped-out section list. Note it.

Failure-threshold gate (per wave): 2 or more slots failed (>1/3 of a full wave) → STOP,
print the failure list, do not dispatch remaining waves, ask the user how to proceed (retry
failed slots, review successes, or abort). One failure → proceed, flagged clearly.

Suspicion gates:

- **GREEN-on-fat-target.** A doc with xref >= 5 OR offsets >= 30 OR ts-density >= 2 that
  returns GREEN with zero WRONG/MISLEADING findings (FIX mode: `status_after: GREEN` with
  `edits_applied: 0`) — treat with suspicion, queue for parent spot-check.
- **Edit-without-citation (FIX mode).** Any `edits_applied` line missing a Ghidra MCP call
  is a contract violation — the parent must re-verify that specific edit before trusting
  it, and flag it.
- **RED-needs-reinvestigate.** Flag separately when `needs_reinvestigate: true`, the primary
  function/address family is wrong, or a struct-table sample invalidates the whole table;
  routes to `/re-investigate`, not to a patch.

## Step 6: Reconcile (after all waves, in parent)

For each successful or partial slot, read the bounded summary carefully, then:

1. **Spot-check.** DETECT mode: pick one GREEN slot, read its `AUDIT_LOG.md` line, and
   independently verify one load-bearing claim against Ghidra/docs; if it disagrees, demote
   to flagged YELLOW in the report. FIX mode: additionally pick 1-2 corrected claims per
   wave (bias toward fat targets and edits with weak evidence), re-decompile the cited
   address yourself, and confirm the edit matches the binary. If a spot-checked edit is
   wrong, the correction introduced new drift — revert or fix it directly (the parent's one
   allowed edit), and treat that worker's whole batch as suspect (spot-check more of it).
2. **Cross-doc contradiction sweep.** For every WRONG/MISLEADING finding (and every FIX-mode
   correction), grep the rest of `docs/research/` for the same address, offset, INI key, or
   function. A sibling citing a different value is now a high-priority queue item — it
   likely shares the root cause. Do NOT auto-edit the sibling; it was not owned by any
   worker this run. Queue it for the next wave or a follow-up.
3. **Cross-slot/root-cause patterns.** Pattern-match across slots: TS-legacy mislabels,
   vtable slot shifts, offsets off by a constant, `param_1` `int` vs `int *` confusion,
   same-address collisions, function-entry vs interior-address confusion, stale Ghidra
   labels, repeated Rust-status drift. FIX mode: aggregate root-cause tags — a cluster is a
   finding bigger than any single fix:
   - Many `RTTI_LABEL_DRIFT` / vtable-slot fixes → the labeler was re-run; sweep all docs
     older than that rerun.
   - Many `STRUCT_FAMILY_CASCADE` (+constant offset) fixes → a base-class size changed;
     every doc citing that struct family is suspect.
   - Many `PARAM1_TYPE_MISREAD` fixes → a systemic authoring error; grep for the same.
   - Many `TS_LEGACY_AS_YR` fixes → run `--area shroud` / the relevant gate sweep next.
4. **AUDIT_LOG oscillation check.** Grep `AUDIT_LOG.md` history for docs oscillating
   RED→GREEN→RED or YELLOW→PATCHED→YELLOW — a recurring incomplete fix or an upstream label
   that keeps flipping; a fresh patch may not stick.
5. **Build the correction-fact / residual queue.** DETECT mode — correction facts from safe
   findings only: exact doc quote or section anchor, exact verified binary fact, binary
   evidence and whether the parent spot-checked it, grouped by target doc; mark
   `needs_reinvestigate` instead of patch-ready when the correction requires rebuilding a
   model, tracing a new caller chain, or resolving a broad YR-activity question. FIX mode —
   residual queue: UNVERIFIABLE findings left unpatched, structural-RED docs routed to
   `/re-investigate`, contradicting siblings, and suspicion-flagged GREENs; input for a
   follow-up run.
6. **Ghidra label-candidate queue (FIX mode).** Collect worker-reported
   rename/label/comment candidates, dedupe by address and kind, discard any candidate
   without direct binary evidence. If two candidates conflict, apply neither; report the
   conflict and route it to `/re-investigate` or a focused manual check.
7. If `--patch-plan` was requested (DETECT mode), add an ordered non-editing patch plan:
   docs to patch first, findings that need parent spot-check before patching, sibling docs
   to verify afterward, and single-doc `/verify-doc` commands to rerun. No replacement
   prose.

The parent does not edit audited docs during reconciliation (sole exception: the FIX-mode
spot-check revert in item 1).

## Step 6.5: Optional Serial Ghidra Label-Sync (FIX mode)

Run only when the user explicitly requested `--sync-ghidra-labels` or otherwise clearly
asked to update Ghidra names/labels/comments, and only after all workers have finished.

- Apply candidates **serially in the parent only**. Never ask workers to mutate Ghidra and
  never mutate Ghidra while a wave is running.
- Re-verify each candidate immediately before applying it: address, function/data identity,
  current label/comment, proposed label/comment, and the body/callsite/data-ref evidence.
- Apply only low-risk metadata fixes: clearly stale function names, misleading data labels,
  missing labels at already-known addresses, or comments that record verified behavior.
- Do not create functions, edit structs, change variable types, rename variables, or save
  broad analysis state unless the user explicitly requested that operation and the current
  tool supports it safely.
- After each mutation, read back the label/comment when the tool allows it and append an
  event to `.audit-swarm-claims.md` with address, old label/comment, new label/comment,
  tool call, and evidence source.
- If mutating Ghidra tools are unavailable, report the verified label-candidate queue and
  stop; do not simulate the mutation in docs.

## Step 7: Report

Return a concise report with:

- Status header: docs processed, failed count, GREEN/YELLOW/RED mix, contradiction count;
  FIX mode adds corrected count, RED→re-investigate count, total edits, status flips
  (e.g. `A YELLOW→GREEN, B RED→YELLOW`), and label candidates reported/applied/deferred.
- One line per slot: doc, status (FIX mode: status flip + edit count + root-cause tags),
  and key finding.
- Highest-impact WRONG/MISLEADING findings — FIX mode: highest-impact corrections with
  was→now, root cause, and verifying call — ranked by player visibility × downstream load.
- Docs NOT corrected / routed to `/re-investigate`, each with the recommended bounded
  target and "no edits applied".
- Cross-doc contradictions surfaced (NOT auto-edited), without resolving them unless the
  evidence was directly checked; queued siblings.
- Cross-wave root-cause patterns and the recommended sweep wave(s).
- Parent spot-checks: what was re-verified and the outcome, including any reverted edit.
- DETECT mode: correction-fact queue grouped by doc, with evidence and spot-check state;
  `--patch-plan` output if requested.
- FIX mode with label sync: applied renames (with the re-verification call), deferred
  conflicts, or "mutating tools unavailable — candidates listed only".
- Suspicion flags (GREEN-on-fat-target and whether the spot-check passed, oversize returns)
  and subagent contract violations.
- Failed slots and proposed single-slot retry commands.
- Verbatim `AUDIT_LOG.md` lines appended.
- Next steps for the user to choose from — FIX mode: review corrected docs with `git diff`
  before committing (this swarm never commits), run the recommended root-cause sweeps,
  route RED docs to `/re-investigate`, re-run failed slots.

Stop after the report. Do not start another swarm or follow-up wave the user didn't
authorize, do not run `/verify-doc` on contradictions, do not commit, do not edit further
docs unless the user asks. The user reviews `git diff` and decides what to run next.

## Anti-patterns — STOP

- **Editing a claim you could not verify from the binary this session.** No inference, no
  YRpp label, no sibling-doc value as the source of a correction. Flag it instead.
- **Patching surface claims on a structurally-RED doc.** Makes it look fixed while it still
  misleads. Stop and route to `/re-investigate`.
- **Two agents owning the same doc.** Dedupe before dispatch. Concurrent edits race and
  corrupt corrections.
- **Letting an agent edit (or audit) a sibling doc** it noticed was wrong. Only the owned
  doc. Queue the sibling for the parent.
- **Exceeding 3 concurrent agents.** Split into waves. The cap is a deliberate Ghidra-MCP
  and review-load bound.
- **A correction without a root cause.** "Wrong → right" with no *why* skips the diagnosis
  that tells you the fix is safe and predicts sibling rot.
- **A correction without an inline citation.** The fix is a new claim; it must carry the
  binary evidence for the next audit to re-check.
- **Skipping the parent spot-check of corrections.** A swarm that auto-fixes without the
  parent re-verifying any edit can launder 3 confident guesses into the corpus.
- **Mutating Ghidra "to clean up while there" from a worker.** Worker read-only is
  non-negotiable. Ghidra metadata cleanup happens only in the optional parent serial
  label-sync phase after all workers finish.
- **Committing/pushing the corrections.** Leave them in the worktree for `git diff` review.

## Rationalization table

| Excuse | Reality |
|---|---|
| "The right value is obvious from the sibling doc, I'll just copy it" | A sibling is a hypothesis too. Verify from the binary or don't edit. |
| "This doc is mostly wrong, I'll rewrite it while I'm here" | Structural-RED is a STOP. Route to /re-investigate; don't rebuild in a fix pass. |
| "30 agents would clear the corpus faster" | 3 is the cap. Wave the rest. Ghidra MCP and review load are the bound. |
| "I corrected it, no need for the parent to re-check" | Auto-fix without spot-check launders guesses into ground truth. Spot-check. |
| "The fix is small, skip the citation" | Small fixes rot fastest and are hardest to re-find. Cite every edit. |
| "Same offset is wrong in three docs, I'll fix all three from this agent" | One doc per agent. Queue the others; the parent dispatches them owned. |
| "It's probably an equivalent mechanism, close enough to leave/patch" | DRIFT by default. Equivalence needs proof. Flag, don't hand-wave. |
| "All GREEN, no edits — clean corpus" | GREEN-on-fat-target with zero edits is the suspicion gate. Spot-check it. |

## Fit Notes For This Project

- Prefer verifying/correcting canonical `*_GHIDRA_REPORT.md` files before derivative
  amendment files. Amendment docs are useful evidence, but they can hide whether the source
  report is now safe; a corrected source report may make an amendment file redundant — note
  it, don't delete it.
- This project's patch-cluster workflow is YELLOW audit → PATCH → targeted
  `/trace-action` or `/re-investigate` follow-up. DETECT mode preserves that
  cadence by outputting correction facts and anchors, not edits. FIX
  mode collapses the first two steps for *verified, isolated* drift while preserving the
  STOP for structural cases; the inline `(corrected YYYY-MM-DD: was X; binary shows Y via
  <call> — ROOT_CAUSE)` annotation is the durable trace per the project's "cite the
  verification call inline" rule.
- Re-investigation reports include `Coverage Ledger`, `Open Questions`, and
  `Implementation Handoff`. When auditing those docs, treat all three as load-bearing: a
  stale handoff or open-question state can mislead implementation just as much as a wrong
  offset.
- High-risk local failure patterns to bias ranking and diagnosis toward: `param_1`
  int-vs-int* (×4 offset bugs), function-entry vs interior-address citations, TS legacy
  treated as live YR, the same field named differently across siblings, RTTI/Ghidra label
  drift after labeler reruns, and Rust-status sections drifting after implementation work.
- For implementation-facing docs, a GREEN audit (or `status_after: GREEN`) is not enough if
  the Rust-status section was not checked. Either spot-check the cited Rust surface or mark
  Rust status as deliberately out of scope.
