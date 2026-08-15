---
name: re-investigate
description: "Use when studying, researching, or reverse-engineering a gamemd.exe system before implementation. Produces a verified research document with an implementation handoff, but never writes Rust code. Usage: '/re-investigate chrono miner teleport' or '/re-investigate garrison fire logic' or '/re-investigate power drain formula'"
---

# RE Investigate — Structured Reverse Engineering

Research `$ARGUMENTS` in gamemd.exe using a structured, evidence-based process. Produce a verified research document. Do NOT write any Rust code.

<HARD-GATE>
Do NOT implement anything. Do NOT modify any .rs file. Do NOT write Rust code or
patches. The research report MUST include an implementation handoff that names
affected Rust surfaces and acceptance scenarios, but it must not contain patch
text or code snippets. The ONLY filesystem artifact is a research document saved
to `<main-checkout>/docs/research/`. Report Ghidra annotation
candidates, but do not synchronize them unless the invocation includes
`--sync-ghidra-labels` or the user directly requests it. If you catch yourself writing
Rust code, STOP and delete it. This applies regardless of how "simple" the
implementation seems.
</HARD-GATE>

**What we are extracting and why.** Research truth remains exact: record scoped
active-gamemd branches, constants, field reads/writes, ordering, rounding, RNG,
assets, draw/audio timing, and uncertainty precisely. Do not label an
approximation as verified. The implementation handoff must separately classify
which findings are milestone-blocking, compounding, exactification residuals,
or unknown-risk for ordinary stock skirmish.

## Iron Laws

```
CONTRADICTIONS ARE PRIMARY EVIDENCE
```

If a screenshot, runtime result, test failure, trace, asset dump, or user
observation contradicts the current explanation, assume the explanation is
incomplete. Do not defend the model and do not implement from it. Reopen the
investigation, name the contradiction in the Open Questions Log, and resolve it
with binary, asset, INI, runtime, or source evidence.

```
TRACE FROM THE OWNER, NOT THE HELPER
```

Start from the behavior owner for the target scenario before trusting a helper:
paint/message/render handlers for UI, tick/update/action pipelines for gameplay,
event trigger paths for audio, actual load/apply paths for parsing, and input
dispatch paths for commands. A helper only proves behavior after you prove the
target owner calls it under the target conditions.

```
PROVE TARGET ACTIVATION
```

Code existing in gamemd.exe is not enough. For every claimed behavior, prove it
is active in the target game, mode, dialog, map, unit, house, difficulty, or
setting. Record the enabling flag/key/case, its default value in standard YR
where applicable, and whether the target scenario actually reaches it.

```
INSPECT THE VARIANT SET
```

Before choosing a conclusion from one variant, inspect the relevant variant set:
all active SHP frames, YR `*md` overrides before base INI fallback, dialog IDs or
switch cases, difficulty/game-mode branches, house/faction overrides, map
overrides, and optional flags. If the full set is too large, state the bounded
subset and mark the rest `deferred` instead of generalizing.

```
STALE DOCS ARE WARNINGS, NOT BLOCKERS
```

Older reports and synthesis docs are maps to evidence, not final authority. When
they conflict with binary, runtime, asset, INI, screenshot, or current Rust
evidence, the primary evidence wins. Record the stale claim and update or hand
off the correction instead of letting the old prose block the investigation.

```
GHIDRA LABELS ARE HINTS, NOT TRUTH
```

Local Ghidra names, labels, comments, xref labels, and decompiler-assigned symbol
names are navigation aids only. This project may contain polluted or stale labels
from earlier scripts. For every load-bearing claim, verify from the function body,
assembly/decompile, callsites, receiver/`this` pointer, argument flow, vtable slot
bytes, data references, and active-YR reachability. Prefer address plus verified
role over label name. If a label is wrong or ambiguous, record the label drift and
do not let the old name drive architecture or implementation.

Every vtable claim requires a CompleteObjectLocator walk (`vtable-4` →
`COL+0x0C` → TypeDescriptor mangled name), a slot read at `vtable+N*4`, and a
decompile of the resulting function body. Cite all three inline. Ghidra display
labels are not identity evidence; if the owner or body is unproven, mark the
claim `UNCHECKED` rather than inferring it from a name.

```
THE TINY DETAILS ARE THE WHOLE POINT — NOTHING ELSE COMES CLOSE
```

**By far the most important output of this skill is the tiny details you would
normally dismiss as not worth writing down.** Not the high-level summary. Not the
algorithm overview. Not the "main flow." Those are the easy parts and they are not
why we are here.

We are here for:

- The `+1` after a multiply that nobody would notice
- The `if (x == 0) return` that handles one specific frame
- The constant `0x67` that turns out to be the cell size in a different unit
- The order in which two writes to the same field happen in the same tick
- The flag bit that's checked once and only matters when another flag is set
- The early-out condition three calls deep in a helper
- The clamp that saturates at `0xFF` instead of `0x100`
- The fact that the function reads the field BEFORE the caller has updated it
- The `<` that should obviously be `<=` and isn't
- The branch that looks dead but writes a side-effect read elsewhere
- The default value a field holds when nothing has touched it yet
- The signed-vs-unsigned comparison that flips behavior at the boundary

