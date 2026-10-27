---
id: doc-6
title: Universe documentation website layout and component analysis
type: design
created_date: '2026-10-27'
updated_date: '2026-10-27'
---

# Universe documentation website — layout & component analysis

**Design analysis only.** This document does not implement a website.

It applies the supplied Astro + Starlight documentation pattern to Universe, using the same extraction style as `doc-5 - Universe-portal-component-inventory.md`: foundations, primitives, composites, page sections, templates, behaviour/build units, contracts, gaps, and implementation priorities.

Inputs assumed:

- documentation site = **dev-repository only**;
- not bundled with `universe-kit`, Conda packages, release archives, or the future generated consumer portal;
- current product direction = headless Conda channel first, browser portal later;
- current CLI is still a skeleton and only exposes `version` today, so documentation must distinguish **available**, **v0.1 target**, and **roadmap** material;
- intended stack, if implemented later = standalone `website/` with Astro + Starlight, Bun/Pixi wrappers, GitHub Pages first.

## Summary

| Kind | Count | Meaning |
|---|---:|---|
| Design foundations | 7 | Tokens, layout rules, status language, metadata contracts |
| Primitive components | 12 | Small repeated building blocks |
| Composite components | 16 | Reusable controls/patterns made from primitives |
| Page-section components | 14 | Large content regions on landing, roadmap, overview, distribution, and reference pages |
| Shell and page templates | 6 | Custom shell plus Starlight page templates |
| Behaviour / build units | 11 | Remark/build/deployment/search behaviour that is not a visual component |

Total: **48 UI components**, supported by **7 foundations** and **11 behaviour/build units**.

Most of the documentation UI should be **adopted from Starlight**, not rebuilt: header, search, sidebar, table of contents, code blocks, tabs, steps, asides, cards, theme, pagination, edit link, and last-updated metadata. The custom design effort should concentrate on the landing page, roadmap page, brand tokens, status communication, and content model.

Design rule used below: something is named as a component only if it repeats, has a stable input contract, owns state/behaviour, or can be reviewed/tested independently. Ordinary Markdown paragraphs, one-off tables, and CSS spacing classes are not components.

## Proposed route and layout map

```text
/
├── /                              Custom landing page
├── /roadmap/                      Custom roadmap page
├── /docs/                         Docs overview
├── /getting-started/
│   ├── installation/
│   ├── quick-start/
│   ├── concepts/
│   ├── configuration/
│   └── updating/
├── /guides/
│   ├── first-universe-kit/
│   ├── github-pages-cep-channel/
│   ├── watch-upstream-releases/
│   ├── serve-locally/
│   └── inspect-binary-audit/
├── /architecture/
│   ├── overview/
│   └── security-model/
├── /distribution/
│   ├── prefix-dev/
│   ├── github-releases/
│   └── releasing/
└── /reference/
    ├── cli/
    ├── universe-toml/
    ├── stdout-protocol/
    └── exit-codes/
```

Changelog should remain an external GitHub Releases link until release notes are generated from an authoritative changelog source. A `/changelog/` route is a later automation candidate, not a v1 docs requirement.

## Component tree

```text
DocsSite
├── CustomSiteShell
│   ├── SiteHeader
│   │   ├── BrandMark
│   │   ├── PrimaryNav
│   │   └── ExternalLink: GitHub / prefix.dev
│   ├── LandingPage
│   │   ├── LandingHero
│   │   │   ├── EyebrowLabel
│   │   │   ├── InstallCommandCard
│   │   │   │   └── CopyButton
│   │   │   ├── VersionChip
│   │   │   └── CTALink ×3
│   │   ├── PillarSection
│   │   ├── OwnerWorkflowSection
│   │   ├── EntryKindsSection
│   │   │   └── KindBadge / StatusBadge
│   │   ├── EvidenceSection
│   │   ├── ChoosePathSection
│   │   ├── FAQSection
│   │   └── FinalCTASection
│   ├── RoadmapPage
│   │   ├── RoadmapPhasesSection
│   │   │   └── RoadmapPhaseCard ×3
│   │   └── GuardrailsSection
│   └── SiteFooter
├── StarlightDocPage
│   ├── SiteHeader (Starlight)
│   ├── DocsSidebar (Starlight)
│   ├── SearchControl (Starlight/Pagefind)
│   ├── OnThisPage (Starlight)
│   └── Article
│       ├── AsideCallout
│       ├── InstallTabs
│       ├── GuideSteps
│       ├── CodeSnippetBlock
│       ├── ReferenceTable
│       └── LinkCardGrid
├── DocsOverviewPage
│   └── DocsOverviewSection
└── NotFoundPage
```

