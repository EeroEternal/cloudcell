# Documentation Layout & Lifecycle

This document describes the structure and lifecycle of the `docs/` tree.

## Directory Structure

- `docs/architecture.md`: High-level system architecture and domain models.
- `docs/design.md`: Single source of truth for UI/UX design specifications.
- `docs/design/`: Detailed design chapters (tokens, colors, typography, layout, components).
- `docs/ai/agents/`: Engineering guidelines, commit standards, and agent governance.
- `docs/dev/`: Pre-implementation proposals and post-release reviews. Claims about
  existing code, SQL and diagrams must be verified by execution before commit — see
  the [`verify-design-doc`](../../.agents/skills/verify-design-doc/SKILL.md) skill.

## Document Lifecycle Discipline

1. **No Phantom Capabilities**: Never document skeleton-only or hypothetical features as ready.
2. **Deterministic Verification**: SQL schemas and code snippets inside documentation must be executable and verified.
