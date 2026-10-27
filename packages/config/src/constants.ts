import { join, resolve } from "node:path";

export const VERSION = "0.1.0";

export const PROJECT_NAME = "universe";
export const OWNER_CHANNEL = "https://prefix.dev/archont561/archont561";

export const DEFAULT_CONDA_SUBDIRS = [
  "noarch",
  "linux-64",
  "linux-aarch64",
  "win-64",
  "osx-64",
  "osx-arm64",
] as const;

export type CondaSubdir = (typeof DEFAULT_CONDA_SUBDIRS)[number];

/** Absolute path to the repository root directory. */
export const REPO_ROOT = resolve(import.meta.dir, "../../..");

/** Absolute paths to canonical workspace directories. */
export const CRATES_DIR = join(REPO_ROOT, "crates");
export const PACKAGES_DIR = join(REPO_ROOT, "packages");
export const SCRIPTS_DIR = join(REPO_ROOT, "scripts");
export const SCHEMAS_DIR = join(REPO_ROOT, "schemas");
export const WORKFLOWS_DIR = join(REPO_ROOT, ".github", "workflows");
export const SKILLS_DIR = join(REPO_ROOT, ".agents", "skills");
export const PORTAL_DIR = join(PACKAGES_DIR, "portal");
export const PORTAL_BUILD_DIR = join(PORTAL_DIR, "build");
