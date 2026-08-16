---
name: prompt-model
description: "Apply current per-model prompting guidance when drafting or reviewing a prompt aimed at a specific Claude model — Codex goal prompts, /loop invocations, swarm worker prompts, or skill instruction text. Use for '/prompt-model draft <model> <task>', '/prompt-model audit <file-or-skill>', or whenever adapting a prompt written for an older Claude model. Covers Opus 5, Fable 5, and Sonnet 5/4.6; detection and rewrite only — never silently rewrites files without being asked."
---

# Prompt Model — Per-Model Prompting Guidance

Apply the current prompting meta for the target Claude model to `$ARGUMENTS`.
Two modes:

- **draft** — the user wants a prompt written or adapted for a named model.
  Produce the prompt text, applying the shared rules plus the model section.
- **audit** — the user names a file, skill, or prompt to check. Report findings
  (location, pattern, why it is outdated for the target model, proposed fix) and
  apply fixes only when asked.

If the target model is not named, ask which one — the sections below diverge and
two of them give **opposite** subagent advice.

## Why this skill exists

The Claude 5 family inverted several habits that were best practice on Opus 4.x:
self-check instructions now cause over-verification, "be conservative" review
filters now suppress real findings, and pressure language over-triggers. Prompts
and skill worker text written in the old meta actively degrade results on new
models. This file is the single place that guidance lives; update it at each
model release from the sources at the bottom.

## Shared rules — all Claude 5-family models

1. **Complete spec up front.** One well-specified first message: scope, stop
   condition, smallest validation, constraints. Drip-feeding instructions across
   turns reduces both token efficiency and output quality. Don't plan to steer
   mid-run.
2. **No pressure language.** `CRITICAL:`, `You MUST`, `NEVER` (in caps, repeated)
   over-trigger on literal-instruction-following models. Say exactly what you
   mean at normal volume: "Use X when..." not "CRITICAL: you MUST use X".
3. **Instructions are followed literally.** The model will not generalize an
   instruction beyond its stated scope, and it will apply hedges ("try to",
   "if possible") as permission to under-deliver. State scope explicitly; drop
   hedges from actual requirements.