If your report describes the system at the level of "it does X, then Y, then Z,"
**you have failed**, no matter how clean the prose is. The summary is the cheap
part. The summary is what a 10-minute skim of the function gives you. We are paying
the cost of a full investigation **specifically to surface the things a skim would
miss**. If the report does not contain at least a dozen findings of the form "this
specific tiny thing, in this specific place, with this specific value, and here's
why it matters" — keep digging. You are not done.

A reader of the final report should think: *"I would never have noticed any of this
on my own."* That feeling is the success criterion. If they instead think *"yeah,
that's roughly what I'd have guessed,"* the report is worthless — it has told them
nothing they didn't already know.

**Default to over-recording.** When you are unsure whether a detail is worth noting,
the answer is yes. The cost of an extra noted constant or branch is one line in the
report. The cost of a missing one is a parity bug nobody can find later. Err
massively toward over-recording. Tiny details cost almost nothing to document and
are almost impossible to recover later without redoing the whole investigation.

```
NO CLAIMS WITHOUT BINARY EVIDENCE
```

Every finding must state its evidence source and confidence level. "I think it works like X" is not a finding — it's a guess. Guesses go in an "Open Questions" section, not in findings.

```
DETAILS COMPOUND — THE INVISIBLE ONES MATTER MOST
```

Most parity-breaking details are **not directly visible** — a one-tick delay before
a turret starts rotating, an off-by-one in a damage clamp, a flag that only matters
on one frame of an animation. These don't jump out to the player, but they compound.
Two "invisible" 5%-off details interacting produce a behavior the player feels but
can't articulate. **If a detail exists in the binary, assume it matters until proven
otherwise.** Document it. Don't filter findings by "will this be visible?" — filter
by "is this what the binary actually does?"

The temptation to skip a detail because "it probably doesn't matter" is the single
biggest failure mode of this skill. Resist it every time. The whole point of the
skill is to capture exactly the details that "probably don't matter" — because in
aggregate, they're what separates exact byte/pixel parity from drift.

This skill exists because details are hard to catch. That is the job. If your report
lists the happy path and skips the edge cases, constants, clamps, and off-by-ones,
you have not done the job — regardless of how clean the report looks.

```
DECLARE THE INVESTIGATION MODE BEFORE YOU DIG
```

Half-investigating a system is worse than not investigating it when the output
pretends to be complete. For an `exhaustive-slice`, decompile every relevant
function, examine every branch, explain every magic number, check every flag
default, and trace every edge case. For a `coverage-map`, do not pretend to
complete the system; make the unknowns and follow-up queue the primary output.
"Ran out of steam" is not a stopping condition. Either drain the slice, or label
the report as partial/coverage-map and show exactly what remains.

There are two valid modes:

- **coverage-map**: use for broad systems, vague user prompts, global architecture,
  tick loops, "overall" behavior, or any topic where the first honest job is to
  discover entry points and unknowns. The output is a coverage map, not a completed
  proof. It must say which areas were verified, touched, not touched, and deferred.
- **exhaustive-slice**: use only for a bounded subsystem small enough to drain every
  open question in the current session. The output may claim completeness only for
  that exact slice, never for the parent system.

Default to **coverage-map** when the scope is broad. Do not ask the user to narrow
unless the request cannot be made useful; instead, make the mode explicit and
produce an honest map of what remains. Never use filenames, headings, summaries, or
phrases like "completion", "fully investigated", "entire system", or "all covered"
unless the Open Questions Log is drained and the Coverage Ledger has no material
unknowns for the stated slice.

```
EXHAUSTION, NOT COVERAGE — THE OPEN QUESTIONS LOG IS THE GATE
```

The single biggest failure mode of this skill is declaring "done" because the
**topic feels covered** — the main flow is described, the report reads well,
the summary makes sense. That is coverage, not exhaustion. Coverage is when a
reader cannot tell what's missing. Exhaustion is when **you** cannot name what's
missing — because you've maintained an explicit list of unknowns and driven it
to zero.

You MUST maintain an **Open Questions Log** throughout the investigation. It is
seeded in Step 0.5, grown in Step 2 every time you encounter a new unknown, and
drained in Step 3. The investigation is not complete until **every entry in the
log is either resolved with evidence or explicitly deferred with a stated reason
and category** (e.g., "deferred — requires runtime debugger to observe"). A
non-empty log with un-deferred items means you are not done — regardless of how
many functions you've decompiled or how polished the report looks.

"Covered the topic" is not a stopping criterion. "Open Questions Log has zero
un-deferred items AND I added no new items in my last full pass" is. This is the
single check that converts the skill from breadth-by-feel to breadth-by-construction.

```
TIBERIAN SUN ≠ YURI'S REVENGE
```

gamemd.exe is the engine for BOTH Tiberian Sun and Yuri's Revenge. It inherited a massive
TS codebase. Many systems, fields, and code paths exist in the binary but are **disabled,
repurposed, or irrelevant** in YR. Before reporting any finding:

