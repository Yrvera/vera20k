---
name: verify-doc
description: >
  Audit an existing gamemd.exe research document against the live binary.
  Verifies every factual claim (addresses, offsets, field names, formulas,
  thresholds, state transitions, INI key mappings) in the doc by decompiling
  in Ghidra. Flags stale, wrong, or unverifiable claims without modifying
  the doc. Usage: '/verify-doc doc-path-or-name' or '/verify-doc' (prompts
  for selection).
---

# Verify Doc — Research Document vs Binary Audit

Audit the research document **$ARGUMENTS** against gamemd.exe. Every
factual claim in the doc must be re-verified from the live binary via
Ghidra MCP. Do NOT modify the document. Do NOT implement any Rust code.
The only output is an audit report.

**Why we audit research docs.** Research claims must remain exact and honest:
wrong thresholds, offsets, formulas, ordering, or active-YR reachability poison
future decisions. Correctness of the document is separate from whether the
finding is current retail-skirmish implementation priority.

<HARD-GATE — DETECTION ONLY>
This skill is read-only against the filesystem. It CANNOT change anything unless
the user explicitly asks to save the report or append an audit-log entry.

- Do NOT modify the research document being audited.
- Do NOT create a "corrected copy" of the doc at a new path.
- Do NOT write or edit any .rs file.
- Do NOT update INI files.
- Do NOT save the audit report to a file. The full report is chat output
  only, unless the user explicitly asks you to save it AFTER seeing it.
- Do NOT rename functions / add labels / save_program in Ghidra. Ghidra
  MCP is used only for read-only decompilation and memory inspection.

An `AUDIT_LOG.md` entry is optional and requires explicit user authorization; a
request to inspect or audit the document alone does not authorize the write.

If the user asks you to "fix" the doc or "apply the corrections", reply
that this skill only detects — they should review the report and run a
separate edit pass to apply any changes: `/audit` for a single doc, `/verify-doc-swarm --fix` for several.

Self-check before every tool call: "Does this tool modify state?" If yes and the
user did not explicitly authorize that exact output, do not call it.
</HARD-GATE>

## Iron Laws

```
GHIDRA LABELS ARE UNTRUSTED HINTS
```

Local Ghidra names, labels, comments, xref labels, and decompiler-assigned symbol
names are navigation aids only. This project may contain polluted or stale labels
from earlier scripts. Verify load-bearing identity claims from function body,
assembly/decompile, callsites, receiver/`this` pointer, argument flow, vtable slot
bytes, data references, and active-YR reachability. Prefer address plus verified
role over label name. If a doc's label is wrong or ambiguous, flag label drift as
a finding instead of treating the label as authority.

```
TINY MISMATCHES COUNT AS WRONG — "CLOSE ENOUGH" IS A FAIL
```

The whole purpose of this audit is to catch the tiny silent errors. Big
errors are easy — anyone re-reading the doc would notice the function
address is wrong. The mismatches we exist to catch are the ones that
look like nothing:

- `<` where the binary has `<=` (or vice versa)
- A clamp at `0xFF` documented as `0x100`
- A constant of `0x67` documented as `0x68`
- "Checks A then B" when the binary checks B then A
- "Returns 0 on error" when the binary returns -1
- A signed comparison documented as unsigned
- An off-by-one in a loop bound
- A field listed at offset `0x40` when it's actually `0x44`
- A flag bit position one off (`0x10` vs `0x20`)
- A "happens every tick" that's actually "every other tick"
- A "post-increment" that's actually a "pre-increment"
- A field updated **before** another field that the doc lists in the
  opposite order
- A default value the doc rounds to "~256" when the binary uses 255

**Every one of these is WRONG, not STALE, not "minor"**. Flag it.
Treat them with the same severity as a wrong function address — because
downstream consumers (`/write-plan`, implementation work) will copy the
doc's value verbatim. A 1-bit error here becomes a 1-bit error in the
engine.

When picking load-bearing claims in Step 1, **do not skip a claim because
it looks small**. The constants, clamps, and orderings are exactly where
docs rot fastest, because authors retype them from memory and skim past
them on review. They are first-class load-bearing claims — pick them.

When verifying in Step 2, **read literally, not approximately.** Compare
operators character-by-character, constants digit-by-digit, offsets
byte-by-byte. If your eye glosses past a constant because "yeah, that's
roughly right," go back and read the actual digits.

