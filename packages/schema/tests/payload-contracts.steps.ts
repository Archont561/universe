/**
 * Steps for `features/payload-contracts.feature`.
 *
 * Two of the three scenarios compare the contract with values the workspace really declares,
 * so a subdir added to `@workspace/config` without a matching `Platform` fails here. The third
 * builds a payload under `satisfies CatalogJson`: if a field the portal reads is renamed, this
 * file stops typechecking, which is the earliest place the drift can be caught.
 *
 * The runner replaces the state with whatever a step returns, `Then` included, so every step
 * hands the state back even when it only asserts.
 */

import { expect } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then, When } from "@aboviq/bun-test-cucumber";
import { DEFAULT_CONDA_SUBDIRS } from "@workspace/config";
import { type CatalogJson, type Platform, VERSION } from "../src/index.ts";

/** Every platform the payload contract names. The `satisfies` is the contract. */
const PLATFORMS = [
  "linux-64",
  "linux-aarch64",
  "win-64",
  "osx-64",
  "osx-arm64",
  "noarch",
] as const satisfies readonly Platform[];

interface State {
  version?: string;
  manifestVersion?: string;
  subdirs?: readonly string[];
  catalog?: CatalogJson;
}

/** One catalog payload, shaped as the producer writes it. */
function catalog(): CatalogJson {
  return {
    schema: 1,
    universe: {
      name: "universe",
      title: "Universe",
      tagline: null,
      description: "Sealed offline snapshots",
      repository: "https://github.com/Archont561/universe",
      accent_color: null,
      generated_at: "2026-10-29T00:00:00Z",
    },
    channels: [
      {
        label: "prefix.dev",
        url: "https://prefix.dev/archont561/archont561",
        kind: "prefix-dev",
        cep15_floor_notice: false,
      },
    ],
    stats: { total_packages: 1, total_artifacts: 1, subdirs: ["linux-64"] },
    packages: [
      {
        name: "universe-airlock",
        latest_version: "0.1.0",
        summary: "Sealed offline snapshots",
        license: "MIT OR Apache-2.0",
        subdirs: ["linux-64"],
        category: null,
        featured: false,
        updated_at: "2026-10-29T00:00:00Z",
        audit_passed: true,
      },
    ],
  };
}

Given("the schema package is imported", (state: State) => ({
  ...state,
  version: VERSION,
  subdirs: DEFAULT_CONDA_SUBDIRS,
}));

Given("a produced catalog payload", (state: State) => ({ ...state, catalog: catalog() }));

When("the declared version is compared with the package manifest", (state: State) => {
  const manifest = JSON.parse(
    readFileSync(join(import.meta.dir, "..", "package.json"), "utf8"),
  ) as { version: string };
  return { ...state, manifestVersion: manifest.version };
});

When("the default Conda subdirs are read", (state: State) => state);

When("its package card is read", (state: State) => state);

Then("they are the same version", (state: State) => {
  expect(state.version).toBe(state.manifestVersion);
  return state;
});

Then("every one of them is a platform the payload names", (state: State) => {
  const subdirs = state.subdirs ?? [];
  expect(subdirs.length).toBeGreaterThan(0);
  for (const subdir of subdirs) {
    expect(PLATFORMS as readonly string[]).toContain(subdir);
  }
  return state;
});

Then(
  "the card names the package, its latest version and whether its audit passed",
  (state: State) => {
    const card = state.catalog?.packages[0];
    expect(card?.name).toBe("universe-airlock");
    expect(card?.latest_version).toBe("0.1.0");
    expect(card?.audit_passed).toBe(true);
    return state;
  },
);

Then("the catalog names the channel a consumer pastes into their manifest", (state: State) => {
  const channel = state.catalog?.channels[0];
  expect(channel?.label).toBe("prefix.dev");
  expect(channel?.url).toBe("https://prefix.dev/archont561/archont561");
  return state;
});
