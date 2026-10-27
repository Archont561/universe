import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, test } from "@archont561/bun-test-utils";
import { VERSION as SCHEMA_VERSION } from "@universe/conda-schema";
import { PORTAL_DIR } from "@workspace/config";
import { VERSION } from "@/src/lib/index.ts";

interface PackageManifest {
  name: string;
  version: string;
}

describe("@universe/portal", () => {
  test
    .scenario("VERSION matches packages/portal/package.json version")
    .given("the packages/portal/package.json manifest", () => ({
      manifestPath: join(PORTAL_DIR, "package.json"),
    }))
    .when("reading the package manifest", ({ manifestPath }) => ({
      manifest: JSON.parse(readFileSync(manifestPath, "utf8")) as PackageManifest,
    }))
    .then("VERSION equals package.json version", ({ expect, manifest }) => {
      expect(manifest.name).toBe("@universe/portal");
      expect(VERSION).toBe(manifest.version);
      expect(VERSION).toBe(SCHEMA_VERSION);
    });
});
