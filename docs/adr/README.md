# Decision records

One page per decision that shaped the GPU work, in the MADR shape: the problem, the choice, what it costs, and how we know it holds. Each record names its component in the front matter.

| ADR | decision |
|---|---|
| [0001](0001-set-0-binding-abi.md) | every resource lives in descriptor set 0 at fixed or dense bindings |
| [0002](0002-one-ssbo-window-for-global-memory.md) | guest global memory is one storage buffer window |
| [0003](0003-uniform-workgroup-scheduling-for-divergence.md) | divergent programs run workgroup-uniform with tracked reconvergence |
| [0004](0004-backing-memory-authority-for-coherence.md) | guest memory is the authority; host copies follow page generations |
| [0005](0005-hmma-exact-accumulation-as-dontunroll-loops.md) | tensor sums use the silicon model, emitted as DontUnroll loops |
| [0006](0006-synchronous-pusher-with-draw-snapshots.md) | the pusher runs synchronously and hands out draw snapshots |
| [0007](0007-t239-decodes-with-the-sm86-table.md) | the T239 decodes with the SM86 instruction table |
| [0008](0008-silicon-first-verification.md) | silicon captures are the reference, known failures are per device |
