/**
 * Steps for `features/static-site.feature`.
 *
 * The route contract is imported, not parsed out of a file: `+layout.ts` is a real module, and
 * what a static host depends on is exactly the values it exports. The landing page is read as
 * source, because the thing being held to account is that it takes its version from the
 * library rather than carrying a copy of it.
 *
 * The runner replaces the state with whatever a step returns, `Then` included, so every step
 * hands the state back even when it only asserts.
 */

import { expect } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Given, Then, When } from "@aboviq/bun-test-cucumber";
import { VERSION } from "@/src/lib/index.ts";
import { prerender, trailingSlash } from "@/src/routes/+layout.ts";

interface State {
  route?: { prerender: unknown; trailingSlash: unknown };
  page?: string;
}

Given("the portal's route contract", (state: State) => ({
  ...state,
  route: { prerender, trailingSlash },
}));

Given("the landing page source", (state: State) => ({
  ...state,
  page: readFileSync(join(import.meta.dir, "..", "src", "routes", "+page.svelte"), "utf8"),
}));

When("it is read", (state: State) => state);

When("it is inspected", (state: State) => state);

Then("prerendering is enabled for every route", (state: State) => {
  expect(state.route?.prerender).toBe(true);
  return state;
});

Then("a trailing slash is the canonical form", (state: State) => {
  expect(state.route?.trailingSlash).toBe("always");
  return state;
});

Then("it reads the version from the portal library", (state: State) => {
  const page = state.page ?? "";
  expect(page).toContain('from "@/src/lib/index.ts"');
  expect(page).toContain("{VERSION}");
  // A version written into the page would drift from the package on the next release.
  expect(page).not.toContain(VERSION);
  return state;
});

Then("it shows the version in a badge a test can find", (state: State) => {
  expect(state.page).toContain('data-testid="portal-version"');
  return state;
});
