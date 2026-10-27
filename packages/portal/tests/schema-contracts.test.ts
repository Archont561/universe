import { describe, test } from "@archont561/bun-test-utils";
import type { CatalogJson, PackageDetailJson } from "@/src/lib/schema.ts";

describe("portal hydration contracts", () => {
  test
    .scenario("a landing-page payload satisfies the CatalogJson contract")
    .given("a minimal catalog hydration payload", () => {
      const catalog = {
        schema: 1,
        universe: {
          name: "tools",
          title: "Tools",
          tagline: null,
          description: "internal tooling channel",
          repository: "https://github.com/example-org/tools",
          accent_color: "#0A74E8",
          generated_at: "2026-10-27T10:00:00Z",
        },
        channels: [
          {
            label: "prefix.dev",
            url: "https://prefix.dev/example-org/tools",
            kind: "prefix-dev",
            cep15_floor_notice: false,
          },
        ],
        stats: { total_packages: 1, total_artifacts: 1, subdirs: ["noarch"] },
        packages: [
          {
            name: "ripgrep",
            latest_version: "14.1.1",
            summary: "line-oriented search tool",
            license: "MIT OR Unlicense",
            subdirs: ["linux-64"],
            category: null,
            featured: false,
            updated_at: "2026-10-27T10:00:00Z",
            audit_passed: true,
          },
        ],
      } satisfies CatalogJson;
      return { catalog };
    })
    .when("reading the package card", ({ catalog }) => ({ card: catalog.packages[0] }))
    .then("the payload is valid as CatalogJson", ({ expect, card, catalog }) => {
      expect(catalog.schema).toBe(1);
      expect(card?.name).toBe("ripgrep");
      expect(card?.audit_passed).toBe(true);
    });

  test
    .scenario("a detail drawer payload satisfies the PackageDetailJson contract")
    .given("a minimal package detail hydration payload", () => {
      const detail = {
        schema: 1,
        name: "ripgrep",
        summary: "line-oriented search tool",
        description: "ripgrep combines the usability of The Silver Searcher with raw speed.",
        homepage: "https://github.com/BurntSushi/ripgrep",
        repository: "https://github.com/BurntSushi/ripgrep",
        doc_url: null,
        license: "MIT OR Unlicense",
        license_family: null,
        latest_version: "14.1.1",
        install: {
          channels: [
            {
              label: "prefix.dev",
              pixi_add: "pixi add ripgrep",
              pixi_global: "pixi global install ripgrep",
              conda_install: "conda install -c tools ripgrep",
              mamba_install: "mamba install -c tools ripgrep",
              pixi_toml: 'channels = ["tools"]',
            },
          ],
        },
        versions: [],
        audit: {
          status: "pass",
          spdx_expression: "MIT OR Unlicense",
          license_files: ["LICENSE.md"],
          binaries: [],
        },
        evidence_html: null,
      } satisfies PackageDetailJson;
      return { detail };
    })
    .when("reading the audit verdict", ({ detail }) => ({ status: detail.audit.status }))
    .then("the payload is valid as PackageDetailJson", ({ expect, detail, status }) => {
      expect(status).toBe("pass");
      expect(detail.install.channels[0]?.pixi_add).toBe("pixi add ripgrep");
    });
});
