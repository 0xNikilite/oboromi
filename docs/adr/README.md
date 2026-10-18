# Decision records

A decision record, or ADR, explains one design choice: the problem, the choice, and what it costs. Each one opens with a short "In plain words" section, then the details. For example, [ADR-0004](0004-backing-memory-authority-for-coherence.md) explains why emulated RAM is the one true copy of all data.

| ADR | Decision |
|---|---|
| [0001](0001-set-0-binding-abi.md) | Every shader finds its data in one fixed table of numbered slots |
| [0002](0002-one-ssbo-window-for-global-memory.md) | Game memory reaches a shader through one window buffer |
| [0003](0003-uniform-workgroup-scheduling-for-divergence.md) | Threads that take different branches run in turns |
| [0004](0004-backing-memory-authority-for-coherence.md) | Emulated RAM is the one true copy; host copies check a write counter |
| [0005](0005-hmma-exact-accumulation-as-dontunroll-loops.md) | Matrix sums copy the chip exactly, written as small loops |
| [0006](0006-synchronous-pusher-with-draw-snapshots.md) | GPU commands run one after another, each draw with frozen settings |
| [0007](0007-t239-decodes-with-the-sm86-table.md) | The Switch 2 chip decodes with the desktop instruction table |
| [0008](0008-silicon-first-verification.md) | Real chips are the reference, known differences are listed per device |

Each record follows the MADR template and names its component in the front matter.
