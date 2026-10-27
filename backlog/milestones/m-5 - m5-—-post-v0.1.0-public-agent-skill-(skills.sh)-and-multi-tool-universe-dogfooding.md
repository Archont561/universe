---
id: m-5
title: "M5 — Post-v1 public Agent Skill and multi-tool Universe dogfooding"
---

## Description

After the integrated Universe v1 release (M10/UNI-16), publish and validate the public `skills/universe-kit/SKILL.md` package for `skills.sh` indexing and prove real multi-package Universe usage, including migrated recipes such as CloudCompare/ODM where appropriate. The skill rollout and generic/Agent-Skill artifact support are post-v1; the presence of a public skill does not expand v1's registry adapter scope.

Exit condition: `skills-ref validate skills/universe-kit`, repository version gates, and public installation checks pass; dogfooding reports reproducible native Conda resolution through the standard static channel and documents which producer/consumer surfaces were exercised.