---
title: frontend/package.json must keep typescript on ~6.0.x until vue-tsc supports TS 7 native binary
author: ILexN
date: 2026-10-26
branch: main
id: l-22317df9
scope: repo
status: active
src: agent
touches:
  - Cargo.lock
  - backend/Cargo.toml
  - frontend/package.json
  - frontend/pnpm-lock.yaml
---

## Decided
Keep `typescript` pinned to `~6.0.3` in `frontend/package.json` rather than upgrading to `typescript@7.x`.

## Tried and ruled out
Upgrading `typescript` to `7.0.2`: `typescript@7.x` replaces the JS compiler (`lib/_tsc.js`) with a native Go binary launcher in `lib/tsc.js` (`process.execve`/`execFileSync`). `vue-tsc` (`@volar/typescript/lib/quickstart/runTsc.js`) monkey-patches `typescript/lib/tsc.js` in-process and expects `module.exports = require("./_tsc.js")`, so `typescript@7.x` fails immediately with `Error: Failed to locate tsc module path from shim`.

## Watch out
When running `pnpm outdated` in `frontend/`, `typescript` will show `7.x` as `Latest`. Do not bump `frontend/package.json` past `~6.0.x` (unless aliasing via `@typescript/typescript6` or after `vue-tsc` drops its JS `_tsc.js` shim requirement).
