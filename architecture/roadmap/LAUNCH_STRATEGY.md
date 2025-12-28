# utf8dok: 12-Week Open Source Launch Strategy

## The Strategic Frame

**Goal**: Transform utf8dok from internal tool to emerging industry standard in 12 weeks.

**The Playbook**: Infrastructure wins, not products. Git beat Subversion by becoming infrastructure. Docker beat VMs by becoming infrastructure. utf8dok beats Pandoc by becoming document infrastructure.

```
Week 1-3          Week 4-6          Week 7-9          Week 10-12
   │                 │                 │                  │
   ▼                 ▼                 ▼                  ▼
┌────────┐      ┌────────┐      ┌────────┐      ┌─────────────┐
│ZERO    │      │DOGFOOD │      │ECOSYSTEM│      │VISIBILITY   │
│FRICTION│  ──▶ │SHOWCASE│  ──▶ │SEEDS   │  ──▶ │& REVOLUTION │
└────────┘      └────────┘      └────────┘      └─────────────┘
  Ship it       Prove it        Extend it        Spread it
```

---

## Phase 1: Zero-Friction Release (Weeks 1-3)

**Objective**: Any engineer can adopt utf8dok in under 10 minutes.

### Week 1: Binary Distribution

| Task | Deliverable | Owner |
|------|-------------|-------|
| Cargo packaging | `cargo install utf8dok` works flawlessly | Core |
| Cross-compilation | Linux, macOS, Windows binaries | CI |
| GitHub Releases | Automated release workflow | CI |
| Version strategy | SemVer, CHANGELOG.md | Core |

```yaml
# .github/workflows/release.yml (target)
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    # Cross-compile, package, upload to GitHub Releases
```

### Week 2: Container & CI Integration

| Task | Deliverable | Owner |
|------|-------------|-------|
| Docker image | `ghcr.io/utf8dok/cli:latest` | Infra |
| GitHub Action | `uses: utf8dok/action@v1` | Infra |
| GitLab CI template | `.gitlab-ci.yml` snippet | Infra |
| Pre-commit hook | `.pre-commit-config.yaml` | Infra |

```yaml
# Target GitHub Action usage:
- uses: utf8dok/action@v1
  with:
    command: render
    input: docs/design.adoc
    output: output/design.docx
    profile: regulatory
```

### Week 3: Developer Experience Polish

| Task | Deliverable | Owner |
|------|-------------|-------|
| 5-minute quickstart | README + asciicast | Docs |
| Error messages | Rust-quality diagnostics | Core |
| `--help` excellence | Clap with examples | CLI |
| Shell completions | bash, zsh, fish, PowerShell | CLI |

**Exit Criteria**: Cold start to working DOCX in < 5 minutes.

---

## Phase 2: Dogfood Showcase (Weeks 4-6)

**Objective**: utf8dok documentation built with utf8dok. The ultimate proof.

### Week 4: Documentation Site Foundation

| Task | Deliverable | Owner |
|------|-------------|-------|
| Site structure | docs.utf8dok.dev skeleton | Docs |
| Build pipeline | AsciiDoc → HTML + DOCX | CI |
| Hosting | GitHub Pages or Netlify | Infra |
| Design system | Minimal, clean, fast | Design |

```
docs/
├── src/
│   ├── getting-started.adoc
│   ├── user-guide/
│   │   ├── extract.adoc
│   │   ├── render.adoc
│   │   └── diagrams.adoc
│   ├── reference/
│   │   ├── cli.adoc
│   │   ├── config.adoc
│   │   └── profiles.adoc
│   └── architecture.adoc
├── templates/
│   └── corporate.dotx
└── utf8dok.toml
```

### Week 5: Content & Examples

| Task | Deliverable | Owner |
|------|-------------|-------|
| Getting Started guide | 3 tutorials (basic, diagrams, CI) | Docs |
| Example repository | `utf8dok/examples` with 5+ use cases | Docs |
| API reference | Auto-generated from code | Core |
| Video walkthrough | 3-minute YouTube/Loom | Marketing |

**Example Repository Structure**:
```
examples/
├── 01-hello-world/          # Minimal example
├── 02-corporate-template/   # Template injection
├── 03-diagrams-mermaid/     # Diagram integration
├── 04-ci-github-actions/    # CI pipeline
├── 05-round-trip/           # Extract → Edit → Render
└── 06-regulatory-profile/   # Strict validation
```

### Week 6: Interactive Playground

