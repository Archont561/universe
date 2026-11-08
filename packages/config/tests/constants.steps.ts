/**
 * Steps for `features/constants.feature`.
 *
 * Every assertion reads the constants this package ships, against the checkout the tests run
 * in — no fixture of a repository, because the point of the constants is that they describe
 * the real one.
 *
 * The runner replaces the state with whatever a step returns, `Then` included, so every step
 * hands the state back even when it only asserts.
 */

import { expect } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then, When } from "@aboviq/bun-test-cucumber";
import {
  CRATES_DIR,
  DEFAULT_CONDA_SUBDIRS,
  PACKAGES_DIR,
  REPO_ROOT,
  VERSION,
} from "../src/index.ts";

interface State {
  repoRoot?: string;
  cratesDir?: string;
  packagesDir?: string;
  version?: string;
  manifestVersion?: string;
  subdirs?: readonly string[];
}

Given("the shared constants are imported", (state: State) => ({
  ...state,
  repoRoot: REPO_ROOT,
  cratesDir: CRATES_DIR,
  packagesDir: PACKAGES_DIR,
}));

Given("the version this package declares", (state: State) => ({ ...state, version: VERSION }));

Given("the subdirs a channel may serve", (state: State) => ({
  ...state,
  subdirs: DEFAULT_CONDA_SUBDIRS,
}));

When("the repository root is asked for", (state: State) => state);

When("it is compared with the package manifest", (state: State) => {
  const manifest = JSON.parse(
    readFileSync(join(import.meta.dir, "..", "package.json"), "utf8"),
  ) as { name: string; version: string };
  expect(manifest.name).toBe("@workspace/config");
  return { ...state, manifestVersion: manifest.version };
});

When("they are read", (state: State) => state);

Then(
  "it holds the Pixi manifest, the Cargo manifest and the root package manifest",
  (state: State) => {
    const root = state.repoRoot ?? "";
    for (const manifest of ["pixi.toml", "Cargo.toml", "package.json"]) {
      expect(existsSync(join(root, manifest))).toBe(true);
    }
    return state;
  },
);

Then("the crates and packages directories sit inside it", (state: State) => {
  const root = state.repoRoot ?? "";
  expect(state.cratesDir?.startsWith(root)).toBe(true);
  expect(state.packagesDir?.startsWith(root)).toBe(true);
  expect(existsSync(state.cratesDir ?? "")).toBe(true);
  expect(existsSync(state.packagesDir ?? "")).toBe(true);
  return state;
});

Then("the two are the same version", (state: State) => {
  expect(state.version).toBe(state.manifestVersion);
  return state;
});

Then("every one of them is non-empty and lowercase", (state: State) => {
  const subdirs = state.subdirs ?? [];
  expect(subdirs.length).toBeGreaterThan(0);
  for (const subdir of subdirs) {
    expect(subdir.length).toBeGreaterThan(0);
    expect(subdir).toBe(subdir.toLowerCase());
  }
  return state;
});

Then("every one of them is unique", (state: State) => {
  const subdirs = state.subdirs ?? [];
  expect(new Set(subdirs).size).toBe(subdirs.length);
  return state;
});