## 1. Foundations

These belong in `brand.css`, `astro.config.mjs`, and authoring conventions — not in components.

| ID | Foundation | Proposed rule | Why it exists |
|---|---|---|---|
| F01 | Brand tokens | One `brand.css` defines accent, sky/violet gradient stops, surfaces, hairlines, text/muted colours, and Starlight overrides for both themes. | Keeps landing and docs visually identical without a design-system package. |
| F02 | Typography | Self-hosted Geist Variable for UI/prose and Geist Mono Variable for code, eyebrow labels, terminal text, and metadata. | Avoids runtime Google Fonts and matches a technical Rust CLI feel. |
| F03 | Surface system | Dark page background by default on custom pages; raised cards use hairline borders, 16–22 px radii, subtle vertical gradients; code/terminal surfaces are darker than cards. | Gives install/evidence material visual priority without heavy shadows everywhere. |
| F04 | Layout grid | Custom pages use a 1180 px max width with 20 px gutters. Docs use Starlight's sidebar/content/TOC rhythm. Prose stays near 70 ch; code and tables may use full article width. | Prevents documentation from becoming either a marketing page or an unreadably wide text dump. |
| F05 | Status taxonomy | `Available now`, `v0.1 target`, `Roadmap`, `Experimental`, `Deprecated`. Status must be text, not colour only. | Critical while the CLI is a skeleton: target commands must not be documented as shipped facts. |
| F06 | Metadata contract | Every page has a specific title + description. Site config owns default OG image, theme colour, canonical base, social links, and one canonical description. | Feeds search results, link previews, `llms.txt`, and page mirrors from one source. |
| F07 | Accessibility / performance baseline | Semantic landmarks, one `h1` per page, visible focus, real links for navigation, `<details>` for FAQ, minimal custom JS, no client framework. | Documentation should be fast, crawlable, and usable before JavaScript loads. |

### Responsive behaviour

| Viewport | Landing / roadmap | Documentation pages |
|---|---|---|
| ≤ 620 px | Header stacks; hero becomes one column; cards and workflow steps become one column; install command wraps with copy button below. | Starlight collapses navigation; article is single column; tables scroll horizontally when needed. |
| 621–900 px | Pillar/entry-kind grids use two columns; phase lists may use one column. | Sidebar and article prioritised; TOC may be hidden or moved by Starlight defaults. |
| > 900 px | Three/four-column grids; hero copy constrained; sections use wide visual rhythm. | Full docs rhythm: sidebar, article, right-hand “On this page”. |

### Visual hierarchy rules

- Gradient text is scarce: at most one phrase per custom page, normally “Universe” in the hero headline.
- Mono uppercase eyebrow labels identify page/section category; they never replace headings.
- Status colours are semantic: green = available/current focus, sky = next, violet/slate = later/roadmap, amber = caution.
- Ecosystem/kind colour must always be paired with text (`Software`, `Data`, `Models`, `Skills`).
- The first screen of every docs page must answer: what is this, who is it for, and is it shipped yet?

## 2. Primitive components

