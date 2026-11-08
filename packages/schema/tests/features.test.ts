/**
 * Entry point for the behaviour suite.
 *
 * Bun's test runner only discovers `.ts` files, so the Gherkin documents in `features/` are
 * loaded from here. The path is resolved against this package, not the repository root, so
 * `bun test tests` behaves the same however it is invoked.
 */
import { join } from "node:path";
import { loadFeatures } from "@aboviq/bun-test-cucumber";

await loadFeatures("features/**/*.feature", join(import.meta.dir, ".."));
