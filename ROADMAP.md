# Roadmap

Living tracker for paloma work. Agents and contributors move an item to the next section as its state changes (Sketched ideas -> Planned -> In progress -> Done) and keep one entry per item, with a short note and a link to the PR when there is one.

## Conventions

- **IDs.** Every item gets an ID when it is created, in the form `PAL-001`, `PAL-002`, ... Take the next unused number (check all sections) and never reuse or renumber an ID.
- **Plans.** To plan an idea, copy [docs/plans/TEMPLATE.md](docs/plans/TEMPLATE.md) to `docs/plans/PAL-NNN-short-slug.md`, write the implementation plan there, then move the item to **Planned** and link the plan file from its entry.
- **Moving items.** Move the whole entry (same ID) between sections; add the PR link once work starts.

## Sketched ideas

Rough ideas that are not yet scoped.

_(none yet)_

## Planned

Scoped and agreed, not started.

_(none yet)_

## In progress

- **PAL-002 · Ultraviolet example app.** A fake VPN CLI in `examples/ultraviolet/` themed as faithfully as possible to the [Ultraviolet design system](docs/design/ultraviolet.md), plus a list of what paloma's theming cannot express yet. Plan: [docs/plans/PAL-002-ultraviolet-example.md](docs/plans/PAL-002-ultraviolet-example.md). PR: branch `claude/pal-002`.

## Done

- **PAL-001 · App-wide theme.** `paloma::set_theme(t)` sets the theme for every prompt in the app; `with_theme(&t)` overrides it for one prompt. Plan: [docs/plans/PAL-001-set-theme.md](docs/plans/PAL-001-set-theme.md) (PR #4). Implemented in PR #5.
- **PAL-000 · Docs and rename.** Development guide ([DEVELOPMENT.md](DEVELOPMENT.md)), usage docs ([docs/USAGE.md](docs/USAGE.md)) and crate rename to `paloma` (PR #1).