| Task | Deliverable | Owner |
|------|-------------|-------|
| WASM build | utf8dok compiled to WASM | Core |
| Web playground | Try in browser (like Rust Playground) | Frontend |
| Shareable links | Encode source in URL | Frontend |
| Template gallery | Browse available templates | Frontend |

**Exit Criteria**: Complete documentation site live, built with utf8dok.

---

## Phase 3: Ecosystem Seeds (Weeks 7-9)

**Objective**: Enable others to extend utf8dok. Create network effects.

### Week 7: Plugin SDK & Architecture

| Task | Deliverable | Owner |
|------|-------------|-------|
| Plugin trait design | `utf8dok-plugin-sdk` crate | Core |
| Lint plugin system | Custom lints via Rhai scripts | Core |
| Template registry | Community template sharing | Infra |
| Plugin documentation | How to build plugins | Docs |

```rust
// crates/utf8dok-plugin-sdk/src/lib.rs
pub trait LintPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, document: &Document) -> Vec<Diagnostic>;
}

pub trait DiagramPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn supported_types(&self) -> &[&str];
    fn render(&self, source: &str, diagram_type: &str) -> Result<Vec<u8>>;
}
```

### Week 8: IDE Integration

| Task | Deliverable | Owner |
|------|-------------|-------|
| VS Code extension | Syntax highlighting + diagnostics | Tooling |
| LSP foundation | Basic language server | Core |
| Snippet library | Common patterns | Tooling |
| Marketplace publish | VS Code Marketplace listing | Tooling |

**VS Code Extension Features (MVP)**:
- Syntax highlighting for AsciiDoc
- Real-time diagnostics from utf8dok
- Snippet completion for common blocks
- "Render to DOCX" command

### Week 9: Integration Ecosystem

| Task | Deliverable | Owner |
|------|-------------|-------|
| Confluence exporter | utf8dok → Confluence | Integrations |
| Notion importer | Notion → AsciiDoc | Integrations |
| Slack bot (optional) | `/utf8dok render` | Integrations |
| MkDocs plugin | utf8dok in MkDocs sites | Integrations |

**Exit Criteria**: Third-party developers can build on utf8dok.

---

## Phase 4: Visibility & Revolution (Weeks 10-12)

**Objective**: Establish thought leadership. Plant the presentation seed.

### Week 10: Content Marketing Blitz

| Task | Deliverable | Owner |
|------|-------------|-------|
| Launch blog post | "Introducing utf8dok" | Marketing |
| Technical deep-dive | "The Document Compiler" architecture | Core |
| Comparison article | utf8dok vs Pandoc vs Asciidoctor | Docs |
| Hacker News launch | Coordinated submission | Team |

**Blog Post Calendar**:
```
Week 10, Day 1: "Introducing utf8dok: The Document Compiler"
Week 10, Day 3: "Why We Built utf8dok" (problem statement)
Week 10, Day 5: "utf8dok Architecture Deep Dive"
Week 11, Day 1: "Round-Trip Editing: How utf8dok Preserves Your Work"
Week 11, Day 3: "Diagrams-as-Code in Corporate Documents"
Week 12, Day 1: "The Future: Document-Driven Presentations"
```

### Week 11: Community Foundation

| Task | Deliverable | Owner |
|------|-------------|-------|
| Discord/Zulip server | Community chat | Community |
| Contributing guide | CONTRIBUTING.md with clear process | Core |
| Governance document | Decision-making transparency | Core |
| RFC process | Feature proposal template | Core |
| First external PR | Merge community contribution | Team |

**Governance Model (Lightweight)**:
```markdown
# GOVERNANCE.md

## Decision Making
- Small changes: Maintainer approval
- Medium changes: 2 maintainer consensus
- Large changes: RFC + community feedback

## Roles
- Maintainers: Core team (your company initially)
- Contributors: Anyone with merged PR
- Community: Discord members

## Roadmap
- Public roadmap in GitHub Projects
- Quarterly planning posts
```

### Week 12: The Presentation Seed

| Task | Deliverable | Owner |
|------|-------------|-------|
| AST presentation hints | `PresentationHint` in AST | Core |
| PPTX spike | Proof-of-concept export | Core |
| Vision document | "Document-Driven Presentations" manifesto | Strategy |
| Teaser demo | 30-second video showing potential | Marketing |

