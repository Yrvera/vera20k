# Retail-Convincing Stock Skirmish Governance Design

## Goal

Make ordinary stock skirmish the project's primary delivery target:

> An experienced Yuri's Revenge player should be able to play ordinary stock
> skirmish matches for 30–60 minutes, using any faction on representative retail
> maps, without repeatedly noticing that the game behaves, looks, sounds, or
> responds differently from `gamemd.exe`. An expert deliberately testing edge
> cases may still find differences.

## Architecture Context

Project behavior is governed by four layers:

1. `AGENTS.md` supplies the repository contract for Codex-compatible agents.
2. `CLAUDE.md` supplies the repository contract for Claude Code.
3. `.agents/skills/` and `.claude/skills/` define specialized research,
   planning, review, tracing, and implementation workflows.
4. `docs/goals/` supplies the operating contract for autonomous long-running
   sessions.

The existing contracts mix two concerns:

- evidence truth: whether Rust is exactly equivalent to active `gamemd.exe`;
- delivery priority: what should be fixed next and what blocks a useful
  milestone.

Treating both concerns as the same gate causes common-path implementation to
wait behind exhaustive mechanism, byte, pixel, and documentation work.

## Impact Analysis

This is a governance change. It does not alter Rust architecture or runtime
behavior directly. It changes how future agents select, plan, review, and
declare work complete.

Primary risks:

- weakening the target until agents accept visibly wrong behavior;
- using "normal play" as an excuse to guess rather than investigate;
- hiding known drift or making false parity claims;
- accepting shortcuts that later break determinism, authority, lifecycle, or
  cross-system integration;
- allowing independent sessions to duplicate or overwrite work.

Mitigations:

- separate the truth bar from the delivery bar;
- retain exact `DRIFT`, `UNCHECKED`, and `UNVERIFIED` labels;
- require a written reason before deferring any known difference;
- keep deterministic state, authority, lifecycle, command acceptance,
  persistence, and common-path downstream effects non-deferrable;
- validate real 30–60 minute production matches and their closed loops;
- preserve explicit work claims, disjoint ownership, and guarded integration.

## Chosen Approach

Use a tiered contract.

### Truth bar

Evidence remains literal. Agents must not invent facts, call an approximation
exact, or turn a matching sample into a parity claim. Exact-audit and
reverse-engineering skills continue to surface mechanism, byte, pixel, timing,
audio, and edge-case differences.

### Delivery bar

Current work is judged by whether it materially improves retail-convincing
ordinary stock skirmish. Fix priority is:

1. crashes, hangs, inability to start or finish a match;
2. broken authority, lifecycle, determinism, commands, or system connections;
3. frequently encountered gameplay, AI, economy, production, movement, combat,
   faction, rendering, UI, input, or audio differences;
4. differences that affect several common player loops;
5. isolated expert-only or rare residuals.

Known residuals in the final category remain recorded but do not block the
five-month milestone when their trigger, player effect, frequency, and
downstream risk are stated.

## Player-Experience Detail Ledger

Planning and review retain a detail ledger, but it is risk-ranked:

- `MILESTONE-BLOCKING`: likely to be noticed repeatedly in ordinary stock
  skirmish, changes outcomes, blocks a loop, or threatens deterministic
  architecture.
- `COMPOUNDING`: individually subtle but frequent or shared enough to alter
  game feel across a normal match.
- `EXACTIFICATION-RESIDUAL`: bounded, rare, expert-probed, and shown not to
  threaten common-path behavior or later architecture.
- `UNKNOWN-RISK`: insufficient evidence to classify; investigate only enough
  to determine whether it can affect the milestone.

One-pixel, one-frame, one-tick, and byte-level findings are not automatically
discarded. They are blockers when noticeable, frequent, outcome-changing,
compounding, or architecturally load-bearing. Otherwise they are honest
residuals.

## Design

### Components

- Rewrite the project-purpose and work-priority sections of `AGENTS.md` and
  `CLAUDE.md`.
- Adapt default planning and review skills to use the player-experience ledger.
- Preserve exact research/audit skills as specialist tools with dual reporting:
  exact verdict plus skirmish-milestone impact.
- Add a new autonomous goal for retail-convincing stock skirmish.
- Keep older exact-parity goals as historical artifacts, not current operating
  contracts.

### Work Selection

Select work from real match journeys and observed production failures, not from
repository-wide missing-code scans. Rank candidates by:

`normal-play frequency × player noticeability × loop breadth × unblock value`

Evidence readiness and implementation risk break ties; missing code by itself
does not establish priority.

### Interfaces / Contracts

Every implementation slice records:

- representative player scenario;
- visible problem and expected retail behavior;
- affected closed loop and neighboring systems;
- evidence needed to avoid guessing;
- production-path validation;
- residual differences and why they do or do not block the milestone.

### Testing Strategy

Use a representative stock-skirmish matrix covering:

- Allied, Soviet, and Yuri player paths;
- representative official maps, theaters, and match sizes;
- AI opponents and ordinary default settings;
- economy, build/placement, power, tech progression, movement, combat,
  harvesting, repair, faction powers, shroud, UI, rendering, input, audio, and
  victory/defeat;
- repeated 30–60 minute production matches.

Focused tests remain useful, but helper tests alone cannot close a
player-visible loop. Exact native differential proof is welcome but is not a
milestone prerequisite.

### Concurrency

One session coordinates integration. Each active slice has an explicit claim,
owned files, branch, and worktree. Sessions inspect live claims, tasks,
worktrees, and diffs before selecting or editing. A claimed dependency causes
the session to choose another valuable disjoint problem rather than duplicate
work.

## Architectural Decisions

- Exact parity becomes a non-blocking future aspiration, not the present
  completion gate.
- Determinism and Rust architecture remain hard constraints because shortcuts
  there create broad player-visible failures and expensive rework.
- Research remains precise; research volume is not progress unless it unlocks
  a player-visible implementation decision.
- `DRIFT` remains an evidence label, not an automatic instruction to fix the
  item immediately.
- The milestone is never advertised as byte-perfect, pixel-perfect, or exact
  parity.

## Alternatives Considered

### Change only the long-running prompt

Rejected because repository and skill instructions would continue to override
the desired priority.

### Remove exactness language everywhere

Rejected because it would weaken evidence discipline, encourage guesses, and
erase useful residual knowledge.

### Keep exact parity as the primary gate

Rejected because it optimizes proof completion ahead of reaching a convincing
ordinary skirmish experience within five months.
