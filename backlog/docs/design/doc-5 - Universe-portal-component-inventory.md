---
id: doc-5
title: Universe portal component inventory
type: design
created_date: '2026-10-27'
updated_date: '2026-10-27'
---

# Universe Portal mockup — component extraction

Source analysed: `assets/universe-portal-mockup.html` (320 lines, one inline stylesheet,
one inline script). The mockup is a single static file, but its DOM, CSS class
system (`47` classes), IDs (`26`), and JavaScript already imply a clear
component architecture for the real SvelteKit portal in `packages/portal`.

## Summary

The mockup contains:

| Kind | Count | Meaning |
|---|---:|---|
| Design foundations | 4 | Tokens and structural surfaces, not rendered as standalone components |
| Primitive components ("atoms") | 13 | Small presentational units with little or no state |
| Composite components ("molecules") | 13 | Groups of primitives forming one reusable control or display unit |
| Section components ("organisms") | 15 | Data-bearing regions of a page |
| Shell and page templates | 4 | `PortalShell`, `PortalFooter`, browse/home, and entry detail |
| State / presenter units | 6 | JavaScript behaviour that should become `.svelte.ts` modules or actions in Svelte 5 |

Total: **45 UI components, supported by 4 foundations and 6 behaviour units**.
The navigation also names
five future pages (Map, Skills, Builds, Snapshots, Docs), but the mockup does
not implement them; they are page placeholders, not extracted components.

The extraction rule used below: something is a component if it has a repeated
visual pattern, an identifiable responsibility, a stable data contract, its own
state/variants, or is a page region that can be tested independently.

## Component tree

```text
PortalShell
├── TopBar
│   ├── BrandMark
│   ├── MainNav
│   │   └── MainNavItem ×6
│   ├── CapabilityBadge
│   └── RepositoryLink
├── BrowsePage
│   ├── UniverseHero
│   │   └── StatsStrip
│   │       └── HeroStat ×5
│   ├── UniverseEndpointPanel
│   │   ├── SnippetTabs
│   │   │   └── PillTab ×4
│   │   └── CodeSnippet
│   │       └── CopyButton
│   ├── FilterRail
│   │   ├── FacetGroup: Ecosystem
│   │   │   └── FilterChip ×6
│   │   ├── FacetGroup: Platform
│   │   │   └── CheckboxFacetItem ×4
│   │   ├── FacetGroup: Category
│   │   │   └── CheckboxFacetItem ×4
│   │   ├── FacetGroup: Trust
│   │   │   └── CheckboxFacetItem ×3
│   │   └── SnapshotCTA
│   └── CatalogBrowser
│       ├── SearchToolbar
│       │   ├── SearchBox
│       │   │   └── KbdHint
│       │   ├── SortSelect
│       │   └── ResultCount
│       ├── EntryCard ×9
│       │   ├── FootprintThumbnail?    (STAC entries only)
│       │   ├── EcosystemBadge
│       │   └── EntryMetaTags
│       │       └── MetaTag ×n
│       └── EmptyState                (only a fallback string in the mockup)
├── EntryDetailPage
│   ├── Breadcrumbs
│   ├── EntryDetailHeader
│   │   ├── EcosystemBadge
│   │   └── EntryMetaTags
│   ├── VerificationEvidence
│   │   └── MetricPillar ×4
│   ├── ArtifactMatrix
│   ├── DependencyList
│   ├── BinaryAuditTable
│   ├── ProvenanceTable
│   ├── EntryInstallPanel
│   │   ├── SnippetTabs
│   │   └── CodeSnippet
│   ├── AboutPanel
│   └── OfflineUsagePanel
└── PortalFooter
```

## 1. Foundations

These are not Svelte components; they belong in UnoCSS/theme configuration and
shared utility classes in the real portal.

