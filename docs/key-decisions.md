# projectlighthouse - Key Decisions & Rationale

## Decision 1: Labs Over Video Courses

**Conclusion: Build labs, not videos**

### Why labs are more effective:
- **Forces building**: TCP server either works or it doesn't - no bullshitting
- **Active learning**: Debugging segfaults > watching someone debug
- **Higher retention**: Doing > watching
- **Better outcomes**: MIT 6.828, CMU 15-213 use labs, not videos
- **Matches mission**: "Be a builder" requires building

### Why video courses fail for systems programming:
- Passive consumption creates watchers, not builders
- False confidence ("I understand" ≠ "I can build")
- ~3-5% completion rate
- Hard to reference specific content
- Don't teach debugging (the real skill)

### The real learning path:
```
Read theory → Try to build → Fail → Debug → Understand → Build again → Learn
```
Videos skip the middle parts, which is where learning happens.

## Decision 2: CLI Validator First, Cloud Labs Later

**Conclusion: Build Rust CLI validator before cloud infrastructure**

### Progression strategy:
1. **Phase 1**: CLI validator (local testing)
2. **Phase 2**: Evaluate demand & completion rates
3. **Phase 3**: Cloud labs only if justified

### Why CLI first:

**Speed to market:**
- 2-3 weeks to build vs months for cloud infrastructure
- Validate exercise design quickly
- Get user feedback faster

**Cost:**
- $0/month vs $200-500+/month minimum
- No infrastructure to maintain while finding product-market fit
- Scale costs only when revenue justifies it

**Complexity:**
- Medium complexity vs high complexity
- One person can build and maintain
- No container orchestration, resource limits, cleanup jobs

**User experience:**
- Instant feedback (no network latency)
- Works offline
- Full control over environment
- Systems programmers should be comfortable with local dev anyway

**Risk mitigation:**
- Proves exercises are good before heavy investment
- Can still add cloud labs later if needed
- Validates that people actually complete exercises

### When to build cloud labs:
- Strong demand from users who can't/won't setup locally
- High exercise completion rates prove value
- Revenue to cover $200-500+/month infrastructure
- Exercises that truly need cloud (multi-machine networking, etc.)

## Decision 3: Rust for CLI Tool

**Conclusion: Use Rust, not Go**

### Why Rust:
- Single binary distribution (easier for users)
- Excellent async support (tokio) for network validators
- Strong process management
- Fast execution = better UX
- Dogfooding - teaching Rust by using Rust
- Cross-compilation is mature

### Why not Go:
- Both would work fine technically
- Rust better aligns with teaching Rust on platform
- Stronger type system helps with validator correctness

## Decision 4: Local-First, Cloud-Optional

**Philosophy: Default to local development**

### Why local-first wins:
1. **Target audience**: Systems programmers should handle local tooling
2. **Real-world skill**: Production work is local development
3. **Debugging practice**: Setting up toolchains is part of the learning
4. **Cost**: Free for users, free for platform
5. **Reliability**: No network dependencies

### Supporting local development:
- Provide Dockerfiles for consistent environments
- `lux doctor` checks local setup
- Good troubleshooting docs
- Most users already have Go/Rust installed

### Cloud as enhancement, not requirement:
- CLI works standalone
- Cloud labs can supplement specific exercises
- Hybrid approach possible (local default, cloud fallback)

## Core Principle

**"Would this have helped me learn to build?"**

Every feature decision should answer this question. If it's just infrastructure for infrastructure's sake, or passive consumption, it doesn't belong.

The goal is builders, not watchers or users.
