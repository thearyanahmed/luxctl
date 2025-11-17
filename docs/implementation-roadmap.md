# projectlighthouse - Implementation Roadmap

### CLI Core
**Goal: Working test runner framework**

Tasks:
- [ ] Set up Rust project with clap for CLI parsing
- [ ] Implement basic commands (init, test, doctor)
- [ ] Build process runner (spawn user binaries, capture output)
- [ ] Create Validator trait and TestResults structure
- [ ] Add pretty output (colored, indicatif for progress)
- [ ] Set up exercise registry structure

**Deliverable**: CLI that can run a dummy validator

### API Client for projectlighthouse.io 

Tasks:
- [ ] Implement API client module
- [ ] Authentication handling (API keys)
- [ ] Store data securely (config file) in `~/.lux/` directory


### Define test structure

Tasks:
- [ ] Define a schema for exercises and tests (YAML/JSON)
- [ ] Create sample exercise with multiple tests
- [ ] Add 2 more exercises:
  - HTTP request parser
  - Memory allocator (basic)
- [ ] Build for multiple platforms (Linux, macOS x86/ARM, Windows)
- [ ] Create installation instructions
- [ ] Write troubleshooting guide
- [ ] Beta test with 5-10 users
- [ ] Fix issues from beta feedback

### More Exercises
- [ ] Port scanner
- [ ] Simple DNS resolver
- [ ] Thread pool / work-stealing scheduler
- [ ] Key-value store with persistence
- [ ] Container runtime basics

### Platform Integration
- [ ] Update book pages to reference CLI exercises
- [ ] Add "Try this exercise" CTAs in content
- [ ] Create dedicated exercises index page
- [ ] Track which exercises users complete (optional analytics)

### Community Building
- [ ] Add project showcase feature on site
- [ ] Create forum/discussions for exercises
- [ ] Share completed projects from users

### Metrics to Track
- CLI download count
- Exercise completion rates
- User feedback on difficulty
- Which exercises are most popular
- Where users get stuck (common errors)

### Decision Point: Cloud Labs?
**Only build if:**
- ✓ High exercise completion rate (>40%)
- ✓ User requests for cloud environments
- ✓ Revenue to cover infrastructure ($500+/month)
- ✓ Exercises that need cloud (multi-machine, etc.)

**If yes, build:**
- Docker orchestration service
- Browser-based terminal (xterm.js)
- Session management
- Resource limits & cleanup

**If no:**
- Double down on local exercises
- Add more complex local labs
- Improve existing validators
- Focus on content quality

## Platform Features (Parallel Track)

### Quick Wins (Week 1-2)
- [ ] Copy button on code blocks
- [ ] Syntax highlighting for Go/Rust/C
- [ ] Dark mode toggle
- [ ] Table of contents sidebar
- [ ] Reading progress tracking

### Medium Effort (Month 2)
- [ ] Export notes as markdown
- [ ] Code diff viewer
- [ ] Search autocomplete
- [ ] Keyboard shortcuts

### Nice to Have (Month 3+)
- [ ] Related pages suggestions
- [ ] Interactive diagrams
- [ ] Flashcards from notes

## Success Metrics

**Month 1:**
- CLI released and downloadable
- 50+ downloads
- 3+ exercises available
- 10+ users completing first exercise

**Month 3:**
- 200+ CLI downloads
- 10+ exercises
- 50+ exercise completions
- User testimonials / feedback
- Decide on cloud labs investment

**Month 6:**
- 500+ active users
- 15-20 exercises
- Project showcase with 20+ submissions
- Clear path to monetization (if desired)

## Risk Mitigation

**Risk: Exercises too hard**
- Start simple, increase difficulty gradually
- Get feedback early and often
- Provide extensive hints system
- Show example solutions after attempts

**Risk: Setup friction**
- Excellent `lux doctor` command
- Docker fallback for environment
- Video walkthroughs for setup
- Active support in forums

**Risk: Low engagement**
- Make exercises genuinely useful
- Tie to real-world projects
- Showcase completed work
- Gamification (badges, streaks)

**Risk: Can't maintain solo**
- Keep scope tight initially
- Automate testing/validation
- Community contributions for exercises
- Focus on quality over quantity

## Next Action
**Start Week 1 tasks:** Set up Rust CLI project with clap