| ID | Foundation | Mockup evidence | Real implementation |
|---|---|---|---|
| F01 | Colour and typography tokens | `:root` variables: `--accent`, `--bg`, `--panel`, `--border`, ecosystem colours, `--mono`, `--sans` | UnoCSS preset/theme plus `--cu-accent` from `universe.toml [pages]` |
| F02 | Surface | `.card`, `.pkg`, `.snapshot`, `.pillar`, `.code` radius/border/gradient treatments | Shared surface utilities/shortcuts; `SurfaceCard.svelte` only if a slot wrapper is needed |
| F03 | Responsive layout primitives | `.wrap`, `.hero`, `.browse`, `.detail-grid`, `.grid`, 920 px media query | Route/layout CSS, not deeply nested layout components |
| F04 | Interaction/a11y baseline | `:focus-visible`, sticky top bar/filter rail, hover lift, `.hidden` | Global CSS and component contracts; visible focus is mandatory |

### Ecosystem colour tokens

`--conda`, `--python`, `--stac`, `--skills`, and `--generic` form a semantic token
group. Colour is always paired with a text label in `EcosystemBadge`, which is
correct: meaning is not carried by colour alone.

## 2. Primitive components

| ID | Component | Mockup anchor | Props / variants | State / events |
|---|---|---|---|---|
| A01 | `BrandMark` | `.logo`, `.orb` | Owner/name, accent, link target | Presentational |
| A02 | `MainNavItem` | `nav.main button` | Label, href, `active` | Navigation event; in Svelte must be an `<a>`, not a button |
| A03 | `CapabilityBadge` | `.cap-badge` | Mode, generated timestamp, snapshot/version | Presentational; tooltip source is `capabilities.json` |
| A04 | `HeroStat` | `.stat` | Value, label | Presentational |
| A05 | `PillTab` | `.tabs button` | Label, value, `active`, disabled | `onselect`; used by both endpoint and install panels |
| A06 | `CopyButton` | `.copy`, `[data-copy]` | Text or target reference | Local transient `copy -> copied ✓` state; clipboard side effect |
| A07 | `FilterChip` | `.chip`, `.chip.active` | Label, value, optional count, `active` | Toggle/select event |
| A08 | `CheckboxFacetItem` | `.filters label > input` | Label, value, checked, count (missing in mockup) | Change event |
| A09 | `MetaTag` | `.tag`, `.ok`, `.warn`, `.cov` | Label, tone (`neutral`, `ok`, `warn`, `coverage`), icon | Presentational |
| A10 | `EcosystemBadge` | `.eco`, `.eco-conda`, etc. | Ecosystem enum | Presentational |
| A11 | `FootprintThumbnail` | `.thumb > svg` | Geometry/bbox or thumbnail URL, alt text | Presentational in static mode |
| A12 | `MetricPillar` | `.pillar` | Value, label, optional tone/link | Presentational |
| A13 | `KbdHint` | `.kbd` | Shortcut label | Presentational |

`RepositoryLink` can remain a normal anchor with an icon; it does not justify a
separate component unless external-link handling is centralised.

## 3. Composite components

| ID | Component | Composition / anchor | Contract |
|---|---|---|---|
| M01 | `MainNav` | `nav.main` + `MainNavItem` | Receives routes and current route; owns responsive overflow behaviour |
| M02 | `StatsStrip` | `.stats` + `HeroStat` | Receives labelled metrics; formats counts/durations |
| M03 | `CodeSnippet` | `.code` + `CopyButton` | Receives the exact copyable text separately from rendered/highlighted text |
| M04 | `SnippetTabs` | `.tabs` + `CodeSnippet` + `.note` | Receives snippets and selected tool; emits tool changes; note is data, not hardcoded markup |
| M05 | `SearchBox` | `.search`, input, `KbdHint` | Two-way query value, placeholder, optional clear action; global shortcut handled outside |
| M06 | `SearchToolbar` | `.toolbar`, `SearchBox`, sort `<select>`, `.count` | Query, sort, result count, total count; emits query/sort changes |
| M07 | `FacetGroup` | facet heading + chips or checkbox items | Generic over single-select chip and multi-select checkbox variants |
| M08 | `EntryMetaTags` | `.meta`, JS `tags(e)` | Converts an entry summary into platform/trust/coverage/license/freshness tags |
| M09 | `MetricPillarGroup` | `.pillars` | Receives evidence metrics and renders the responsive grid |
| M10 | `Breadcrumbs` | `.crumbs` | Route segments; Back is a link in the real routed app |
| M11 | `SnapshotCTA` | `.snapshot` | Snapshot pin, counts, total size, manifest URL; may be absent when no snapshot exists |
| M12 | `DataTable` | four `table` instances | Typed columns/rows; should not be over-generalised until the table patterns stabilise |
| M13 | `KeyValueList` | Provenance and About tables | Label/value pairs with text, mono, and link cells |

