# projectlighthouse - Feature Roadmap

## Existing Features
- Text-based courses (books with pages/lessons)
- Commenting system
- Note-taking against selected text
- Admin-only access (behind middleware)

## Planned Core Features (Immediate)

### CLI Validator Tool
**Priority: HIGH - Build First**
- Local validation tool written in Rust
- Tests user solutions without cloud infrastructure
- Instant feedback loop
- Similar to Codecrafters but self-validated

### Enhanced Reading Experience
- Copy button on code blocks with syntax highlighting (Go/Rust/C/Assembly)
- Table of contents sidebar (auto-generated from headers)
- Reading progress bar + "pick up where you left off"
- Anchor links to specific paragraphs
- Keyboard shortcuts (←/→ navigation, / for search, Ctrl+D for dark mode)
- Estimated read time per page
- Dark/light themes

### Content Tools
- Download/export notes as markdown
- Code diff viewer for before/after examples
- Inline terminology tooltips
- Related pages suggestions
- Page dependencies/prerequisites markers

### Search & Navigation
- Search autocomplete with suggestions
- Full-text search across books/pages
- Search within personal notes

## Deferred Features (After CLI Validation)

### Cloud Labs (Phase 2)
**Build only after CLI proves demand**
- Browser-based coding environments
- Docker container per session
- Pre-configured toolchains
- Embedded terminal
- **Cost consideration**: $200-500+/month minimum
- **Complexity**: High - requires orchestration

### Community Features (Phase 3)
- Project showcase gallery
- Discussion forums by topic
- Code review / peer feedback
- Q&A format

## Explicitly Rejected
- Video courses (passive learning, low completion rates)
- JavaScript-first approach
- Generic framework tutorials
- Anything that doesn't force building