| ID | Component | Source if implemented | Props / variants | Notes |
|---|---|---|---|---|
| A01 | `BrandMark` | Custom SVG in Starlight `logo` + custom header | Logo asset, alt text, home href | Orbital package mark; separate simpler favicon variant. |
| A02 | `EyebrowLabel` | CSS class / tiny Astro component | Text | Mono uppercase section label, e.g. `OWNER WORKFLOW`. |
| A03 | `GradientText` | CSS utility | Inline text | Landing/roadmap only; not needed inside Starlight prose. |
| A04 | `StatusBadge` | Custom Astro/MDX pattern or CSS span | `available`, `v0.1-target`, `roadmap`, `experimental`, `deprecated` | Core docs-integrity primitive while the product is pre-release. |
| A05 | `VersionChip` | Custom Astro | Version, release state | Example: `target v0.1.0 · pre-release`. Fed by `PUBLIC_VERSION` where possible. |
| A06 | `CTALink` | Custom Astro/CSS | `primary`, `secondary`, `ghost`; href; external flag | Button-styled anchors only. Never use `<button>` for navigation. |
| A07 | `ExternalLink` | Starlight attrs / custom anchor convention | href, label, `target`, `rel` | Used for GitHub, Releases, prefix.dev. |
| A08 | `CopyButton` | Custom Astro island-free script on landing | Exact copy text | Only needed for the hero install snippet; Starlight code blocks already provide copying. |
| A09 | `KbdHint` | Starlight search UI / CSS | Shortcut label | Search affordance; do not build a custom command palette. |
| A10 | `KindBadge` | Custom span/CSS | `software`, `data`, `models`, `skills` | Entry taxonomy communication; always text-labelled. |
| A11 | `HeroStat` | Custom Astro | Value, label | Optional. Use only for real numbers; avoid fake marketing metrics before v0.1 exists. |
| A12 | `IconBadge` | Starlight icons / inline SVG | Icon, tone | Pillar and link-card markers. Decorative icons must be `aria-hidden`. |

## 3. Composite components

| ID | Component | Source if implemented | Contract / responsibility |
|---|---|---|---|
| M01 | `SiteHeader` | Starlight header for docs; custom equivalent for landing/roadmap | Brand, primary routes, search/theme on docs, GitHub and prefix.dev links. Keep nav to about five items. |
| M02 | `DocsSidebar` | Starlight `sidebar` config | Explicit groups in reader-journey order; autogenerated pages inside each group; external changelog link at the end. |
| M03 | `SearchControl` | Starlight/Pagefind built-in | Static full-text search. Do not replace with custom search for launch. |
| M04 | `OnThisPage` | Starlight built-in | H2–H3 table of contents. Imposes heading discipline on authors. |
| M05 | `InstallCommandCard` | Custom Astro | Terminal chrome, one primary install command, exact `copyText`, optional platform note. |
| M06 | `InstallTabs` | Starlight `Tabs` in MDX | Pixi / Conda / GitHub Release / Source. Use `syncKey` if install-method tabs repeat across pages. |
| M07 | `CodeSnippetBlock` | Starlight/Expressive Code fenced blocks | Language-tagged snippets with titles where helpful. Version examples use `%%VERSION%%`. |
| M08 | `GuideSteps` | Starlight `Steps` in MDX | Ordered workflows: install → declare → lint/plan → audit → build/publish → consume. |
| M09 | `AsideCallout` | Starlight asides | `note`, `tip`, `caution`, `danger`. Mandatory pre-release/status warning pattern. |
| M10 | `LinkCardGrid` | Starlight `CardGrid`, `Card`, `LinkCard` | Docs overview and “choose a path” navigation. |
| M11 | `StatusMatrix` | Markdown table pattern | Entry kinds/adapters/commands by status. Tables beat decorative matrices for reference accuracy. |
| M12 | `ReferenceTable` | Markdown table pattern | CLI commands, config fields, exit codes, stdout envelope fields. Should later be generated where an authoritative source exists. |
| M13 | `FlowDiagram` | Static HTML/SVG in Astro or Markdown code-art | Recipe → plan → build/audit fragments → generation → index → target. No runtime diagram library at launch. |
| M14 | `FAQAccordion` | Native `<details>/<summary>` | “Is this bundled?”, “Do consumers need Universe?”, “Where do blobs live?” Works without JS. |
| M15 | `ReleaseChannelCard` | Custom/MDX card pattern | prefix.dev, GitHub Releases, source checkout: audience, install command, verification rule. |
| M16 | `RoadmapPhaseCard` | Custom Astro | Phase label, promise, bullets, non-goals. Now / Next / Later. |

