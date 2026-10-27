import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, test } from "@archont561/bun-test-utils";
import { PACKAGES_DIR } from "@workspace/config";
import { VERSION } from "@/src/index.ts";

interface PackageManifest {
  name: string;
  version: string;
}

describe("@universe/conda-schema", () => {
  test
    .scenario("VERSION matches packages/schema/package.json version")
    .given("the packages/schema/package.json manifest", () => ({
      manifestPath: join(PACKAGES_DIR, "schema", "package.json"),
    }))
    .when("reading the package manifest", ({ manifestPath }) => ({
      manifest: JSON.parse(readFileSync(manifestPath, "utf8")) as PackageManifest,
    }))
    .then("VERSION equals package.json version", ({ expect, manifest }) => {
      expect(manifest.name).toBe("@universe/conda-schema");
      expect(VERSION).toBe(manifest.version);
    });
});
