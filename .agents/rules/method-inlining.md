# Method Inlining

Apply the canonical attribute policy in [performance-and-readability.md](performance-and-readability.md). Keep inlining decisions scoped to actual call paths and preserve architecture checks for ALU, CCR, and cold exception handlers.

Cross-crate accessors and forwarding helpers are candidates for `inline`; CPU register accessors and ultra-hot ALU/CCR helpers use `inline(always)`. Attribute presence alone does not demonstrate a speedup.

For the decision matrix and measured evaluation, use [profile-external](../skills/profile-external/SKILL.md) and its [inlining examples](../skills/profile-external/references/inlining-examples.md).
