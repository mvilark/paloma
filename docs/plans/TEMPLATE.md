# PAL-NNN: Title

Copy this file to `docs/plans/PAL-NNN-short-slug.md` (same ID as the roadmap item). Once the plan is written, move the item to **Planned** in [ROADMAP.md](../../ROADMAP.md) and link this file from it.

**Write for an agent without your context.** The plan must be complete enough for a separate agent to implement it alone, when all that agent has done is study the project and read every `.md` file in the repo. Close every design question in the plan (names, signatures, defaults, behaviour at the edges); quote the client's decisions instead of referring to conversations; name the existing code to reuse by path. Choose the executor before writing the rest (see **Executor**).

## Executor

Model and effort that will implement this plan, and why. Weigh:

| Criterion | Pushes toward |
| --- | --- |
| Design left open after the plan | a stronger model; better, close it in the plan |
| Risk of subtle bugs (terminal state, concurrency, cursor maths, byte-identical output) | higher effort |
| Size, and how mechanical the change is | a cheaper model for mechanical work |
| Verification needs (pty checks, golden diffs) | at least medium effort |
| Token economy | the cheapest executor that meets the acceptance criteria without rework |

The planner reviews the executor's diff and reruns the checks before anything is pushed.

## Context and decisions

What problem this solves, and every client decision that applies (dated, quoted or paraphrased exactly).

## Files

Files to create and to change, one line each on what changes.

## Public API

Exact signatures with their doc summaries, defaults, and what stays backwards compatible.

## Implementation steps

Ordered steps. For each: what to write, which existing functions or patterns to reuse (with paths), and any trap to avoid.

## Tests and acceptance criteria

- Golden snapshots that must stay byte-identical, and new ones to add.
- Unit tests to add (where, what they assert).
- Manual or pty checks, with the exact keys to send and what the screen must show.
- The full check set from `DEVELOPMENT.md` (tests with all, default and no features; `cargo lint`; `cargo format-check`; examples; docs with warnings denied; MSRV).

## Out of scope

What not to do in this phase, and which later item covers it.

## Status notes

Dated progress notes as work happens, with PR links.
