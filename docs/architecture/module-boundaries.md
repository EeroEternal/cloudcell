# Module Boundaries & Crate Architecture

This document defines module boundaries, dependency flow, and cross-module interaction rules.

## 1. Core Principles

1. **Unidirectional Dependency Flow**: Transport (`server`) depends on domain (`sandbox`, `api_key`, `snapshot`) and `state`. Domain must not import `server`.
2. **No Direct Table Tampering Across Modules**: Cross-module data access goes through the `AppState` maps (and later through repository types). Do not reach into another module's HashMap from handlers you do not own.
3. **Change Atomicity**: Breaking API changes need a versioned path (`/api/v1` stays stable; add `/api/v2` or additive fields).
4. **Cell runtime stays out of this crate**: A future `cloudcell-agent` crate may drive `sand`. This binary does not `clone` namespaces or link `libagentcell`.
