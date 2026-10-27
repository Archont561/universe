import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "@archont561/bun-test-utils";
import {
  CRATES_DIR,
  DEFAULT_CONDA_SUBDIRS,
  PACKAGES_DIR,
  REPO_ROOT,
  VERSION,
} from "@/src/index.ts";
import { createUnoConfig } from "@/src/unocss.ts";

interface PackageManifest {
  name: string;
  version: string;
}

describe("@workspace/config", () => {
  test
    .scenario("VERSION matches packages/config/package.json version")
    .given("the packages/config/package.json manifest", () => {
      const manifestPath = join(import.meta.dir, "..", "package.json");
      return { manifestPath };
    })
    .when("reading the package manifest version", ({ manifestPath }) => {
      const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as PackageManifest;
      return { manifest };
    })
    .then("VERSION equals package.json version", ({ expect: sExpect, manifest }) => {
      sExpect(manifest.name).toBe("@workspace/config");
      sExpect(VERSION).toBe(manifest.version);
    });

  test("REPO_ROOT resolves to the monorepo root containing root manifests", () => {
    expect(existsSync(join(REPO_ROOT, "pixi.toml"))).toBe(true);
    expect(existsSync(join(REPO_ROOT, "Cargo.toml"))).toBe(true);
    expect(existsSync(join(REPO_ROOT, "package.json"))).toBe(true);
    expect(CRATES_DIR.startsWith(REPO_ROOT)).toBe(true);
    expect(PACKAGES_DIR.startsWith(REPO_ROOT)).toBe(true);
    expect(DEFAULT_CONDA_SUBDIRS).toContain("linux-64");
  });

  test("createUnoConfig returns a valid offline UnoCSS configuration", () => {
    const config = createUnoConfig({ rules: [["cu-custom", { opacity: "1" }]] });
    expect(Array.isArray(config.presets)).toBe(true);
    expect((config.presets ?? []).length).toBeGreaterThanOrEqual(3);
    expect(Array.isArray(config.transformers)).toBe(true);
  });

  test.prop(
    "every default conda subdir is non-empty lowercase",
    (fc) => ({ subdir: fc.constantFrom(...DEFAULT_CONDA_SUBDIRS) }),
    (_ctx, { subdir }) => {
      expect(subdir.length).toBeGreaterThan(0);
      expect<string>(subdir).toBe(subdir.toLowerCase());
    },
  );
});
