---
title: "Questions go one at a time, even when a skill says to batch them"
author: ILexN
date: 2026-11-18
branch: main
id: l-29d5812d
scope: repo
status: active
src: agent
touches:
  - AGENTS.md
  - ".agents/skills/grilling/SKILL.md"
---

## Decided
- The AGENTS.md working rule (one focused question, wait for the answer, never batch) wins over an invoked skill's own protocol when the two conflict. The grilling skill formats its rounds as a numbered batch of frontier questions; in this repo, ask those questions one by one, each with its recommended answer, and wait between them.
## Watch out
- The improve-codebase-architecture command's grilling step is where this bites: its skill explicitly says "ask the whole frontier in one round", and the agent followed that — the user had to repeat the repo rule.

In the code: AGENTS.md (Working rules), .agents/skills/grilling/SKILL.md (round format).