## 4. Section components

| ID | Component | Page region and responsibility | Main data source in the real system |
|---|---|---|---|
| O01 | `TopBar` | Sticky identity, navigation, capability disclosure, repository link | `UniverseHeader`, route table, `Capabilities` |
| O02 | `UniverseHero` | Universe title/tagline/description and summary metrics | `UniverseHeader`, extended `UniverseStats` |
| O03 | `UniverseEndpointPanel` | First-run "use this universe" snippets by ecosystem/tool | Generalised universe endpoints + `InstallSnippet[]` |
| O04 | `FilterRail` | Ecosystem, platform, category, and trust facets; houses `SnapshotCTA` in the mockup | Catalog facet counts + browser filter state |
| O05 | `CatalogBrowser` | Toolbar, responsive entry grid, result count, empty state, static/serve note | `CatalogJson` shards or serve search API |
| O06 | `EntryCard` | One package/collection/skill/asset summary linking to detail | New `EntryCardSummary` replacing Conda-only `PackageCardSummary` |
| O07 | `EntryDetailHeader` | Ecosystem, name/version, summary, meta tags | `EntryDetailJson` header subset |
| O08 | `VerificationEvidence` | Audit, coverage, pinning, and platform-count summary | Audit + `CoverageFragment` + artifact set |
| O09 | `ArtifactMatrix` | Version/platform/size/hash/download table | Existing `VersionReleaseRow`, generalised beyond Conda |
| O10 | `DependencyList` | Runtime dependencies/constraints/assets rendered as tags in the mockup | Typed `DependencyRef[]`, not free strings |
| O11 | `BinaryAuditTable` | Per-binary format, architecture, RPATH verdict, imported libraries | Existing `AuditReportView` / `BinaryAuditRow` |
| O12 | `ProvenancePanel` | Source, build run, attestation, SBOM, recipe/source manifest | **Missing today:** needs a `ProvenanceView` schema |
| O13 | `EntryInstallPanel` | Sticky/sidebar per-entry install snippets and endpoint note | Existing `InstallInstructions`, generalised to snippet list |
| O14 | `OfflineUsagePanel` | Snapshot inclusion and Airlock execution snippet | **Missing today:** needs `SnapshotSummary`/offline view data |
| O15 | `AboutPanel` | License, homepage, repository, updated timestamp | `PackageDetailJson` metadata, generalised |

`PortalFooter` is a layout organism rather than a data-bearing section: it
shows generator/schema identity and the machine-readable endpoint roots.

## 5. Page templates

| ID | Template | Mockup anchor | Real SvelteKit route |
|---|---|---|---|
| T01 | `BrowsePage` | `#home-view` | `src/routes/+page.svelte` |
| T02 | `EntryDetailPage` | `#detail-view` | `src/routes/packages/[name]/+page.svelte` or ecosystem-aware equivalent |

`PortalShell` maps to `+layout.svelte`. Map, Skills, Builds, Snapshots, and
Docs are **navigation labels only** in this mockup; no components for them can
be extracted honestly yet.

## 6. Behaviour currently embedded in JavaScript

These are the mockup's real "components" in behaviour even though they are not
DOM elements. In the Svelte 5 portal they belong in pure `.svelte.ts` modules,
consistent with the project's decision to test view logic without Storybook.