```
THE BINARY IS GROUND TRUTH. THE DOC IS A HYPOTHESIS.
```

Research docs accumulate drift. A doc that was correct when written can
be wrong now because: Ghidra re-analysis shifted addresses, annotations
were renamed, the author misread `param_1` type, or the finding was
TS-legacy misidentified as active YR behavior. Treat every claim as
unverified until re-checked from the binary.

```
TIBERIAN SUN ≠ YURI'S REVENGE
```

gamemd.exe hosts both engines. A claim that is technically true about
the binary ("FUN_X checks SpecialFlags bit 0x1000") may be **misleading**
if the doc implies the code is active in standard YR. Every active-in-YR
assertion must be re-verified: check default flag values, trace callers,
confirm the code path is reachable in a normal skirmish.

## Step 0 — Resolve the target doc

1. Parse `$ARGUMENTS`:
   - Absolute path → read directly
   - Relative path → try `docs/`, then `<main-checkout>/docs/research/`
   - Bare name (e.g., "garrison") → search both directories for matching `*GHIDRA_REPORT.md` / `*GARRISON*.md`
   - If target resolution is broad or ambiguous, use the research-index MCP first:

     ```text
     research_search(query="<doc-or-topic>", limit=10)
     research_validate(topic="<doc-or-topic>", limit=10)
     ```

     If the MCP tools are unavailable, use the equivalent repo-local CLI
     `search.py` and `validate.py` commands.

     If the exact system is known, add `--system <system>` (examples:
     `bridges`, `miner`, `skirmish-ui`, `chrono`). If that returns zero docs,
     rerun without `--system` and use the inferred exact system. The index helps
     choose the doc; the audit itself still verifies claims against the binary.
2. If no match or multiple matches, list candidates and ask the user to pick one.
3. **Check the audit log and moving-target window.** Read `<main-checkout>/docs/research/AUDIT_LOG.md`
   if it exists. If the target doc has an entry from the last ~30 days with status
   GREEN, mention it to the user and ask whether to re-audit or skip — recent
   GREEN audits rarely drift. YELLOW/RED entries are worth re-checking since the
   user may have patched the doc; just note the prior finding so you can confirm
   it was addressed. If it was audited today or within the last few hours, do not
   silently repeat the audit. Also inspect the target doc's mtime; if it changed in
   the last 6 hours, a parallel session may be rewriting it. Read the recent work
   first and get explicit confirmation before auditing a moving target.
4. Read the full doc. Confirm it's a research/RE document (not a plan, not a design spec).
   If it's not a research doc, say so and stop — this skill is for RE reports only.

## Step 1 — Identify the load-bearing claims

Don't try to enumerate every fact in the doc. Long research docs have
hundreds of restatements, examples, and derivative claims, and exhaustive
enumeration burns context without improving the audit. Instead, pick the
**~15-25 load-bearing claims** — the ones downstream consumers (other
docs, /write-plan, implementation code) will actually rely on. Output a
numbered list grouped by bucket below; every later verification step
refers back to these numbers.

**A claim is load-bearing if** any of these are true:
- It anchors other claims (a function entry address that pseudocode walks through; a struct base offset that field offsets are computed from)
- It's a public-interface fact (INI key → struct offset mapping, vtable slot dispatch)
- It's a behavioral primitive (state-machine transition condition, damage formula, threshold value)
- It's gating-active-in-YR (the doc claims a code path runs in standard YR — high-blast-radius if wrong)
- It contradicts a sibling doc (any time two docs make different claims about the same thing, both are load-bearing)

**A claim is NOT load-bearing** (skip it) if it's:
- A restated fact already verified upstream in the same doc
- A natural-language summary of pseudocode you'll verify directly
- A "what this code does at a high level" narrative
- A claim about the doc's own conclusions or confidence

### Buckets to draw load-bearing claims from

#### A. Binary-location claims
- Function addresses (e.g., `0x00429a90`, `FUN_00abcdef`)
- Function names assigned to addresses (e.g., "AStar_main_loop at 0x...")
- Vtable offsets and the methods they dispatch to
- Global/static data addresses (`DAT_...`) and what they hold
- Callers and callees asserted for a given function