```rust
// Plant the seed in utf8dok-ast (Week 12)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresentationHint {
    /// This block is a candidate for a slide
    pub slide_candidate: bool,
    /// Suggested visual treatment
    pub visual_type: Option<VisualType>,
    /// Emphasis level for layout
    pub emphasis: EmphasisLevel,
    /// Speaker notes content
    pub speaker_notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VisualType {
    Diagram,
    Chart,
    Quote,
    CodeBlock,
    ImageGallery,
    Comparison,
}
```

**Exit Criteria**: utf8dok has public presence, community forming, and presentation foundation planted.

---

## Weekly Execution Rhythm

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         WEEKLY CADENCE                                   │
│                                                                          │
│  Monday        Tuesday       Wednesday     Thursday      Friday         │
│  ────────────────────────────────────────────────────────────────────── │
│  Planning      Build         Build         Build         Ship +         │
│  & Design      Sprint        Sprint        Sprint        Retrospective  │
│                                                                          │
│  • Week goals  • Code        • Code        • Code        • Merge PRs    │
│  • Blockers    • Tests       • Tests       • Docs        • Release      │
│  • Priorities  • Review      • Review      • Review      • Announce     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Success Metrics

### Quantitative (Week 12 Targets)

| Metric | Target | Measurement |
|--------|--------|-------------|
| GitHub stars | 500+ | GitHub API |
| Cargo downloads | 1,000+ | crates.io |
| Docker pulls | 500+ | ghcr.io |
| Discord members | 100+ | Discord |
| External PRs | 10+ | GitHub |
| Documentation pages | 30+ | Site |

### Qualitative

| Milestone | Evidence |
|-----------|----------|
| Mentioned in Rust Weekly | Newsletter link |
| First external blog post | Someone writes about utf8dok |
| First production user | Company using utf8dok in CI |
| Conference CFP accepted | Talk proposal accepted |

---

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Scope creep | High | High | Ruthless prioritization, defer to "Phase 2" |
| Quality sacrifice | Medium | High | CI gates, code review, no broken main |
| Community silence | Medium | Medium | Active Discord presence, respond to all issues |
| Burnout | Medium | High | Sustainable pace, clear ownership |
| Competitor launch | Low | Medium | Speed is the defense; ship early |

---

## Team Allocation (Suggested)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ROLE ALLOCATION                                  │
│                                                                          │
│  Core Development (60%)                                                 │
│  ├── Compiler/AST work                                                  │
│  ├── Diagram rendering                                                  │
│  ├── OOXML handling                                                     │
│  └── LSP/Plugin SDK                                                     │
│                                                                          │
│  DevOps/Infrastructure (20%)                                            │
│  ├── CI/CD pipelines                                                    │
│  ├── Release automation                                                 │
│  ├── Docker/GitHub Actions                                              │
│  └── Hosting                                                            │
│                                                                          │
│  Documentation/Community (20%)                                          │
│  ├── User guides                                                        │
│  ├── Examples                                                           │
│  ├── Blog posts                                                         │
│  └── Community management                                               │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## The One Thing Each Week

| Week | The One Thing That Must Ship |
|------|------------------------------|
| 1 | `cargo install utf8dok` works |
| 2 | GitHub Action published |
| 3 | 5-minute quickstart complete |
| 4 | docs.utf8dok.dev live |
| 5 | Examples repository with 5 use cases |
| 6 | Web playground functional |
| 7 | Plugin SDK crate published |
| 8 | VS Code extension in marketplace |
| 9 | First third-party integration |
| 10 | Launch blog post published |
| 11 | Discord community active |
| 12 | Presentation hints in AST |

---

## Post-12-Week Horizon

This 12-week sprint establishes the foundation. What follows:

**Months 4-6: Consolidation**
- Full LSP implementation
- Enterprise features (audit logs, compliance reports)
- Professional services offering

**Months 7-9: Expansion**
- PPTX generation (the TikTok-era vision)
- Additional format outputs (PDF, HTML, Confluence)
- International documentation (i18n)

**Months 10-12: Institutionalization**
- Conference speaking circuit
- University partnerships
- Standards body engagement
- Foundation consideration

---

## Starting Tomorrow

**Day 1 Checklist**:

```bash
# 1. Verify current crate structure
cargo build --release --all-features

# 2. Set up release workflow
# Create .github/workflows/release.yml

# 3. Write CHANGELOG.md with current state

# 4. Create GitHub Discussions for community

# 5. Reserve names:
#    - crates.io: utf8dok (if not already)
#    - Docker Hub / ghcr.io: utf8dok
#    - npm: @utf8dok (for future VS Code)

# 6. Draft the 5-minute quickstart in README
```

**The revolution starts with shipping.**