## 4. Page-section components

| ID | Section | Page | Job | Main inputs |
|---|---|---|---|---|
| O01 | `LandingHero` | `/` | Say “A Universe in Your Pocket”, explain the one-sentence value, show install command and version/status. | Canonical description, install command, version, CTA routes |
| O02 | `PillarSection` | `/` | Three design pillars: static by default, evidence not vibes, offline is a shape. | Static copy; icon badges |
| O03 | `OwnerWorkflowSection` | `/` | Four-step owner lane: declare, plan/audit, build, publish coordinate. | Workflow copy; later command names |
| O04 | `EntryKindsSection` | `/` | Software now; data/models/skills later, with status badges. | Entry-kind taxonomy + roadmap status |
| O05 | `EvidenceSection` | `/` | Explain fragments: binary audit, licence, coverage, provenance, hashes, unknowns. | Architecture/security source pages |
| O06 | `ChoosePathSection` | `/` | Route readers by intent: install, first universe, architecture, roadmap. | Link cards |
| O07 | `FAQSection` | `/` | Correct likely misconceptions, especially docs-site vs generated portal. | FAQ pairs |
| O08 | `FinalCTASection` | `/` | One closing action: quick start or prefix.dev. | CTA links |
| O09 | `RoadmapPhasesSection` | `/roadmap/` | Now v0.1 headless Conda; next generated portal; later data/models/skills/offline snapshots. | Curated roadmap summary |
| O10 | `GuardrailsSection` | `/roadmap/` | Non-goals: not a hosted registry, not an adapter zoo, not a background service, not evidence theatre. | Decision summaries |
| O11 | `DocsOverviewSection` | `/docs/` | Explain site structure and the docs-site vs consumer-portal boundary. | Card grid + status note |
| O12 | `DistributionSection` | distribution pages | Compare owner-tool distribution with universe blob distribution. | Release channels and asset names |
| O13 | `CLIReferenceSection` | `/reference/cli/` | Commands with status, stdout rules, examples. | Present: manual. Future: generated from `--help` |
| O14 | `SchemaReferenceSection` | `/reference/universe-toml/` | Field tables and examples. | Present: manual summary. Future: generated from Rust schema |

## 5. Shell and page templates

| ID | Template | Source if implemented | Layout contract |
|---|---|---|---|
| T01 | `CustomSiteShell` | Custom Astro layout | Shared `<head>` SEO, header, footer, brand CSS for landing and roadmap only. |
| T02 | `LandingPage` | `src/pages/index.astro` | Full-width marketing/docs hybrid; server-rendered HTML; one copy-button script maximum. |
| T03 | `RoadmapPage` | `src/pages/roadmap.astro` | Same shell; phase cards and guardrails; explicit “not a compatibility promise” copy. |
| T04 | `StarlightDocPage` | Starlight default | Sidebar + article + TOC + pagination + edit link + last updated. Main documentation workhorse. |
| T05 | `DocsOverviewPage` | Starlight doc/splash content page | Short orientation page with cards; not a second landing page. |
| T06 | `NotFoundPage` | Starlight default or light customisation | Helpful links back to docs overview, installation, and roadmap. Keep low priority. |

### Documentation article anatomy

Every Starlight article should follow the same internal layout:

