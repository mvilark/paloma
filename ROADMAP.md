# Roadmap

Living tracker for paloma work. Agents and contributors move an item to the next section as its state changes (Sketched ideas -> Planned -> In progress -> Done) and keep one entry per item, with a short note and a link to the PR when there is one.

## Conventions

- **IDs.** Every item gets an ID when it is created, in the form `PAL-001`, `PAL-002`, ... Take the next unused number (check all sections) and never reuse or renumber an ID.
- **Plans.** To plan an idea, copy [docs/plans/TEMPLATE.md](docs/plans/TEMPLATE.md) to `docs/plans/PAL-NNN-short-slug.md`, write the implementation plan there, then move the item to **Planned** and link the plan file from its entry.
- **Moving items.** Move the whole entry (same ID) between sections; add the PR link once work starts.

## Sketched ideas

Rough ideas that are not yet scoped.

- **PAL-012 · Background detection.** OSC 11 query to choose a dark or light palette (PAL-002 Gap 11). See [PAL-004](docs/plans/PAL-004-ui-framework.md#later-not-scheduled-sketched-ideas).
- **PAL-013 · Several live lines.** More than one spinner or progress bar on screen at once, if a real app needs it. See [PAL-004](docs/plans/PAL-004-ui-framework.md#later-not-scheduled-sketched-ideas).

## Planned

Scoped and agreed, not started.

- **PAL-004 · CLI UI framework.** Umbrella plan: turn paloma from a prompt library into a UI framework for prompt-and-flow CLIs (messages, output blocks, live tasks, progress), with tokens, families that share a base, and one settings cascade. Plan: [docs/plans/PAL-004-ui-framework.md](docs/plans/PAL-004-ui-framework.md). Its phases are the items below. Waiting on three direction decisions listed in the plan's open questions.
- **PAL-005 · Housekeeping and safety nets.** Convention fixes and golden tests for the built-in themes and the Ultraviolet example. Plan: [PAL-004, Phase 0](docs/plans/PAL-004-ui-framework.md#pal-005--phase-0-housekeeping-and-safety-nets).
- **PAL-006 · Foundations and messages.** Options families, terminal layer, tokens, `Context`, `restyle`, and `Message` (success / info / warning / error with a hint). Plan: [PAL-004, Phase 1](docs/plans/PAL-004-ui-framework.md#pal-006--phase-1-foundations-and-messages).
- **PAL-007 · Output blocks.** `Section`, `KeyValue`, `Table`, `Panel`, `link`, `gradient`. Plan: [PAL-004, Phase 2](docs/plans/PAL-004-ui-framework.md#pal-007--phase-2-output-blocks).
- **PAL-008 · Live tasks.** `Task` with spinner, steps and elapsed time; motion off for CI and pipes. Plan: [PAL-004, Phase 3](docs/plans/PAL-004-ui-framework.md#pal-008--phase-3-live-tasks).
- **PAL-009 · Theme v2 for prompts.** Prompt hooks take a context and a view; token-driven default layout. Breaking. Plan: [PAL-004, Phase 4](docs/plans/PAL-004-ui-framework.md#pal-009--phase-4-theme-v2-for-prompts).
- **PAL-010 · Prompt gaps.** Hints, rich items, live validation, step counters, cancel and back, presets and unattended mode. Breaking (`Error`). Plan: [PAL-004, Phase 5](docs/plans/PAL-004-ui-framework.md#pal-010--phase-5-prompt-gaps-from-pal-002).
- **PAL-011 · Progress bars.** `Progress` on the live base. Plan: [PAL-004, Phase 6](docs/plans/PAL-004-ui-framework.md#pal-011--phase-6-progress-bars).

## In progress

_(none)_

## Done

- **PAL-003 · App-wide options.** `paloma::set_options` with per-prompt groups and `general`, `PromptBase` shared by all prompts, `Confirm` buttons and `on_shortcut`, `Password` mask and fixed-length result, input on its own line with errors under it. Recorded after the fact: it had no plan. PR #8.
- **PAL-002 · Ultraviolet example app.** A fake VPN CLI in `examples/ultraviolet/` themed as faithfully as possible to the [Ultraviolet design system](docs/design/ultraviolet.md), plus a list of what paloma's theming cannot express yet. Plan: [docs/plans/PAL-002-ultraviolet-example.md](docs/plans/PAL-002-ultraviolet-example.md). Plan in PR #6, implemented in PR #7.

- **PAL-001 · App-wide theme.** `paloma::set_theme(t)` sets the theme for every prompt in the app; `with_theme(&t)` overrides it for one prompt. Plan: [docs/plans/PAL-001-set-theme.md](docs/plans/PAL-001-set-theme.md) (PR #4). Implemented in PR #5.
- **PAL-000 · Docs and rename.** Development guide ([DEVELOPMENT.md](DEVELOPMENT.md)), usage docs ([docs/USAGE.md](docs/USAGE.md)) and crate rename to `paloma` (PR #1).
