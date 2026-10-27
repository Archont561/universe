/** Hand-maintained portal payload contracts. These are not generated from the Rust core. */
export const VERSION = "0.1.0";

export type Platform = "linux-64" | "linux-aarch64" | "win-64" | "osx-64" | "osx-arm64" | "noarch";

export interface UniverseHeader {
  name: string;
  title: string;
  tagline: string | null;
  description: string;
  repository: string;
  accent_color: string | null;
  generated_at: string;
}

export interface UniverseStats {
  total_packages: number;
  total_artifacts: number;
  subdirs: Platform[];
}

export interface ChannelEndpoint {
  label: string;
  url: string;
  kind: string;
  cep15_floor_notice: boolean;
}

export interface PackageCardSummary {
  name: string;
  latest_version: string;
  summary: string;
  license: string;
  subdirs: Platform[];
  category: string | null;
  featured: boolean;
  updated_at: string;
  audit_passed: boolean;
}

export interface CatalogJson {
  schema: number;
  universe: UniverseHeader;
  channels: ChannelEndpoint[];
  stats: UniverseStats;
  packages: PackageCardSummary[];
}

export interface ChannelInstallSnippet {
  label: string;
  pixi_add: string;
  pixi_global: string;
  conda_install: string;
  mamba_install: string;
  pixi_toml: string;
}

export interface InstallInstructions {
  channels: ChannelInstallSnippet[];
}

export interface BinaryAuditRow {
  path: string;
  format: string;
  architecture: string;
  rpath_status: string;
  needed_libs: string[];
}

export interface AuditReportView {
  status: string;
  spdx_expression: string;
  license_files: string[];
  binaries: BinaryAuditRow[];
}

export interface VersionReleaseRow {
  version: string;
  subdir: Platform;
  url: string;
  sha256: string;
  size_bytes: number;
  build_string: string;
  build_number: number;
  timestamp: number;
  depends: string[];
  constrains: string[];
  run_exports: Record<string, string[]>;
}

export interface PackageDetailJson {
  schema: number;
  name: string;
  summary: string;
  description: string;
  homepage: string | null;
  repository: string | null;
  doc_url: string | null;
  license: string;
  license_family: string | null;
  latest_version: string;
  install: InstallInstructions;
  versions: VersionReleaseRow[];
  audit: AuditReportView;
  evidence_html: string | null;
}