#### B. Struct layout claims
- Class/struct name → byte offset → field name → type
- Struct total size
- Inheritance layout (base class offset within derived class)
- Flag/bitfield layouts and which bits mean what

#### C. Behavioral claims
- Formulas (damage, speed, ROT, ore growth, etc.)
- State machines (states, transitions, triggering conditions)
- Thresholds, clamps, min/max bounds
- Order of operations ("A before B", "checked in this sequence")
- Magic constants and their stated meaning

#### D. INI mapping claims
- "[Section] Key= maps to struct offset 0x...."
- Default values claimed for INI keys
- Claims about whether a key is read, ignored, or superseded

#### E. YR-activity claims
- "Active in YR: Yes/No/Conditional"
- Flag gating ("behind SpecialFlags & 0x1000")
- Default values for gating flags in standard YR

#### F. Cross-reference claims
- References to other docs ("see X_GHIDRA_REPORT.md for Y")
- References to Rust files ("implemented in src/sim/..." — Rust claims are
  lower priority but should be spot-checked since they rot fastest)

**Output of Step 1:** a numbered list grouped by bucket, ~15-25 entries
total. If the doc is so terse that fewer than ~10 load-bearing claims
exist, audit them all and note in the report that the doc's surface area
is small. If the doc is so dense that ~25 doesn't cover the load-bearing
claims, pick the ~25 highest-impact ones and explicitly flag in the
report which areas you did NOT cover.

## Step 2 — Verify against the binary (THE CORE)

Work through the numbered claim list. Use Ghidra MCP as the primary tool.
Verification order: **A (addresses) → B (struct layouts) → C (behavior) →
D (INI mapping) → E (YR activity) → F (cross-refs)**. Earlier buckets
invalidate later ones — if the address is wrong, every behavioral claim
attached to it is moot.