| ID | Behaviour unit | Mockup implementation | Proposed real module | Responsibility |
|---|---|---|---|---|
| S01 | Catalog browser state | `ENTRIES`, `ecoFilter`, `query`, `render()` | `catalog-filter.svelte.ts` | Query, facets, sort, filtered result, counts, URL-param synchronisation |
| S02 | Endpoint snippet state | `SNIPPETS`, `#endpoint-tabs` click handler | `install-snippet.svelte.ts` | Selected ecosystem/tool and current snippet/note |
| S03 | Entry tag presenter | `tags(e)` | `entry-meta.ts` or part of `EntryCard` presenter | Pure conversion from entry fields to tag descriptors |
| S04 | Detail switcher | `showDetail(name)`, `#back` | Router + route `load` | Replaced by real URLs, route params, and detail JSON loading |
| S05 | Clipboard action | `[data-copy]` listeners | `copy.svelte.ts` / Svelte action | Clipboard write, fallback, transient confirmation |
| S06 | Search shortcut | global `keydown` for Cmd/Ctrl-K | Shell-level action | Focuses catalog search without stealing unrelated shortcuts |

Mockup functions also reveal formatting responsibilities that should become
pure utilities rather than components: human file size, relative time, short
hash display, count formatting, and ecosystem label lookup.

## 7. Data contracts implied by the mockup

### Mock entry object

```ts
{
  name, eco, version, summary, license, platforms,
  audit, coverage, updated, featured, deps, source,
  stac?, scripts?
}
```

This is broader than the current Conda-only `PackageCardSummary`. Portal schema
version 2 needs an `EntryCardSummary` with ecosystem/kind, source identity,
coverage, and ecosystem-specific optional summary data (STAC extent/thumbnail,
skill script presence, artifact kind).

### Other implied payloads

| Payload | Mockup usage | Current status |
|---|---|---|
| Universe identity and stats | Header and hero | Mostly exists; stats need per-ecosystem counts and last-build time |
| Universe endpoints/snippets | Hero endpoint panel | Exists for Conda only; replace fixed snippet fields with `InstallSnippet[]` |
| Capabilities | Badge and static/serve note | New schema required |
| Facets | Chips, platforms, categories, trust filters | Chip counts are hardcoded; must be derived or emitted with catalog data |
| Snapshot summary | Snapshot CTA and offline panel | New schema required |
| Artifact rows | Detail table generated from `platforms` with fake size/hash | Existing `VersionReleaseRow` covers Conda; generalise build fields |
| Dependencies | String tags | Needs typed internal/external references and version specs |
| Binary audit | Hardcoded CloudCompare rows | Existing `AuditReportView` is sufficient for Conda/native artifacts |
| Coverage | Percent strings only | New `CoverageSummary` fragment/view model required |
| Provenance | Hardcoded workflow/attestation/SBOM row | New `ProvenanceView` schema required |

## 8. Mockup-only constructs that must not become components as-is

The review found several shortcuts that should be corrected during extraction:

1. **Card semantics:** each card is a `<button>` containing headings, paragraphs,
   and div-like content. Use `<article>` with a stretched `<a>` instead.
2. **Navigation semantics:** top navigation uses `<button>`; real routes need
   anchors and current-page semantics.
3. **Detail is not entry-correct:** binary audit, provenance, recipe path, and
   install command are still Conda/CloudCompare-shaped regardless of selected
   entry. These sections need capability/emptiness rules per ecosystem.
4. **Filters are mostly decorative:** only ecosystem chips and text search work;
   platform, category, trust, and sort controls exist visually but have no state.
5. **Search scope is narrow:** it searches string concatenation of name,
   summary, and ecosystem only. Real search tokens/tags/categories need a defined
   index contract.
6. **Counts are intentionally inconsistent:** the hero and chips describe a
   128-entry universe while `ENTRIES` contains nine examples. Do not encode the
   displayed numbers as fixture truth.
