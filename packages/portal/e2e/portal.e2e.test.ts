import { existsSync, readFileSync, statSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import { describe, expect, test } from "@archont561/bun-test-utils";
import { PORTAL_BUILD_DIR, PORTAL_DIR } from "@workspace/config";
import { VERSION } from "@/src/lib/index.ts";

interface PackageManifest {
  version: string;
}

interface TestServer {
  url: string;
  handle: (fn: (req: Request) => Response | Promise<Response>) => void;
}

interface WebPage {
  mode: "dom" | "browser";
  goto: (url: string) => Promise<void>;
  textContent: (selector: string) => Promise<string | null>;
  html: () => Promise<string>;
}

const MIME_BY_EXT: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
};

describe("@universe/portal static SSG output", () => {
  test("prerendered build/index.html embeds relative assets and package VERSION", () => {
    const manifest = JSON.parse(
      readFileSync(join(PORTAL_DIR, "package.json"), "utf8"),
    ) as PackageManifest;
    const indexPath = join(PORTAL_BUILD_DIR, "index.html");

    expect(existsSync(indexPath)).toBe(true);
    const html = readFileSync(indexPath, "utf8");
    expect(html).toContain(`v${manifest.version}`);
    expect(html).toContain(`v${VERSION}`);
    expect(html).toContain("./assets/");
    expect(html).not.toContain("/_app/");
  });

  test("renders cleanly when served from an arbitrary repository subpath", async ({
    testServer,
    webPage,
  }) => {
    const server = testServer as TestServer;
    const page = webPage as WebPage;
    const mountPrefix = "/acme/arbitrary-universe";

    server.handle((req) => {
      const { pathname } = new URL(req.url);
      if (!pathname.startsWith(mountPrefix)) {
        return new Response("Not Found", { status: 404 });
      }
      const rel = pathname.slice(mountPrefix.length).replace(/^\/+/, "") || "index.html";
      const filePath = resolve(PORTAL_BUILD_DIR, rel);
      if (!filePath.startsWith(resolve(PORTAL_BUILD_DIR))) {
        return new Response("Forbidden", { status: 403 });
      }
      if (!existsSync(filePath) || !statSync(filePath).isFile()) {
        return new Response("Not Found", { status: 404 });
      }
      const contentType = MIME_BY_EXT[extname(filePath)] ?? "application/octet-stream";
      return new Response(readFileSync(filePath), {
        status: 200,
        headers: { "content-type": contentType },
      });
    });

    await page.goto(`${server.url}${mountPrefix}/`);
    expect(await page.textContent("h1")).toBe("universe");
    const rendered = await page.html();
    expect(rendered).toContain(`v${VERSION}`);
    expect(rendered).toContain(
      "Portable CEP-15 / CEP-16 static Conda channel and documentation portal.",
    );
  });
});
