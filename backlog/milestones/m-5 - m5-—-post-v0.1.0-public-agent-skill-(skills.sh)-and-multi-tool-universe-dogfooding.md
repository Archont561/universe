---
id: m-5
title: "M5 — Post-v0.1.0 public Agent Skill (skills.sh) and multi-tool universe dogfooding"
---

## Description

Once the headless `universe-kit v0.1.0` is live on `archont561/archont561`, publish and validate the top-level `skills/universe-kit/SKILL.md` package for `skills.sh` indexing (`npx skills add Archont561/universe`) and prove the full system by bootstrapping a live multi-package universe (migrating recipes such as `CloudCompare` / `ODM`) publishing to both `prefix.dev` and a headless GitHub Pages CEP-15/CEP-16 solver channel. Per decision-11 the browser documentation portal is not part of this milestone; live portal verification belongs to M6 (`UNI-26`).

Exit condition: `skills-ref validate skills/universe-kit` and `pixi run repo-check` version gates pass, `npx skills add Archont561/universe` installs cleanly, and a live consumer universe serves installable Conda packages through `pixi`/`conda` resolution against the headless channel.