1. **Title + description** — specific enough to work as a search result and `llms.txt` summary.
2. **Status aside when relevant** — pre-release, target, or roadmap warning before commands appear.
3. **Outcome paragraph** — what the reader will understand or be able to do.
4. **Body pattern by page type**:
   - getting started: short explanation → example → next link;
   - guide: prerequisites → `Steps` → verification → troubleshooting/next;
   - architecture: claim → diagram/table → invariants → trade-offs;
   - reference: status key → tables → examples → edge cases.
5. **Cross-links at point of need** — concepts link to reference; guides link to architecture; reference links back to guides.
6. **Generated footer furniture** — edit link, last updated, prev/next from Starlight.

### Landing page anatomy

```text
┌──────────────────────────────────────────────────────────────┐
│ header: logo · Install · Docs · Roadmap · prefix.dev · GitHub │
├──────────────────────────────────────────────────────────────┤
│ eyebrow: Rust CLI · static-first · self-hosted                │
│ H1: A Universe in Your Pocket.                                │
│ lead: software, data, models, skills...                       │
│ terminal install card + Copy                                  │
│ [Get started] [Read concepts] [GitHub]     version/status     │
├──────────────────────────────────────────────────────────────┤
│ pillars: Static by default | Evidence | Offline is a shape   │
├──────────────────────────────────────────────────────────────┤
│ owner workflow: 01 declare → 02 plan/audit → 03 build → 04 publish │
├──────────────────────────────────────────────────────────────┤
│ entry kinds: Software v0.1 | Data roadmap | Models | Skills   │
├──────────────────────────────────────────────────────────────┤
│ evidence + choose-a-path cards + FAQ + final CTA              │
└──────────────────────────────────────────────────────────────┘
```

## 6. Behaviour and build units

These are “invisible components”: build-time generators and small runtime behaviours. In an Astro implementation they should stay isolated so content authors do not hand-maintain generated outputs.

| ID | Unit | Proposed owner | Responsibility |
|---|---|---|---|
| B01 | `VersionRewriter` | Remark plugin | Replace `%%VERSION%%` in text/inline code/fenced code from `PUBLIC_VERSION`; must also flow into `.md` mirrors. |
| B02 | `SiteConfig` | `astro.config.mjs` + small constants module | Title, canonical description, repo URL, channel URL, version fallback, site/base env handling. |
| B03 | `CanonicalAndSitemap` | Astro/Starlight built-in | Emits canonicals, `sitemap-index.xml`, `sitemap-0.xml`; respects GitHub Pages base path. |
| B04 | `RobotsEmitter` | Static file or generated endpoint | Allows search + named AI crawlers; ends with absolute Sitemap URL. Prefer generated if base/domain can change. |
| B05 | `LLMMirrorGenerator` | `starlight-llms-markdown` | Emits `llms.txt`, `llms-full.txt`, section indexes, `/<slug>.md`, and `/<slug>/index.md`. Never hand-write these. |
| B06 | `LandingStructuredData` | Custom page `<head>` | `SoftwareApplication` JSON-LD: version, OS, download URL, repo, licence, feature list. |
| B07 | `SearchIndex` | Starlight/Pagefind | Builds static search index; works hosted over HTTP, not a product feature. |
| B08 | `ThemePreference` | Starlight built-in | Light/dark selection and persistence. Custom pages may launch dark-first but must not fight docs theme tokens. |
| B09 | `InstallCopyAction` | Tiny inline script on landing only | Clipboard write + transient “Copied” label; exact text passed as data, not scraped from DOM. |
| B10 | `OutputVerifier` | Docs workflow | Asserts sitemap, robots, OG image, `llms*.txt`, one known `.md` mirror, canonical prefix, and no leaked `%%VERSION%%`. |
| B11 | `FutureReferenceGenerators` | Reserved pipeline | Later generate CLI reference from `--help`, config tables from Rust schema, changelog from release notes. Not launch scope. |

Version pinning is part of B01/B05 risk control: Astro/Starlight/plugin versions must be locked together, because Markdown processor and plugin APIs move quickly. The build is not acceptable until B10 passes.

