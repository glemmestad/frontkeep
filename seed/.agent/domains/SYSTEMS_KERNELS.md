# Systems & kernels domain overlay

Pulled in when the work is low-level systems: OS/kernel code, drivers, embedded
firmware, RTOS, bootloaders, or anything `no_std`/bare-metal. There is no runtime
to catch you here — a mistake is memory corruption, a hang, or a brick, and the
debugger may not exist.

## Memory & safety
- Account for every pointer and buffer: bounds, lifetime, ownership, alignment. Out-of-bounds and use-after-free here corrupt silently, then crash somewhere unrelated.
- Treat all hardware/peripheral input as untrusted and bound it. A device register can return anything; a DMA buffer can be the wrong size.
- Minimize and isolate `unsafe`/raw memory access. Where it's unavoidable, document the invariant it upholds and the conditions that would break it.

## Concurrency & interrupts
- Interrupt handlers run concurrently with everything: keep them short, reentrancy-safe, and free of blocking calls or allocation. Guard shared state with the right primitive (disable interrupts, lock-free, or a real lock — know which).
- Beware priority inversion and deadlock in an RTOS. State the locking order and stick to it.
- Don't assume atomicity. A read-modify-write on a shared register is a race unless you made it atomic.

## Resources & real-time
- No dynamic allocation on a hot or interrupt path unless the platform guarantees it's safe; prefer static/pool allocation. A heap on an embedded target is a footgun.
- Bound the stack — deep recursion or large stack frames overflow without warning. Budget it.
- For real-time work, state the timing deadline and show the worst-case path meets it. "Usually fast enough" is not real-time.

## Done bar
A systems/kernel change is done when memory access is bounds- and
lifetime-accounted, `unsafe` is isolated with its invariant documented, interrupt
handlers are short and reentrancy-safe, shared state has the correct
synchronization with a stated lock order, allocation on hot/interrupt paths is
avoided or justified, stack usage is bounded, and any real-time deadline is shown
to be met on the worst-case path.