**If Ghidra MCP is unreachable** (connection-refused, empty `list_instances`, or "No program
loaded"), invoke the `/ghidra-up` skill before auditing anything. Do not classify claims as
UNVERIFIABLE because the bridge is down - that produces a false RED.

### Per-claim verification protocol

For each claim:

1. **State what you're checking** (one sentence, referencing claim number).
2. **Go to the binary.** Decompile the function, read the memory region,
   or dump the struct layout — whatever the claim demands.
3. **Compare literally.** Exact addresses, exact offsets, exact operators
   (`>` vs `>=`, `<` vs `<=`), exact constants.
4. **Classify the result:**
   - **CONFIRMED** — binary matches the doc exactly
   - **WRONG** — binary contradicts the doc (include actual value)
   - **STALE** — doc was correct for an earlier Ghidra state but
     addresses/labels have since shifted (note the new address)
   - **UNVERIFIABLE** — cannot resolve from binary alone (e.g., doc claims
     a design intent that leaves no runtime signature)
   - **MISLEADING** — technically accurate but omits critical context
     (most commonly: TS-legacy code presented as YR behavior)

### Ghidra pitfalls to re-check (the ones that produce silently-wrong docs)

- **`param_1` type** — `int` means byte offsets; `int *` means index × 4.
  This is the #1 source of wrong struct offsets. Re-verify type for every
  function whose offsets the doc cites.
- **AnimTypeClass uses `int *`** — doc may have `int *` offsets multiplied
  correctly OR raw. Check before accepting.
- **`DAT_` globals** — if the doc cites a value from a data table, re-read
  memory at that address with `read_memory`. Values may differ from what
  the decompiler's comment suggested.
- **Vtable dispatch** — if the doc names a method called "virtually", confirm
  the concrete implementation at the claimed vtable slot. Vtable indices
  shift when RTTI labeler reruns. Follow the full protocol below.
- **Signed vs unsigned** — a claim like "clamped to 0-255" is different from
  "sign-extended to int". Check the MOVZX vs MOVSX.

### Vtable claim verification

Every claim that slot N of class C's vtable is method M requires all three:

1. **Owner:** read the CompleteObjectLocator pointer at `vtable-4`, then the
   TypeDescriptor at `COL+0x0C`, then its mangled name at `TypeDescriptor+0x08`.
   It must name class C; a Ghidra display label is not evidence.
2. **Slot:** read the function pointer at `vtable + N*4`. gamemd is 32-bit, so
   slot 17 is byte offset `0x44`, not `0x11`.
3. **Body:** decompile the resulting function address and verify that its body
   implements M's claimed behavior.

Cite all three calls inline. Owner or body mismatch is `WRONG`; `__purecall` in
a construction snapshot is `STALE` only when the live vtable is independently
verified; an unreachable TypeDescriptor is `UNVERIFIABLE`. Mangled RTTI identity
wins over a conflicting display label.

### TS-vs-YR re-check (mandatory for every E-bucket claim)

For every "Active in YR" assertion:

1. **Find the gating flag.** Re-trace in the binary — don't trust the doc
   to have identified the right flag.
2. **Read the default value.** Check `ini/rulesmd.ini`, `ini/rules.ini`,
   and any hardcoded default in the binary. TS defaults still appear in
   the binary but YR ships different defaults.
3. **Trace callers.** Is the code path reachable from standard YR game
   setup? Some functions are only called by TS-era mission triggers that
   no YR map uses.
4. **Flag misidentification.** Field/flag names may be TS-era. "Tiberium"
   on a warhead gates veins, not ore. Doc claims that trust the name are
   suspect.

### Struct layout sanity check

When the doc publishes a struct layout table:

1. Pick 3-5 fields at random (not just the first few — authors often get
   early offsets right and drift later).
2. Find a function that reads/writes each picked field. Decompile it.
   Confirm the offset in the decompilation matches the doc's table.
3. Check the struct's total size claim against an allocation site
   (constructor call, array stride).

If any random pick is wrong, treat the entire table as SUSPECT and
re-verify all fields the doc's consumers actually care about.

## Step 3 — Cross-reference pass

Skim the doc one more time for claims you might have missed because they
weren't phrased as obvious facts:

- "Obviously the engine must do X" — verify it does.
- "We infer that..." — re-classify as UNVERIFIABLE or verify it.
- Pseudocode blocks — compare each line to the decompilation.
- Tables with "Purpose" columns — the offset+type may be right but the
  purpose narrative can be fabricated. Spot-check.

## Step 4 — Cross-doc consistency (optional, if time permits)

If the doc references sibling docs (e.g., "See ARMOR_TYPES_GHIDRA_REPORT.md"),
check the sibling for contradictions. Two docs disagreeing is worth flagging
even if both are internally consistent — it means at least one is wrong.

Do NOT expand the audit into those sibling docs. Just note the contradiction
and which doc the binary supports.

## Step 5 — Report

Structure:

```
## Doc Audit: <doc path>

**Doc status:** GREEN / YELLOW / RED
  - GREEN: 0 WRONG, 0 MISLEADING, <5% UNVERIFIABLE — safe to rely on
  - YELLOW: minor staleness or a few wrong offsets — usable with flagged caveats
  - RED: structural errors (wrong struct base address, TS-legacy framed as
    YR behavior, wrong primary function) — consumers should not trust this doc
    until rewritten

### Summary
- Load-bearing claims audited: N (out of the ~15-25 selected in Step 1)
- CONFIRMED: N
- WRONG: N
- STALE: N
- UNVERIFIABLE: N
- MISLEADING: N
- (If applicable) Areas NOT covered: <one-line list of long pseudocode blocks
  / sub-systems you deliberately scoped out — be honest, don't pretend
  uncovered areas were verified>

Be honest about scope. If you only got through 12 of the 22 claims you
listed, the count is 12 — don't fabricate. The user reads this number
to decide whether to trust the GREEN/YELLOW/RED status.

### Wrong Claims
[For each WRONG:]
- **Claim #N (section "..."):** doc says X
- **Binary shows:** Y (address/offset/evidence)
- **Impact:** what downstream code/decisions would this mislead?

### Stale Claims
[For each STALE:]
- **Claim #N:** address/label in doc is `OLD`; current Ghidra state shows `NEW`
- **Still semantically correct?** yes/no
- **Suggested update:** one-line pointer (not a rewrite)

### Misleading Claims (TS-legacy framed as YR)
[For each MISLEADING:]
- **Claim #N:** doc implies behavior is active in YR
- **Actual gating:** flag / default value / caller analysis
- **Real YR status:** inactive / conditional / degenerate

### Unverifiable Claims
[For each UNVERIFIABLE:]
- **Claim #N:** what the doc asserts
- **Why it can't be verified from binary:** (design intent, author inference,
  historical context, etc.)
- **Treat as:** informational / remove / flag as inference

### Contradictions With Sibling Docs
[If any — list the conflict and which doc the binary supports.]

### Verdict
One paragraph: is this doc safe to use as input for /write-plan
or new implementation work? If RED, what's the shortest
path to making it safe (targeted rewrite vs full /re-investigate)?
```

### Rules for the report

- **Lead with the status and WRONG claims.** Don't bury errors under
  tables of confirmations.
- **No match tables.** Don't list every claim that checked out.
- **Every WRONG / STALE / MISLEADING needs binary evidence** — an address,
  an offset, a decompiled line. "Seems wrong" is not a finding.
- **No fixes.** Do not propose doc rewrites. The user decides whether to
  re-investigate, patch, or deprecate the doc.
- **Quote the doc verbatim** when flagging a specific claim, so the user
  can Ctrl-F to the exact line they need to edit.

## Anti-patterns

- **Summarizing the doc instead of auditing it.** If your report restates
  what the doc says without checking the binary, you did Step 1 and skipped
  Step 2. Start over.
- **Accepting addresses without decompiling.** An address in a doc is a
  claim, not a fact. Decompile it.
- **Trusting the struct table's first few entries.** Authors get the head
  right and the tail wrong. Random-sample deeper into the table.
- **Flagging everything as UNVERIFIABLE.** If most claims land in this bucket,
  you're not using Ghidra hard enough. Re-read Step 2.
- **Turning the audit into a re-investigation.** If the doc is RED, STOP
  and recommend `/re-investigate`. Don't silently rebuild the doc in the
  audit report.
- **Ignoring the YR/TS distinction.** Every behavioral claim the doc makes
  needs an active-in-YR check, even if the doc didn't explicitly frame it
  that way.
- **Fabricating coverage counts.** If you didn't enumerate the claim list
  in Step 1, don't make up a number for "Total claims audited". Either go
  back and enumerate, or write "smoke check" in the report and mark the
  status conservatively.
- **Downgrading a tiny mismatch to "STALE" or "minor".** An off-by-one is
  WRONG, not stale. A clamp value off by 1 is WRONG, not minor. STALE means
  "the address shifted but the semantics still hold". A wrong constant or
  wrong operator is a semantic change — that's WRONG.
- **Skimming past constants because they "look right".** The pattern is:
  eye sees `0xFF`, brain reads "byte max", check passes. Then later the
  binary turns out to use `0x7F` (signed byte max) and a parity bug ships.
  Read the digits. Every time.

## Step 6 — Optional audit-log entry

After delivering the report in chat, append one line to
`<main-checkout>/docs/research/AUDIT_LOG.md` only if the user
explicitly requested logging. Otherwise stop after the chat report.

### Line format

```
- **YYYY-MM-DD** — `<doc-filename>` — STATUS — <one-line key finding>
```

Examples:
```
- **2026-05-05** — `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` — YELLOW — slot-18 vtable label wrong; teleport-vs-drive decision oversimplified (BuildingTypeClass+0x16B3 = DockUnload, gating omitted)
- **2026-05-05** — `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` — GREEN — CLSID_TeleportLocomotion byte error ({4A582790} → {4A582747}); FootClass__PerCellProcess mislabeled as "dock state transition"
- **2026-05-05** — `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` — GREEN — 0x4D6AA0 mislabeled as Mission_Harvest (actually Mission_AreaGuard)
```

If the doc is GREEN with no notable findings, write `clean` for the
finding field.

### File creation

If `AUDIT_LOG.md` doesn't exist yet, create it with this header and a
blank line, then add your entry:

```
# Verify-Doc Audit Log

A running log of /verify-doc runs. One line per audit. See chat history
or git log for the full reports.

```

### Constraints

- One line per audit. Don't paste the full report into the log.
- Don't edit prior entries. Append-only.
- Don't write any other file. The full audit report stays in chat.

### Why this matters

Chat reports vanish after compaction. Without the log, the next session
auditing the same doc cluster will rediscover the same findings from
scratch, and cross-doc contradictions found in earlier audits get lost.
The log is a 30-second cost that pays back the next time you (or
another session) needs to know "has this doc been verified recently?"