4. **Review prompts: coverage first, filter later.** "Only report high-severity
   issues" / "be conservative" / "don't nitpick" is followed literally — the
   model finds the bugs and then declines to report them. Instead:

   > Report every issue you find, including ones you are uncertain about or
   > consider low-severity. Do not filter for importance or confidence at this
   > stage — a separate verification step will do that. For each finding,
   > include your confidence level and an estimated severity so a downstream
   > filter can rank them.

   Severity/frequency ranking (the project's frequency-clause rule) belongs in
   the parent/reconciliation stage, not the finder prompt.
5. **Positive examples beat prohibition lists.** Show the desired output shape;
   a prohibition against a failure the model wasn't going to make can anchor it
   toward that failure.
6. **No step-by-step choreography for judgment work.** State outcomes,
   constraints, and how to verify; keep numbered steps only where exactly one
   sequence is safe (destructive commands, save protocols).

## Opus 5 (`claude-opus-5`)

Best-documented gain: the goal-shaped run — full spec, long autonomous
execution. Deltas from 4.8-era prompting:

- **Delete verification scaffolding.** It self-verifies unprompted. Remove
  "verify your work", "double-check your answer", "use a subagent to verify",
  and separate harness verification steps. This is a delete, not a rewrite —
  removing them reduces tokens with no quality loss.
- **Constrain scope.** It expands tasks on its own judgment. Standard snippet:

  > Deliver what was asked, at the scope intended. Make routine judgment calls
  > yourself, and check in only when different readings of the request would
  > lead to materially different work. If the request seems mistaken or a
  > better approach exists, say so in a sentence and continue with the task as
  > asked rather than quietly narrowing, widening, or transforming it. Finish
  > the whole task, and stop short of actions clearly beyond what was asked.

- **Prompt for brevity; effort won't do it.** Effort controls thinking volume,
  not visible response length. If output length matters, say so:

  > Keep responses focused, brief, and concise. Spend most of the response on
  > the main answer.

  Files it writes are also longer by default — add a deliverable-length line
  for report-producing prompts ("cover the substance; do not pad with filler
  sections, redundant summaries, or boilerplate").
- **Cap subagent spawning.** It delegates *more* readily than 4.8 (opposite of
  the 4.8-era "encourage delegation" advice — remove that if present):

  > Delegate to a subagent only for large tasks that are genuinely independent
  > and parallelizable. Do not delegate work you can finish yourself in a
  > handful of tool calls, and do not use subagents to verify your own work.
  > Keep spawn counts low.

  In Claude Code the default system prompt already adds delegation restraint on
  Opus 5; add the snippet only for custom system prompts or other harnesses.
- **Effort:** default `high`; `low`/`medium` are unusually strong — use them as
  the primary cost/latency lever; reserve `xhigh` for the hardest coding and
  agentic work. Re-sweep rather than carrying settings from an older model.
- **Leave thinking on.** Disabling it can make the model write tool calls as
  plain text (they silently never run) or leak `<thinking>` tags. Thinking on
  at `low` effort beats thinking off at similar cost.
- **Self-correction narration:** it flags its own earlier mistakes at length.
  For user-facing prompts: correct only errors that change the reader's code,
  conclusions, or decisions; fix silent slips without noting them.

## Fable 5 (`claude-fable-5`)

Strongest on work *above* what prior models could do; prompts written for older
models are often too prescriptive and reduce its output quality.

- **De-prescribe.** A/B with old step-by-step scaffolding removed. State the
  goal and constraints; let it own the plan. It updates its own approach
  mid-task well — let it.
- **Delegation is encouraged — the opposite of Opus 5.** Parallel subagents are
  dependable; prefer async delegation ("delegate independent subtasks and keep
  working while they run; intervene if a subagent goes off track") over
  suppression or spawn-and-block.
- **Ground progress claims.** For long autonomous runs:

  > Before reporting progress, audit each claim against a tool result from this
  > session. Only report work you can point to evidence for; if something is
  > not yet verified, say so explicitly.

- **State boundaries explicitly.** It sometimes takes adjacent-but-unrequested
  actions. Name what is out of scope ("when the user is describing a problem,
  the deliverable is your assessment — report findings and stop").
- **Give it a memory surface.** It performs notably better when told where to
  write learnings (even a plain `.md`) and to consult that file in future
  sessions.
- **Give the reason, not just the request.** "I'm working on [larger task] for
  [who]; they need [what the output enables]. With that in mind: [request]."
- **Plan for long turns.** Single requests can run many minutes at higher
  effort; structure the prompt so check-ins are asynchronous, not blocking.
- **Autonomy nudge for pipelines.** Deep in long sessions it can end a turn
  with a statement of intent instead of the tool call, or ask permission it
  doesn't need. For autonomous runs add: "you are operating autonomously; for
  reversible actions that follow from the request, proceed without asking; end
  your turn only when the task is complete or blocked on user-only input."
- **Avoid rendering context-budget countdowns** into its context — they trigger
  premature wrap-up. Thinking is always on; there is no disable.
- **Effort:** sweep including `low`/`medium` for routine work — low effort on
  Fable often exceeds `xhigh` on prior models.

## Sonnet 5 (`claude-sonnet-5`) and Sonnet 4.6

Near-Opus on coding/agentic work at Sonnet cost. Same shared rules; deltas:

- **More agentic than 4.x Sonnet by default** — reaches for tools and
  self-verification loops readily. With thinking disabled it is *less*
  tool-eager; if a thinking-off route relies on tool calls, add an explicit
  triggering nudge in the prompt or tool description.
- **Effort:** default `high` (≈ Sonnet 4.6 `max` in capability); `medium` ≈
  Sonnet 4.6 `high` — a good cost step-down. `xhigh` for the hardest coding
  and agentic tasks. If reasoning looks shallow at `low`/`medium`, raise effort
  rather than prompting around it.
- **Drop forced progress-update scaffolding** ("summarize every N tool calls")
  — its default interim updates are good; describe the desired shape only if
  they still need tuning.
- **Sonnet 4.6 differences:** thinking is *off* when the `thinking` field is
  omitted (Sonnet 5 runs adaptive by default), and the deprecated
  `budget_tokens` escape hatch still works there. Prompt-side guidance is
  otherwise the same.

## Project prompt shapes

- **Codex goal prompts (4-part).** Keep them short, but the first message must
  carry the full spec: player scenario / scope, non-deferrable constraints,
  smallest production validation, stop condition. Strip any "verify your work"
  part when the target is Opus 5 — it double-verifies. Keep the gamemd-source
  requirement (that is context, not verification scaffolding — it stays).
- **/loop prompts.** The invocation still goes inline, verbatim, at the top
  (CLAUDE.md rule — unchanged). Reserve /loop for genuinely iterative
  discovery (scan passes until dry); a single buildable task on Opus 5 or
  Fable 5 is better as one goal-shaped run — looping it pays re-briefing
  overhead per iteration without adding value.
- **Swarm worker prompts** (re-swarm, trace-swarm, verify-doc-swarm,
  rust-scan lenses). Finder/reviewer workers get the coverage-first reporting
  rule from Shared rule 4; ranking by player-visibility × frequency stays in
  the parent. Workers stay read-only regardless of model.

## Audit checklist

When auditing a prompt, skill, or worker text, grep for these and classify each
hit against the target model's section before proposing an edit:

| Signal | Pattern | Typical fix |
|---|---|---|
| `verify|double-check|re-verify|check your work` | Verification scaffolding | Delete on Opus 5; keep only harness-external validation steps |
| `CRITICAL|MUST|NEVER|ALWAYS` in caps | Pressure language | Restate at normal volume with the reason |
| `only report|high-severity|be conservative|don't nitpick` | Severity filter in a finder prompt | Coverage-first reporting; filter downstream |
| `every N tool calls|summarize progress` | Forced narration cadence | Delete; describe desired update shape only if needed |
| `spawn subagents|delegate|use parallel agents` | Delegation nudge | Cap on Opus 5; keep/encourage on Fable 5 |
| `think step by step|<scratchpad>` | Pre-thinking-era scaffold | Delete; control depth with effort |
| `STEP 1:|STEP 2:` on judgment work | Over-choreography | State outcome + constraints; keep steps only for fragile sequences |

Report findings with location, pattern, and why it is outdated for the target
model. Do not rewrite files unless the invocation asked for fixes.

## Sources and maintenance

Derived from Anthropic's model-specific guides (fetched 2026-08-16):
- https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5
- https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices
- https://platform.claude.com/docs/en/about-claude/models/migration-guide

Re-check these at each model release and update the per-model sections; the
sections encode point-in-time behavior, and per-model advice can invert between
releases (Opus 4.8 → Opus 5 flipped the subagent advice).
