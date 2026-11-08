/**
 * Preload that teaches `bun test` to run the `.feature` files in `features/`.
 *
 * Gherkin scenarios and their step definitions are two halves of one suite, so the plugin is
 * pointed at `tests/**\/*.steps.ts` and nothing else: a step file is a step file wherever it
 * sits under `tests/`.
 */

import { bunTestCucumber } from "@aboviq/bun-test-cucumber";
import { plugin } from "bun";

await plugin(bunTestCucumber({ stepDefinitionsPattern: "tests/**/*.steps.ts" }));