## 7. Content and data contracts

| Contract | Source of truth | Consumers | Notes |
|---|---|---|---|
| Canonical description | Site config | Meta description, OG/Twitter, `llms.txt` blockquote, landing lead | Recommended: “Rust toolkit for building self-hosted universes of software, data, models, and skills. Publish anywhere—and take them offline.” |
| Doc frontmatter | Each Markdown/MDX file | Starlight title/meta/sidebar, sitemap, LLM mirrors | Required: `title`, one-sentence `description`, `sidebar.order`. Recommended extension later: `status`. |
| Release version | Release tag at deploy; package fallback during development | `%%VERSION%%`, version chip, JSON-LD | Strip leading `v` once, centrally. |
| Install commands | Distribution design: prefix.dev + Releases | Hero, installation tabs, distribution pages | Keep identical strings across pages; review as API surface. |
| CLI commands/status | Rust CLI help (future generated) | CLI reference, quick start, guides | Today only `version` is available. Everything else needs a status label. |
| `universe.toml` fields | Rust schema types | Configuration + reference | Docs must say Rust types are authoritative until generation exists. |
| Roadmap bullets | Curated from backlog decisions | Roadmap page, entry-kind statuses | Do not auto-import the whole backlog into public docs. |
| OG image | Committed static PNG + regeneration script | Social previews | 1200×630; regenerate only when brand/version changes. |

## 8. Navigation and information architecture

| Sidebar group | Reader job | Representative pages | Dominant components |
|---|---|---|---|
| Docs overview | Orient and choose a path | `/docs/` | `LinkCardGrid`, status aside |
| Getting Started | Install and learn the model | installation, quick start, concepts, configuration, updating | `InstallTabs`, `GuideSteps`, asides |
| Guides | Complete an owner task | first universe, GitHub channel, watcher, local use, binary audit | `GuideSteps`, `FlowDiagram`, tables |
| Architecture | Understand invariants | overview, security model | `FlowDiagram`, plane/invariant tables |
| Distribution | Get or release the tool | prefix.dev, GitHub Releases, releasing | `ReleaseChannelCard`, asset tables |
| Reference | Look up exact contracts | CLI, `universe.toml`, stdout protocol, exit codes | `StatusMatrix`, `ReferenceTable`, snippets |
| Roadmap / Changelog | See what is not shipped yet | custom roadmap; external releases | `RoadmapPhaseCard`, external link |

Navigation rules:

- Sidebar order follows the reader journey, not the repository folder structure.
- “Roadmap” is visible from the top header because pre-release honesty is part of the product story.
- Prefix.dev and GitHub are external actions, visually secondary to Docs.
- The generated consumer portal is never linked as if it were this documentation site.

## 9. Adopt, customise, or avoid

### Adopt from Starlight — do not rebuild

- page shell, sidebar, mobile navigation, search, TOC, theme picker;
- code highlighting and copy buttons inside docs;
- `Tabs`, `Steps`, asides, `Card`, `LinkCard`, `FileTree`;
- edit links, last-updated dates, prev/next pagination;
- sitemap/canonical generation.

### Customise lightly

- `brand.css` tokens and Starlight accent variables;
- logo, favicon, default OG/Twitter image, theme colour;
- sidebar labels/order and external changelog link;
- default head metadata;
- LLM plugin section labels and optional external links.

### Build custom only where Starlight is the wrong tool

- landing page and roadmap page shells/sections;
- hero install terminal and copy action;
- status/entry-kind visual language;
- static architecture flow visual;
- generated `robots.txt` if domain/base must be environment-driven.

### Avoid at launch

- custom search, custom sidebar, custom TOC, custom code renderer;
- React/Svelte islands for documentation content;
- a component library package shared with the future consumer portal;
- analytics/tracking by default;
- generated portal data, live GitHub API calls, or release fetching at request time;
- hand-maintained `llms.txt` or `.md` mirrors.

