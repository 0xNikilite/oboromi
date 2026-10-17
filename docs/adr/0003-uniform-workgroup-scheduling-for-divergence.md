---
status: "accepted"
date: 2026-10-17
decision-makers: jopdorp
component: [oboromi-gpu]
---
# Workgroup-uniform scheduling for divergent warps

## Context and Problem Statement

The translator runs each lane as a loop over a guest pc. Warp collectives (VOTE, SHFL, MATCH, REDUX, tensor ops) and BAR trade data through workgroup barriers, which every invocation must reach together. With lanes at different pcs that is undefined behaviour, and partial-mask votes could not be replayed.

## Considered Options

* Structurize the guest control flow into Vulkan loops and selections.
* Native subgroup operations only.
* Keep the pc loop and schedule it uniformly across the workgroup.

## Decision Outcome

Keep the pc loop. A program with collectives or BAR steps every invocation together: each step the workgroup votes the lowest runnable pc, lanes at that pc run it, the rest sit out with memory effects masked. BSSY and BSYNC tokens are tracked per lane, and BAR arrivals per barrier id until the required lanes arrive. Programs without collectives keep the free per-lane loop. Shared-memory exchange is the reference everywhere.

### Consequences

* Good, because divergent votes, shuffles and barriers match silicon, including the partial-mask cases.
* Good, because it runs on lavapipe, whose subgroups are 8 lanes wide.
* Bad, because programs with collectives pay a vote and a barrier per step.
* Bad, because fairness under spin loops is not verified.

### Confirmation

The warp scoreboard passes on Intel and lavapipe, and tests cover unequal branch lengths, nested tokens and early exit.

Sources: core/src/gpu/sm86/mod.rs `translate_program`, design note on divergence.
