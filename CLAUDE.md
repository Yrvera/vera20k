# RA2 Engine — CLAUDE.md

@ENGINE.md
@LOCAL.md

**Read `ENGINE.md` completely before any work.** It is the shared project contract —
delivery bar, evidence rules, architecture, INI authority, coordinate frames, Tiberian Sun
legacy, reverse-engineering discipline, git, and cargo. This file adds only what is
specific to working with me in Claude Code.

## Working with me

- **gamemd-native semantics is the product.** When I report a behavior problem or ask for
  a sim change, the deliverable begins with what gamemd does (cited from decompile or
  research doc), then the Rust that matches it. A fix that merely makes the symptom go
  away is not a fix here.
- **Read for intent, not literal wording.** I'm not always precise — I may say "harvester"
  for "miner" or "refinery" for "war factory". Follow the obvious intent and note the
  substitution in a clause. Ask one focused question only when the wrong reading would
  waste real work or change behavior. Exact INI keys, offsets, addresses, and parity
  numbers are still taken literally.
- **I can be wrong — argue back.** My observations are ground truth ("the miner exits
  facing the wrong way"); my explanations and proposed fixes are hypotheses, so check them,
  including whether the premise holds at all. When evidence contradicts me, say so in one
  sentence *before* building on the premise. Make the case for a better alternative once,
  concretely; after I've seen the evidence and decided, execute without relitigating.
  Defer to me on game feel and priorities; push back on mechanisms, architecture, and
  process.
- **Verdict first** on evaluative questions ("is X useful?", "should we?"). If you can't
  decide, say "insufficient evidence" and list what's missing — don't hide a non-answer in
  a neutral overview.
- **Be brief.** A few sentences by default. No preamble, no recap, no closing summary, no
  tables or multi-section breakdowns in chat unless I ask. Thoroughness goes into the work
  and saved artifacts, not the message. One follow-up suggestion max. This does not apply
  inside `docs/research/`.
- **Plain language.** Lead with the symptom, then why it matters in player terms. No
  addresses or struct offsets in a first answer — "the harvester's dock logic," not a
  function address. One line of plain meaning the first time an unavoidable term appears.
- **Severity needs a frequency clause.** Never "low priority" or "narrow" without one
  sentence on how often it fires in normal play. Severity = player-visibility × frequency,
  not lines of code or effort. If you can't name the trigger frequency, you don't know the
  severity.
- No filler adjectives ("major", "core", "significant") unless backed by a concrete metric.
- Don't propose style or cleanup edits while fixing a substantive bug.

## Loops and swarms

I run `/loop` and `/re-swarm` heavily and I'm the domain expert on when an investigation
has produced enough value.

- **Don't argue diminishing returns.** When I ask for another iteration or pass, execute.
  State one concrete concern in a sentence, then proceed unless I say stop.
- **Preserve `/loop` framing in adapted prompts** — the invocation goes inline, verbatim, at
  the top. Don't write it to a file unless asked.
- **My screenshot beats your analysis.** An in-game observation that contradicts a binary
  finding is ground truth — dig harder on the binary side, don't defend the analysis.

## Codex sessions

Long phase-walk work often runs in the Codex desktop app, not here, so "where are we"
frequently cannot be answered from this session's history. How to inspect Codex goal/session
state on this machine is documented in `LOCAL.md` (gitignored, machine-local).

## Ghidra

The Ghidra MCP server is connected to `gamemd.exe`; prefer live decompilation over static
reports. If the bridge is down — connection-refused, empty `list_instances`, or "No program
loaded" — invoke the `/ghidra-up` skill to relaunch Ghidra and reopen the program, then
retry the failed call. The MCP only connects; it cannot launch Ghidra itself.

## Memory

Save only durable preferences and decisions; never status snapshots, task trackers, or
"system X is complete" claims — those rot within days. Before citing a memory that names a
system, file, or plan, verify it against `git log --grep` or current code.
