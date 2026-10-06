# 0011. In-memory flight recorder

- Status: accepted
- Date: 2026-10-06
- Deciders: Kemal Aslıyüksek

## Context and problem

When the machine feels slow, opening any program is slow too, and by the time a window
appears the spike that caused the slowdown may already be over. A tool that only shows
"now" misses the moment the user cared about.

## Considered options

- Show only the current state.
- Persist everything to an on-disk time-series database from day one.
- Record continuously with full ETW tracing (Windows Performance Recorder style).
- Keep a bounded in-memory history of recent samples.

## Decision

The daemon keeps the **last 15 minutes in memory**, in fixed-capacity ring buffers
(`vagus_core::RingBuffer`):

- **System metrics:** every sample (900 samples at 1 Hz).
- **Processes:** the top N consumers per resource for every sample, plus totals. N is
  configurable; the default is decided in Phase 1.
- **No disk persistence in Phase 1.** Optional downsampled on-disk history is an open
  question for later phases.
- **Memory budget:** at most 10 MB for the recorder (target, verified in Phase 1).
- **Allocation reuse:** evicted samples are handed back by `RingBuffer::push` so their
  allocations can be reused, and the steady state allocates nothing.

## Consequences

- Positive: the UI can answer "what just happened", memory use is constant, and there
  is no disk I/O.
- Negative: history is lost when the daemon restarts or the machine reboots.
- Negative: storing only the top N loses small consumers. That is acceptable, because
  diagnosis looks for the large ones.

## Why the other options were rejected

- Current state only: misses spikes that ended before the window opened.
- On-disk time-series database: adds disk I/O, complexity, retention decisions and
  privacy questions before anyone has shown the need.
- Full ETW tracing: needs admin, produces large amounts of data, and is a debugging
  tool rather than an always-on monitor.