7. **Copy logic depends on DOM position:** `firstChild.textContent` and stripping
   the word `copy` are prototype hacks. `CodeSnippet` must own a `copyText` prop.
8. **No pending states:** loading, error, stale snapshot, unavailable artifact,
   empty search screenshot, and mobile filter disclosure are absent.

## 9. Missing components for a production extraction

These are not extractable from the mockup, but the component architecture
should reserve slots for them:

- `EntryCardSkeleton` and `CatalogSkeleton`
- `ErrorState` and a formal `EmptyState`
- `ActiveFilterBar` with remove/clear actions
- `LoadMore` or pagination for sharded catalogs
- `MobileFilterDrawer`
- `VersionSelector`
- `DependencyLink` (internal package vs external specification)
- `CoverageTrend` (small chart; currently only a percentage pillar)
- `EvidenceProse` for sanitized evidence HTML
- `StacFootprintMap` (the mockup only has `FootprintThumbnail`)
- `SkillFileTree` and script-review notice for skill detail pages
- `StaleSnapshotNotice` driven by `Capabilities.generated_at`

## 10. Proposed Svelte file placement

```text
packages/portal/src/lib/
├── components/
│   ├── chrome/
│   │   ├── TopBar.svelte
│   │   ├── BrandMark.svelte
│   │   ├── MainNav.svelte
│   │   ├── CapabilityBadge.svelte
│   │   └── PortalFooter.svelte
│   ├── catalog/
│   │   ├── UniverseHero.svelte
│   │   ├── StatsStrip.svelte
│   │   ├── FilterRail.svelte
│   │   ├── FacetGroup.svelte
│   │   ├── SearchToolbar.svelte
│   │   ├── CatalogBrowser.svelte
│   │   ├── EntryCard.svelte
│   │   ├── EntryMetaTags.svelte
│   │   └── SnapshotCTA.svelte
│   ├── detail/
│   │   ├── EntryDetailHeader.svelte
│   │   ├── VerificationEvidence.svelte
│   │   ├── ArtifactMatrix.svelte
│   │   ├── DependencyList.svelte
│   │   ├── BinaryAuditTable.svelte
│   │   ├── ProvenancePanel.svelte
│   │   ├── AboutPanel.svelte
│   │   └── OfflineUsagePanel.svelte
│   └── install/
│       ├── CodeSnippet.svelte
│       ├── CopyButton.svelte
│       ├── SnippetTabs.svelte
│       ├── UniverseEndpointPanel.svelte
│       └── EntryInstallPanel.svelte
├── state/
│   ├── catalog-filter.svelte.ts
│   ├── install-snippet.svelte.ts
│   ├── platform-matrix.svelte.ts
│   └── copy.svelte.ts
└── format.ts
```

Shared single-purpose primitives (`EcosystemBadge`, `MetaTag`, `FilterChip`,
`MetricPillar`) can live beside their first major consumer until reuse proves a
separate `primitives/` folder is worthwhile. Avoid creating a generic
component for every CSS class: `.wrap`, `.section`, `.note`, and `.meta` are
styling/layout conveniences, not component boundaries.

## Extraction priority

1. **P0 — shell and browse:** `TopBar`, `CapabilityBadge`, `UniverseHero`,
   `UniverseEndpointPanel`, `FilterRail`, `SearchToolbar`, `EntryCard`,
   `CatalogBrowser`, and `catalog-filter.svelte.ts`.
2. **P0 — detail:** `EntryDetailHeader`, `EntryInstallPanel`, `ArtifactMatrix`,
   `VerificationEvidence`, and route-based detail loading.
3. **P1 — evidence:** `BinaryAuditTable`, coverage trend, `ProvenancePanel`,
   `SnapshotCTA`, and `OfflineUsagePanel`.
4. **P1 — robustness:** skeleton, error/empty states, active filters, mobile
   filter drawer, version selector.
5. **P2 — ecosystem-specific detail:** STAC footprint map/items and skill file
   tree/script review, once their schemas exist.