1. **Verify the code is active in YR** — not gated behind a TS-only flag, not a dead code
   path, not overridden by a YR-specific implementation.
2. **Don't trust field/flag names at face value.** TS-era names persist in the binary but
   may gate something completely different in YR context. Example: `Tiberium=yes` on a
   WarheadTypeClass does NOT gate ore destruction — it only gates **vein** destruction.
   The name misleads because "Tiberium" is TS terminology. Always trace what the flag
   actually controls in the code, not what the name implies.
3. **Check default values.** A feature may exist in code but default to off in YR
   (e.g., `FogOfWar` defaults false — it's TS legacy).
4. **When documenting, always state "Active in YR: Yes/No/Conditional"** for each finding.
   If conditional, state the exact condition and its default value in a standard YR skirmish.

## Step 0 — Scope, prior work, and duplication check

1. Parse `$ARGUMENTS` into a specific system, mechanic, or class to investigate.

2. Classify the request as `coverage-map` or `exhaustive-slice` and state that mode
   to the user before proceeding.

   Use `coverage-map` for broad prompts such as "global timing", "the combat
   system", "movement overall", "find disparities", "entire system", or any request
   whose entry points are not already bounded. The deliverable is a coverage ledger
   and a follow-up investigation queue.

   Use `exhaustive-slice` only when the target is narrow enough to resolve every
   open question in one pass, such as one helper function, one INI key's read/use
   chain, one state transition, or one specific gameplay action.

3. If `$ARGUMENTS` is so broad that even a useful coverage map cannot be scoped,
   ask the user to narrow scope before proceeding.

4. **Plan-path check (do this first, before anything else).** If `$ARGUMENTS` references an investigation plan at `docs/plans/YYYY-MM-DD-<topic>-investigation-plan.md`, OR the wording matches a recent plan in `docs/plans/`:
   - Read the plan's frontmatter and locate the **Expected Output** path (typically `<main-checkout>/docs/research/<TOPIC>_GHIDRA_REPORT.md`).
   - `ls` that path. If the file exists AND its mtime is newer than the plan's mtime, a parallel/earlier session already executed the plan. Go to step 5.

5. Use the research-index MCP before broad manual doc search:

   ```text
   research_brief(query="<topic>", limit=8)
   ```

   If MCP is unavailable, use the equivalent repo-local `brief.py` CLI.

   If the exact system is known, add `--system <system>` (examples: `bridges`,
   `miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun
   without `--system`, then rerun with the inferred exact system. Use
   `validate.py`, `map.py`, `search.py`, and `graph.py evidence` to identify
   existing verified docs, stale warnings, and likely Rust touchpoints. Treat the
   index as navigation; live Ghidra/binary evidence still decides new findings.

6. Check for existing research — search BOTH locations:
   - `docs/` (in-repo)
   - `<main-checkout>/docs/research/` (ignored in-repo research corpus)

   Sort matches by mtime descending. Read any matching report modified in the
   last 6 hours before proceeding; it may be a parallel session's output.

7. **Decide before starting the investigation work.** Apply this table and tell the user which row fires:

   | Prior state | Action |
   |-------------|--------|
    | No relevant research exists | Proceed to Step 1 — full investigation. |
    | Matching report modified in the last 6 hours | **STOP.** Read it fully, compare it to the requested scope, and report covered/partial/open before any duplicate work. |
   | Partial / LOW–MEDIUM confidence report exists | Proceed to Step 1 — scope to **gaps + verification only**, do not re-cover ground. |
   | Recent HIGH-confidence report covers the topic | **STOP.** Recommend `/verify-doc <report>` for audit, or `/re-investigate <topic> --extend <specific gap>` if the user wants a targeted follow-up. Do not silently re-execute. |
   | **Expected-output file from a plan already exists and is newer than the plan** | **STOP.** A parallel session executed it. Read the report, diff against the plan's Success Criteria, and report to the user: "covered / partial / open" per criterion. Do not re-execute. |
   | Recent report has explicit open questions or "Phase 2/3 deferred" markers | Proceed to Step 1 — scope to **those specific gaps**, cite the parent report. |

   When proceeding, state which row applied and what's in-scope vs. out-of-scope.

8. If proceeding, read any existing report fully before Step 1. Your job is to **extend or verify** it, not duplicate it.

## Step 1 — Parallel research

Gather these inputs as three local workstreams. Run independent filesystem
searches in parallel when practical. Use internal Codex subagents only when the
active session instructions permit; never create a user-owned task/thread without
an explicit request. Each input returns structured findings.

When subagents are used, require each return to stay at 150 lines or fewer, name
every artifact path, and include at most five load-bearing verified facts with
one-line evidence. Raw decompilation, assembly, INI dumps, source bodies, and full
struct tables belong in artifacts, not the return.

**Workstream A — Doc search:**
- Search `docs/` and `<main-checkout>/docs/research/` for ALL reports related to the topic. Search broadly — related systems, not just exact name matches.
- For each relevant report: extract verified findings (formulas, state machines, offsets, constants, conditions).
- Note any conflicting claims between reports.
- Return: list of findings with source file for each.

**Workstream B — INI data extraction:**
- Grep `ini/rulesmd.ini` and `ini/artmd.ini` for all keys related to the target system.
- Also check `ini/rules.ini` and `ini/art.ini` for base RA2 values (YR `*md` overrides take priority).
- For each relevant key: extract the key name, default value, section it appears in, and what it likely controls.
- Return: structured list of INI keys with values and sections.

**Workstream C — Rust implementation scan:**
- Search `src/` for the current Rust implementation of this system.
- Extract: what's implemented, what state machine states exist, what formulas are used, what INI keys are read.
- Do NOT judge correctness — just document what exists.
- Return: file paths, function names, current logic summary.

## Step 1.5 — Seed the Open Questions Log

Before touching Ghidra, write the initial Open Questions Log. This is the
investigation's spine. Maintain it with Codex `update_plan` when that is useful,
or as a working `<TOPIC>_OPEN_QUESTIONS.md` file in your scratch space,
whichever keeps it visible across tool calls. The final state lives in Section 7
of the report.

Start the log with the selected investigation mode:

```
[MODE] coverage-map | exhaustive-slice
[SCOPE] exact slice being claimed
[NON-SCOPE] related areas intentionally not claimed
```

If the selected mode is `coverage-map`, deferred entries are expected, but they
must become the follow-up queue. If the selected mode is `exhaustive-slice`, any
material deferred entry downgrades the final report to partial/coverage-map.

**Required seed entries (every investigation must include all of these):**

1. **Entry points** — every function/vtable slot/string xref that could plausibly
   enter this system. Each one becomes an open question: "Is FUN_XXXXXX actually
   on a live path in YR? What does it do?"
2. **Every INI key from Workstream B** — each one becomes: "Where is this key read in
   the binary? What does it gate? What's the default-vs-override behavior?"
3. **Every existing claim from Workstream A** — each prior-doc finding becomes: "Is
   this still true in the current binary? What's the address that proves it?"
4. **Every Rust function from Workstream C** — each one becomes: "Does this match
   gamemd's behavior, and what specific output does it produce vs the binary?"
5. **Tick-cycle integration** — "When in the tick does this system run? What
   reads its output? What writes to its inputs?"
6. **TS legacy filter** — "Which fields/flags/branches in this system are TS
   holdovers vs. live in YR? What's the default for each gate?"
7. **Edge / corner cases** — explicitly enumerate at least: null pointer, zero
   value, max value, empty container, first-tick, last-tick, paused, replay/save
   restore. Each is an open question until proven irrelevant or handled.

**Format for each entry** (keep terse — one line per question):

```
[OPEN] <id> — <question>
[RESOLVED] <id> — <one-line answer> (evidence: <address / doc / INI key>)
[DEFERRED] <id> — <one-line reason> (category: out-of-scope | needs-runtime-debugger | requires-different-system-context | bounded-cost-too-high)
```

A seed log with fewer than ~15 entries for a non-trivial system means you have
not thought hard enough about what could be unknown. Add more before proceeding.

## Step 2 — Ghidra deep dive (THE CORE — spend most of your time here)

After collecting parallel results, go to Ghidra MCP for the authoritative binary analysis.
This step is the entire point of the investigation. Do NOT shortcut it.

Keep evidence collection read-only while any parallel reader is active. A
child/worker never mutates Ghidra; it reports candidates for the parent with address,
current metadata, proposed mutation, and the live binary calls that prove it. This
skill stops at that candidate ledger by default. An explicitly authorized root sync
follows ENGINE.md after every reader has stopped.

**Time budget:** Spend at least 70% of your total effort in this step. If you've only
decompiled 2-3 functions, you almost certainly haven't gone deep enough. A typical
investigation should decompile **5-15+ functions** depending on system complexity.

**Finding the entry point:**
1. Search for relevant string constants (INI key names, error messages, class names)
2. Follow xrefs from strings into the functions that read/use them
3. If searching for a class method, find the class vtable first, then locate the method
4. If Ghidra missed a function boundary, do not create it under metadata-sync
   authorization. Inspect bytes/reachable callers read-only or record the
   boundary uncertainty unless the user explicitly authorized function creation.

**For each function you decompile:**
1. Note the address and check `param_1` type:
   - `param_1` is `int` → offsets like `*(param_1 + 0x98)` are direct byte offsets
   - `param_1` is `int *` → `param_1[0xac]` means byte offset = `0xac * 4`
   - `*(type *)((int)param_1 + 0x372)` is always a direct byte offset regardless
2. Extract: conditions, formulas, constants, state transitions, branching logic
3. **Follow ALL callees** — decompile every helper function, not just "non-trivial" ones.
   A 5-line helper can contain the critical detail that makes the whole system make sense.
   The only exception is obvious utility functions (memset, strlen, etc.)
4. Check xrefs to understand who calls this function and when
5. **Read the branch you DIDN'T expect.** When you see an if/else or switch, don't just
   follow the "normal" path — examine the error path, the edge case path, the fallback.
   These are where the important behavioral details live.
6. Record an annotation candidate only when the evidence meets ENGINE.md's label or
   synthetic-xref certainty gate. Otherwise explicitly record `annotation: none`.

**Depth requirements — follow the chain:**
- When you find the primary function, decompile it fully.
- For each callee in that function: decompile it too.
- For important callees (state machines, formulas, lookups): go one MORE level deep.
- For the primary function's callers: decompile at least the immediate caller to understand
  when/how this system is invoked and what context is passed in.
- **If you see a vtable dispatch** (call through a function pointer), resolve which concrete
  method is being called. Don't just note "calls vtable+0x48" — find the actual implementation.
- **If you see a global variable or static array**, inspect its contents or initialization.
  These often contain lookup tables, default values, or state that changes behavior.

**Branching rule — every unknown spawns a new Open Questions entry.**

This is mandatory and load-bearing. The skill cannot be exploratory if you do
not externalize unknowns the instant you encounter them. Every time you hit
ANY of the following while decompiling, **add a new `[OPEN]` entry to the log
before continuing the current function**:

- A **callee** you haven't decompiled yet (even "obvious" helpers — log it,
  decide later)
- A **caller** of the current function you haven't traced
- A **struct field** read or written at an offset you haven't documented
- A **global variable** or static array referenced
- A **flag bit** checked or set (each bit is its own entry unless already covered)
- A **vtable slot** dispatched through — log "which concrete method is bound here?"
- A **constant** whose meaning is not immediately obvious
- A **branch** (if/switch) where you haven't yet examined both/all paths
- A **string xref** the function uses
- An **INI key** read by this function that wasn't in the Workstream B list

You may not close a parent open question until **every child entry it spawned
is resolved or explicitly deferred**. A function is not "done" because you
read it top to bottom — it is done when its open questions are drained.

**Pass discipline.** Work in passes. A pass = decompile-until-no-new-callees-or-
unknowns-pop-up. Between passes, re-read the Open Questions Log top to bottom
and pick the next `[OPEN]` item. The investigation may not exit Step 2 until
**you have completed a full pass that added zero new entries to the log**.
That zero-add pass is the empirical signal of exhaustion. If your last pass
added even one entry, do another pass.

**Detail extraction — the things that matter most:**

The details below are the *primary deliverable* of the investigation. A report that
describes the overall algorithm but misses the constants, clamps, orderings, and edge
cases is a failed report — even if the summary reads well. Many of these details are
invisible at a glance but load-bearing for parity. Treat every one as non-optional.

- **Magic numbers**: Every constant in the code (0x100, 0x600, 256, etc.) — document what
  it represents. Don't just note the value; figure out WHY that value.
- **Bit flags and masks**: When you see `& 0x1F` or `| 0x40`, document each bit's meaning.
  Check if other functions use the same flags — build the complete flag set.
- **Timing and ordering**: Note the exact order of operations. "A happens before B" is a
  finding. "A and B happen but I didn't check the order" is incomplete.
- **Edge cases**: What happens when a value is 0? When a pointer is null? When an array
  index is out of bounds? The original engine has specific behavior for these — document it.
- **Clamping and bounds**: Does the code clamp values? What are the min/max? Is it
  saturating or wrapping? These details affect gameplay.
- **Off-by-ones and inclusive/exclusive ranges**: `<` vs `<=`, `i < n` vs `i <= n`, ranges
  that include or exclude their endpoints. These are invisible bugs that cause 1-tick or
  1-cell drift from gamemd.exe. Always check.
- **Rounding and truncation**: When the code converts between types or divides, does it
  round, truncate, or floor? Which direction? This matters for fixed-point sim math.
- **Default values and initialization**: What does a field hold before the system touches
  it? Zero-init, constructor-set, copied from a template? This decides edge-case behavior.
- **Sign, overflow, saturation**: Signed vs unsigned comparisons, whether arithmetic can
  wrap, whether the code catches it. An unsigned underflow on a counter changes behavior.
- **"Dead" assignments and side effects**: A write that looks dead may be read by another
  system later in the tick. Note it; don't skip it because "nothing uses this here."

**UI / visual paint-path completeness:**

For UI, menu, shell, sidebar, and render parity, the first draw helper is not the
composition. Trace from the top-level paint/message/render entry point until the
handler returns, including every draw call that runs after the parent background,
base chrome, or "main" helper. A visual report is incomplete unless it can explain
the full ordered composition for the target mode/dialog.

Required for every UI/visual investigation:

- **Paint-path closure**: Start at the top-level paint/message handler and follow all
  active draw callees to return. Do not stop at `RightPanel__Draw`, a background
  helper, or the first plausible owner-draw function.
- **Ordered composition ledger**: Record draw order, function/address, condition,
  asset/file, exact frame/index, source/destination rect, palette/convert path,
  clipping/scissor if present, and z-order role.
- **Per-dialog flag proof**: For optional helpers, resolve the target dialog/mode's
  flag setters. Record setter function, allow-list/switch case, written byte/bit,
  shifted byte read in the paint handler, and whether the target dialog activates it.
- **All-frame SHP inspection**: Dump or inspect every frame of SHP assets involved
  before making a visual claim. If a call draws frame 1, frame 2, or any nonzero
  frame, name it explicitly.
- **Asset role matrix**: Classify every relevant asset as `Loaded`, `Drawn`,
  `Visible in target`, `Content/preview`, `Chrome/container`, `Overlay`,
  `Transition-only`, or `Inactive in target`. Assets can have multiple roles.
- **Scoped negative claims**: "Not used as preview backing" does not mean "not
  visible." Every negative claim must name the role it excludes and the role it
  does not exclude.
- **Screenshot contradiction trigger**: If a user screenshot, reference capture, or
  asset dump contradicts the current conclusion, treat the research as incomplete.
  Reopen the paint path and look for a missing layer, frame, flag, rect, or palette.

**Verify YR activity (CRITICAL — do this for every function you decompile):**
- Is this code actually active in Yuri's Revenge, or is it dormant Tiberian Sun legacy?
- Check if it's gated behind a flag/setting — if so, what's the default in YR?
- If gated behind `SpecialFlags & 0x1000` or similar, note it as TS-legacy/optional
- Trace what flags/fields actually gate in the code — don't infer from the name.
  TS-era field names are often misleading in YR context (e.g., "Tiberium" fields
  that control vein behavior, not ore behavior).
- When you find a bool/flag being checked, read the INI key it maps to, confirm
  the default value, and check if any standard YR content actually sets it to a
  non-default value.

**Do NOT stop the Ghidra dive early.** Common traps:
- "I think I have enough to write the report" → No. Follow one more call chain.
- "This helper probably just does X" → Decompile it. Verify. "Probably" is not evidence.
- "The main function is clear, I'll skip the edge cases" → The edge cases ARE the details.
- "I've been at this a while" → Good. That means you're being thorough.

After the deep dive, you should be able to answer: "If someone asked me an obscure
corner-case question about this system, could I answer it from what I found?" If not,
keep digging.

## Step 3 — Exhaustion gate (the Open Questions Log must drain)

This is the hard gate, not a soft checklist. You may not proceed to Step 4
until **every** entry in the Open Questions Log is either `[RESOLVED]` with
evidence or `[DEFERRED]` with an explicit reason and category.

**Procedure — execute in order, do not skip:**

1. **Re-read the entire Open Questions Log top to bottom.** For each `[OPEN]`
   entry: either go back to Ghidra and resolve it (preferred), or write an
   explicit `[DEFERRED]` line with reason + category. "Not relevant" is not a
   reason — say *why* it's not relevant. "Too deep to follow" requires a
   category (`bounded-cost-too-high`) and an estimate of what would resolve it.

2. **Run the zero-add pass test.** Do one more full Ghidra pass over the
   primary function and its top-level callees. If this pass causes you to
   add even one new `[OPEN]` entry, you are not done — resolve it and repeat
   the pass. The investigation exits Step 3 only after a pass that adds zero
   entries.

3. **Adversarial reader test.** Write down 5 specific corner-case questions a
   sharp reader might ask about this system (e.g., "what happens if the unit
   is destroyed mid-animation?", "what's the order if two of these fire on
   the same tick?"). For each, either answer from evidence in your notes or
   add it to the log as `[OPEN]` and resolve it. If you cannot generate 5
   non-trivial questions, you do not understand the system well enough yet.

4. **Spot-check at least 2 findings** by going back to Ghidra and re-reading
   the decompilation cold. Fresh eyes often catch things you glossed over.

5. **Sanity-check the deferral pile.** If more than ~25% of your log entries
   ended up `[DEFERRED]`, the report is not a complete investigation — it's a
   partial one. Either resolve more, or rescope the report's title/scope to
   honestly reflect what was covered (e.g., "Phase 1 — entry points only").
   Do not hide partial coverage behind confident-sounding prose.

**Anti-checkbox rule.** Do not satisfy this gate by adding the literal words
"resolved" or "deferred" without doing the work. Each `[RESOLVED]` line MUST
cite a Ghidra address, doc path, or INI key as evidence. Each `[DEFERRED]`
line MUST state a category from the allowed set and a one-line reason a
future reader can act on.

## Step 4 — Synthesize findings

Cross-reference all sources. For each finding:

| Field | Content |
|-------|---------|
| **What** | The behavior, formula, or state machine |
| **Evidence** | Which source confirmed it (Ghidra address, doc name, INI key) |
| **Confidence** | HIGH (verified from binary), MEDIUM (consistent across docs + INI), LOW (inferred/single source) |
| **Active in YR?** | Yes / No / Conditional (state the condition) |

**Resolve conflicts:**
- If docs say X but Ghidra says Y → Ghidra wins, flag the stale doc
- If two docs conflict → go to Ghidra to settle it
- If INI has a key but no code reads it → note as potentially unused

**Build the Coverage Ledger from the log.** Every planned function, discovered
function, major branch, INI key, and Rust comparison point must appear exactly once
with one of the allowed statuses. Do not collapse "touched" into "verified".

**Build the Implementation Handoff.** This is required even for research-only
work. The handoff is not a patch plan; it is the minimum bridge from verified
binary behavior to a future Rust implementation without making the implementer
rediscover the same facts. Each handoff item must use this shape:

```
Verified behavior -> binary evidence -> current Rust delta -> affected surface -> acceptance scenario -> risk / do-not-do note
```

Rules:
- If Rust already matches, say `current Rust delta: none observed` and give the
  verification surface.
- If Rust was not scanned deeply enough, say `current Rust delta: unchecked` and
  cite the file/function search that remains.
- Do not invent code structure. Name existing files/functions when found; otherwise
  say what kind of surface is needed.
- Every handoff item needs an exact-mechanism, byte/pixel, player-visible, or
  deterministic acceptance scenario.
- A report with no implementation handoff is PARTIAL unless the target is purely
  documentary and has no Rust-facing implication.

## Step 5 — Write research document

Save to `<main-checkout>/docs/research/<TOPIC>_GHIDRA_REPORT.md` (or update existing). This is the canonical ignored research archive; do not save research reports into tracked/published `docs/` locations outside `docs/research/`. Follow this structure:

```markdown
# <System Name> — Ghidra Research Report

**Address(es):** `0x00XXXXXX` (primary function)
**Investigation Mode:** coverage-map | exhaustive-slice
**Claimed Scope:** exact slice this report actually verifies
**Non-Scope:** related areas this report does not claim to cover
**Confidence:** High/Medium/Low (overall)
**Active in YR:** Yes/No/Conditional

## 1. Overview
What this system does in 2-3 sentences.

## 2. Class Layout / Key Offsets
Table of struct fields with byte offsets, types, and purpose.

## 3. Core Logic
The main algorithm, state machine, or formula. Use pseudocode, not C.
Include actual constants from the binary.

## 4. INI Keys
Table of relevant INI keys, their types, defaults, and effects.

## 5. Integration Points
What calls this system? What does it call? When does it run in the tick cycle?

## 6. Current Rust Implementation Status
What we have vs what's missing. File paths and line numbers.

## 7. Coverage Ledger
Use this table. Do not omit it.

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| <name> | verified | <address/doc/INI> | none |
| <name> | touched-not-exhausted | <address/doc/INI> | <specific gap> |
| <name> | not-touched | none | <why it matters> |
| <name> | deferred | <reason> | <next investigation> |

Status values: `verified`, `touched-not-exhausted`, `not-touched`, `deferred`,
`conflict-needs-resolution`.

## 8. Open Questions — Final State of the Investigation Log
The canonical final state of the Open Questions Log from Step 1.5 / Step 3.
Every entry MUST be `[RESOLVED]` or `[DEFERRED]` — no `[OPEN]` items may
ship in a completed report. Use this exact format:

- `[RESOLVED] <id> — <question> → <answer>` (evidence: `<address / doc / INI>`)
- `[DEFERRED] <id> — <question>` (category: `out-of-scope` | `needs-runtime-debugger` | `requires-different-system-context` | `bounded-cost-too-high`; reason: `<one line>`; next-step-if-pursued: `<one line>`)

If the deferred pile is large, also include a short paragraph stating which
parts of the system are NOT covered by this report and what a follow-up
investigation would need to do.

## 9. Visual/UI Composition Ledger
Required when the report covers UI, shell, sidebar, menu, viewport, sprite,
or other visual composition. Omit only when the investigated system has no
visual surface.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|-------|--------------------|------------------------|---------------|---------------|-------------------|--------------------|------|
| 1 | <function> | <always / flag setter evidence> | <file#frame> | <src->dst> | <palette path> | yes/no/conditional | <chrome/content/overlay/etc.> |

Also include an asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|-------|--------|-------|-------------------|-----------------|------------------|---------|-----------------|----------|----------|

## 10. Implementation Handoff
Use this table. Keep it concrete and implementation-facing, but do not include
Rust code or patch text.

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| <binary behavior> | <address/doc/INI> | <missing/mismatch/none/unchecked> | <file/function or needed surface> | <mechanism/byte/pixel effect to reproduce> | <exact-mechanism, byte/pixel, or deterministic test> | <wrong implementation to avoid> |

If this report extends or corrects prior work, include a short `Stale Docs /
Follow-up Docs` list after the table with exact replacement wording for any
known stale claim.

## 11. Ghidra Annotation Candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|----------------|------------------|-------------------|------|------------|--------|
| <address> | <FUN_*/DAT_*/missing ref> | <label/comment/reference> | <rename/create_label/comment/add_memory_reference> | <body + binding or source bytes + target> | <applied/deferred/conflict/worker-report-only> |

Write `None` when no candidate passes ENGINE.md's certainty gate. Do not turn
uncertainty into a cleanup candidate.

## Sources
- Ghidra addresses decompiled
- Doc files referenced
- INI files checked
```

## Step 5.5 — Optional Ghidra annotation sync

By default, stop after reporting Section 11. If `--sync-ghidra-labels` was provided or
the user directly requested synchronization, the root/standalone investigator waits
until every reader has stopped and follows ENGINE.md's serial sync protocol. A read-only
request or `--no-sync-ghidra-labels` disables synchronization. A child/worker always
stops after reporting candidates.

## Confidence levels

- **HIGH** — You decompiled the function, read the assembly context, traced the call chain, and confirmed the behavior. You would bet money on it.
- **MEDIUM** — Multiple docs agree, INI keys are consistent, but you didn't verify every detail in the binary. Or you decompiled but the decompilation was ambiguous.
- **LOW** — Single source, inferred from naming/context, or extrapolated from similar systems. Mark clearly and add to Open Questions.

## Red Flags — STOP

If you catch yourself:
- Writing Rust code → DELETE IT. You are researching, not implementing.
- Omitting the Implementation Handoff → the report is PARTIAL. Future
  implementers need verified behavior translated into affected Rust surfaces,
  acceptance scenarios, and risks.
- Claiming a finding without stating the evidence source → add the source.
- Saying "probably" or "likely" in a finding → move it to Open Questions or go verify in Ghidra.
- Skipping Ghidra because "the docs seem complete" → the docs could be wrong. Spot-check at minimum.
- Copy-pasting a doc's claims without verifying → that's summarizing, not investigating.
- Guessing what a helper function does instead of decompiling it → decompile it.
- Assuming code is active in YR without checking → verify it's not TS-legacy.
- Skipping a detail because it "probably isn't visible to the player" → details drive
  parity. Invisibility is not a filter. Document it.
- Stopping early because the report "has enough" → the job is complete investigation,
  not adequate investigation. Go one more level.
- Leaving a branch, constant, or flag unexplained because "it doesn't look important" →
  if it's in the binary and on an active path, it's important until proven otherwise.
- Declaring the investigation done while the Open Questions Log still has `[OPEN]`
  items → not done. Resolve or explicitly defer (with category + reason) every entry.
- Skipping a Ghidra pass because "the report seems complete" → you don't know if it's
  complete until a full pass adds zero new log entries. Run the pass.
- Marking entries `[RESOLVED]` without citing an address, doc, or INI key as evidence
  → that's a checkbox, not a resolution. Add the evidence or move it to `[DEFERRED]`.
- Generating fewer than 5 adversarial corner-case questions in Step 3 → you don't
  understand the system well enough to gate on exhaustion yet. Keep digging.
- Calling a broad coverage-map "complete" → rename/reframe it. A broad map is useful
  only if it exposes what remains unknown.
- Saying planned/touched functions were "investigated" without a coverage status →
  list each as verified, touched-not-exhausted, not-touched, or deferred.
- Omitting the Coverage Ledger → the report is invalid. Future implementers need to
  know what not to trust yet.
- Stopping UI visual research at the first helper or background draw: incomplete.
  Trace the full paint/message handler through every later active draw call.
- Dumping only SHP frame 0: incomplete for visual parity. Multi-frame SHPs must be
  inspected until the exact drawn frame is proven.
- Turning a role-scoped negative into a global negative: misleading. "Not preview
  backing" does not prove "not visible chrome."
- Ignoring a screenshot/reference contradiction because the current call stack seems
  plausible: reopen the paint path. The contradiction is evidence of missing scope.

## Rationalization table

| Excuse | Reality |
|--------|---------|
| "The docs already cover this" | Docs can be stale or wrong. Verify critical claims in Ghidra. |
| "I can see from the function name what it does" | Ghidra labels can be wrong. YRpp labels are not ground truth. Decompile. |
| "This is simple enough to just implement" | You are researching. Provide a handoff, not a patch. |
| "I'll just note the address and come back later" | Decompile now while you have context. Addresses without analysis are useless. |
| "The INI key name makes the behavior obvious" | INI keys don't tell you edge cases, defaults, or interaction with other systems. |
| "I already know how this works from a previous session" | Verify. Memory is not evidence. |
| "Just a quick code fix while I'm here" | NO. Hard gate. Research only. |
| "The log has open items but they're minor" | Then they take 30 seconds to resolve. Resolve them. |
| "I've covered the main flow, the rest is edge cases" | Edge cases ARE the deliverable. The main flow is what a skim already gives. |
| "The zero-add pass test is overkill for this system" | It's the only objective signal of exhaustion. Run it. |
| "Generating 5 corner-case questions feels artificial" | If you can't generate them, you don't know the system. The exercise is the test. |

## After completion

Present a brief summary to the user:
1. What system was investigated
2. Key findings (3-5 bullets, most important first)
3. What's already implemented vs what's missing
4. Open questions that need more work
5. Highest-leverage implementation handoff item
6. Path to the saved research document
7. Ghidra annotations: candidates / applied / deferred / conflicted
