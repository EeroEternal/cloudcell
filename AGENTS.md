# AGENTS.md — Code Agent Collaboration Specification

This document is the **lightweight entry point** for coding agents in this repository. Do **not** load every chapter by default. Details live in [`docs/ai/agents/`](docs/ai/agents/) and `.agents/skills/`.

## Knowledge Tiering & Token Budget Discipline

| Tier | Content | Entry Point |
| --- | --- | --- |
| **Standing Constraints** | Inviolable rules across all tasks | This file; expanded in [`docs/ai/agents/`](docs/ai/agents/) |
| **Reusable Workflows** | Domain procedures & gates | `.agents/skills/*/SKILL.md` |
| **Vendor/Env Bridges** | Cloudflare Pages, GCP, tunnel | [`docs/deploy.md`](docs/deploy.md) |
| **Domain Specs** | UI / architecture / sandbox contract | [`docs/design.md`](docs/design.md) + [`docs/architecture.md`](docs/architecture.md) + [`docs/sandbox.md`](docs/sandbox.md) |

- Hard limit: **80 lines / 1200 tokens**. Near the limit, **zero-sum** (add one, remove one).
- Never promote a one-off mistake to a global rule. Use skill [`promote-lesson`](.agents/skills/promote-lesson/SKILL.md) after ≥ 2 independent sessions plus human review.

## Agent Reading Map

| Task Signal | Required Reading |
| --- | --- |
| Any visible page / Admin UI | skill [`admin-ui-change`](.agents/skills/admin-ui-change/SKILL.md) → [`docs/design.md`](docs/design.md); [`ui-entry.md`](docs/ai/agents/ui-entry.md) |
| Admin domain modules / API contract | skill [`admin-domain-resource`](.agents/skills/admin-domain-resource/SKILL.md) |
| Sandbox / snapshot / exec / agentcell | [`docs/sandbox.md`](docs/sandbox.md) + [`docs/architecture.md`](docs/architecture.md) |
| Cloudflare Pages / GCP / `api.cloudcell.dev` | [`docs/deploy.md`](docs/deploy.md) |
| `git stash` | skill [`git-stash-safe`](.agents/skills/git-stash-safe/SKILL.md) |
| SQL migrations (`migrations/NNN_*.sql`) | skill [`add-sql-migration`](.agents/skills/add-sql-migration/SKILL.md) |
| Design docs / DDL / Mermaid | skill [`verify-design-doc`](.agents/skills/verify-design-doc/SKILL.md) |
| Release / tagging / production | skill [`release`](.agents/skills/release/SKILL.md) |
| Code review / PR audit | skill [`review`](.agents/skills/review/SKILL.md) |
| API key create / rotate / hash | skill [`api-key-lifecycle-security`](.agents/skills/api-key-lifecycle-security/SKILL.md) |
| Settings IA / identity | skill [`user-attributes-settings`](.agents/skills/user-attributes-settings/SKILL.md) |
| `tokio::spawn` / daemons / exit codes | [`engineering.md`](docs/ai/agents/engineering.md) |
| Commits | [`commit-style.md`](docs/ai/agents/commit-style.md) |
| Crate splits / SQL joins | [`module-boundaries.md`](docs/architecture/module-boundaries.md) |

## Always Active (Highest Standing Constraints)

1. **No Piggybacking**: Commits/PRs must not carry unrelated changes. Split with `git reset --mixed HEAD~1`.
2. **Zero Hallucination Code**: Every definition has callers; every cache field has a store policy; metrics track success and failure; every `TODO` references an issue. Design docs must not cite skeleton-only features as shipped.
3. **Safe Stash**: Honest names; `git diff --stat` before stash; `cargo check --tests` after pop; never stash `Cargo.toml`, `Cargo.lock`, or build scripts.
4. **Release Guardrail**: **Do not merge to main or tag a release without explicit human approval.**
5. **UI stack + viewport**: Product UI is `admin/`. Never ship parallel HTML/JS pages. Dialogs `max-h-[85vh]` + `overflow-y-auto`. No casual subtitles. Global config only on Settings.
6. **Admin i18n**: User-visible copy uses `t('namespace.key')` (fallback allowed until `zh.ts`/`en.ts` exist). No mixed-language literals.
7. **Sorting & Search Clarity**: Sort options include direction (e.g. "Created (New → Old)"); search placeholders list searchable fields.
8. **Core vs plugin**: Auth, egress, masking, and vendor adaptors are middleware/plugins — not hardcoded into the sandbox data plane.
9. **Isolation boundary**: The API process must not `clone` jails. Cells run in `sand serve` on a Linux node. Never `--net host` or default 60% RAM in multi-tenant paths.
10. **Pre-push gate**: Run skill [`pre-push-local-gates`](.agents/skills/pre-push-local-gates/SKILL.md) before push. Do not use CI as a local sandbox.