## 10. Proposed file placement if implemented later

This is a placement recommendation only; it should not be created as part of this analysis.

```text
website/
├── astro.config.mjs
├── package.json
├── bun.lock
├── README.md
├── public/
│   ├── favicon.svg
│   └── og.png
├── scripts/
│   └── generate-og.mjs
└── src/
    ├── assets/
    │   └── logo.svg
    ├── content.config.ts
    ├── content/docs/
    │   ├── docs.mdx
    │   ├── getting-started/
    │   ├── guides/
    │   ├── architecture/
    │   ├── distribution/
    │   └── reference/
    ├── layouts/
    │   └── CustomPage.astro
    ├── pages/
    │   ├── index.astro
    │   ├── roadmap.astro
    │   └── robots.txt.ts        (optional generated endpoint)
    ├── plugins/
    │   └── remark-version.mjs
    ├── site.ts
    └── styles/
        └── brand.css

.github/workflows/docs.yml       # SHA-pinned Pages build/deploy
pixi.toml                        # docs-dev/docs-build/docs-check wrappers only
```

Boundary rules for that placement:

- `website/` is standalone; exclude it from root Bun workspaces, Turbo, Cargo, product `check`, Conda recipe inputs, and release archives.
- Product CI may ignore `website/**`; a dedicated docs workflow owns website verification.
- Workflow actions must use 40-character SHAs to satisfy the repository's existing policy.
- Pixi tasks are wrappers only; adding tasks must not alter product environments or lockfile semantics beyond task definitions.

## 11. Gaps and open design decisions

1. **No visual mockup exists yet for the docs site.** This analysis defines structure and contracts; a throwaway HTML mockup would still help before implementation, as it did for the portal.
2. **Brand mark is not final.** The orbital-package logo, favicon, and OG art need one approved source asset before being reused in README, portal, and docs.
3. **Name tension:** repository/product today is `universe-kit`, while the broader direction is “Universe”. The site title can be “Universe”, but install/distribution copy must keep saying `universe-kit` until a rename actually happens.
4. **CLI reference cannot be generated meaningfully yet.** With only `version` implemented, generated help would be honest but nearly empty. Launch with a status table; add generation when commands land.
5. **Schema reference drift risk.** Manual `universe.toml` tables will drift unless Rust schema generation eventually feeds the docs or CI checks field names.
6. **Astro/Starlight/plugin version coupling.** The Markdown processor, Starlight, and `starlight-llms-markdown` must be pinned and smoke-tested together; output assertions matter more than “build succeeded”.
7. **Base-path correctness is a launch blocker.** GitHub Pages project sites require `site` + `base`; canonicals, sitemap, robots, OG URL, internal links, and `llms.txt` links must all be checked under `/universe-kit/`.
8. **Custom-domain decision changes only config, not content.** If a domain is adopted later, update environment values, rebuild, verify discovery surfaces, then switch DNS and repository homepage.
9. **Search quality depends on headings/descriptions.** Pagefind cannot rescue vague titles. Frontmatter linting should eventually enforce description length and uniqueness.
10. **Docs-site vs portal language must stay explicit.** Every boundary page — docs overview, roadmap, architecture overview — should repeat that this site is not bundled and not the generated consumer portal.

## Implementation priority, when approved

1. **P0 — skeleton and integrity:** standalone config, `brand.css`, Starlight shell, sidebar, docs overview, installation, concepts, CLI status reference.
2. **P0 — landing:** hero, install card, pillars, owner workflow, entry-kind statuses, FAQ boundary copy.
3. **P1 — discovery:** sitemap/canonical checks, robots, OG image, JSON-LD, LLM mirrors, output verifier, Pages workflow.
4. **P1 — content depth:** guides, architecture/security, distribution, `universe.toml`, stdout protocol, exit codes.
5. **P2 — automation:** generated CLI help, generated schema tables, changelog generation, link/frontmatter linting.
