# UTF8DOK Roadmap Schedule

> This file tracks the 90-day roadmap for post-PPTX development.
> **Last Updated:** 2026-01-01

## Current Phase: 24 - Data Engine (Week 1)

## Completed Phases

| Phase | Name | Status | Date |
|-------|------|--------|------|
| 1-13 | Compliance Platform (LSP) | ✅ Complete | 2025 |
| 20 | Workspace Intelligence | ✅ Complete | 2025 |
| 22 | PPTX Generation Crate | ✅ Complete | 2025-12 |
| 23 | Presentation Bridge | ✅ Complete | 2026-01-01 |

## In Progress

### Phase 24: Data Engine - Week 1
**Goal:** Implement `utf8dok-data` with `calamine` integration

| Task | Status | Notes |
|------|--------|-------|
| Create utf8dok-data crate | ⬜ Pending | |
| Define DataSource trait | ⬜ Pending | |
| Implement ExcelSource | ⬜ Pending | calamine wrapper |
| Implement TableConverter | ⬜ Pending | Range → AST Table |
| Integration tests | ⬜ Pending | tests/fixtures/simple.xlsx |

## 90-Day Schedule

### Month 1: Data Engine (Weeks 1-4)
- **Week 1:** Core crate, calamine integration, basic table conversion
- **Week 2:** Range parsing, cell type handling, error recovery
- **Week 3:** Include directive integration in parser
- **Week 4:** CLI integration, documentation, testing

### Month 2: Publishing Engine (Weeks 5-8)
- **Week 5:** `utf8dok-publish` crate, target abstraction
- **Week 6:** Confluence Storage Format generator
- **Week 7:** SharePoint/Graph API integration
- **Week 8:** Authentication, incremental updates

### Month 3: PDF Engine (Weeks 9-12)
- **Week 9:** `utf8dok-pdf` crate, Typst evaluation
- **Week 10:** Basic document rendering
- **Week 11:** Tables, images, code blocks
- **Week 12:** Themes, ToC, polish

## Architecture Decisions

| ADR | Title | Status |
|-----|-------|--------|
| ADR-010 | PPTX Dual-Nature Documents | ✅ Accepted |
| ADR-012 | Boring AST (Data Engine) | 📝 Proposed |

## Checkpoints

- [ ] **Checkpoint 1 (Week 1):** `cargo test -p utf8dok-data` passes
- [ ] **Checkpoint 2 (Week 4):** `include::file.xlsx[...]` works in CLI
- [ ] **Checkpoint 3 (Week 8):** `utf8dok publish --target confluence` works
- [ ] **Checkpoint 4 (Week 12):** `utf8dok render --format pdf` works

## Session Handoff Notes

When resuming development:
1. Check this file for current phase
2. Run `cargo test --workspace` to verify state
3. Continue from the next pending task
