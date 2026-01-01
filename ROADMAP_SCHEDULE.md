# UTF8DOK Roadmap Schedule

> This file tracks the 90-day roadmap for post-PPTX development.
> **Last Updated:** 2026-01-01

## Current Phase: 25 - Publishing Engine (Week 1)

## Completed Phases

| Phase | Name | Status | Date |
|-------|------|--------|------|
| 1-13 | Compliance Platform (LSP) | ✅ Complete | 2025 |
| 20 | Workspace Intelligence | ✅ Complete | 2025 |
| 22 | PPTX Generation Crate | ✅ Complete | 2025-12 |
| 23 | Presentation Bridge | ✅ Complete | 2026-01-01 |
| 24 | Data Engine (Month 1) | ✅ Complete | 2026-01-01 |

## In Progress

### Phase 25: Publishing Engine - Week 1
**Goal:** Create utf8dok-publish crate with target abstraction

| Task | Status | Notes |
|------|--------|-------|
| Create utf8dok-publish crate | ⬜ Pending | Workspace addition |
| Define PublishTarget trait | ⬜ Pending | Abstract publish interface |
| Confluence Storage Format | ⬜ Pending | XHTML generation |
| SharePoint integration | ⬜ Pending | Graph API setup |
| Basic authentication | ⬜ Pending | OAuth/token support |

### Phase 24: Data Engine - Complete ✅

| Week | Tasks | Status |
|------|-------|--------|
| Week 1 | Core crate, ExcelSource, TableConverter | ✅ Done |
| Week 2 | Extended range syntax, CSV, date handling | ✅ Done |
| Week 3 | Include directive integration in parser | ✅ Done |
| Week 4 | CLI integration, documentation, testing | ✅ Done |

### Week 3 Completed ✅

| Task | Status | Notes |
|------|--------|-------|
| Parse include::file.xlsx[...] | ✅ Done | IncludeDirective parser |
| Wire data engine to parser | ✅ Done | resolve_data_include() |
| Table insertion in AST | ✅ Done | Block::Table from data |
| Error handling/diagnostics | ✅ Done | Placeholder paragraphs |
| Integration tests | ✅ Done | 8 tests in include_tests.rs |

### Week 2 Completed ✅

| Task | Status | Notes |
|------|--------|-------|
| Extended range syntax | ✅ Done | RangeSpec enum (A:C, 1:10, *, A1) |
| Date/time formatting | ✅ Done | Excel serial → ISO 8601 |
| CSV data source | ✅ Done | CsvSource + TSV/semicolon |
| Auto-detect file type | ✅ Done | DataEngine.read_table_auto() |
| Test coverage | ✅ Done | 40 tests (30 unit + 10 integration) |

### Week 1 Completed ✅

| Task | Status | Notes |
|------|--------|-------|
| Create utf8dok-data crate | ✅ Done | Commit 034882a |
| Define DataSource trait | ✅ Done | sources/mod.rs |
| Implement ExcelSource | ✅ Done | calamine 0.32 wrapper |
| Implement TableConverter | ✅ Done | Range → AST Table |
| Integration tests | ✅ Done | 24 tests (14 unit + 10 integration) |

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

- [x] **Checkpoint 1 (Week 1):** `cargo test -p utf8dok-data` passes ✅ 24 tests
- [x] **Checkpoint 2 (Week 4):** `include::file.xlsx[...]` works in CLI ✅ 53 tests
- [ ] **Checkpoint 3 (Week 8):** `utf8dok publish --target confluence` works
- [ ] **Checkpoint 4 (Week 12):** `utf8dok render --format pdf` works

## Session Handoff Notes

When resuming development:
1. Check this file for current phase
2. Run `cargo test --workspace` to verify state
3. Continue from the next pending task
