# WebView2 window open time and memory

- Date: 2026-10-08
- Phase 0 item: WebView2 ([roadmap](../roadmap.md))
- Machine: the [reference machine](2026-10-06-reference-machine-baseline.md), Windows
  build 26300.9457, on AC, effective power mode "max performance", WebView2 runtime
  installed with Windows.
- Scope labels as in the [process snapshot note](2026-10-06-process-snapshot-cost.md).

## Question

How long does a WebView2 window take to show its first frame, cold and warm? What memory
does it hold while open and after it is closed? The budgets in the design document are
below 300 ms for a warm open and below 1.5 s for a cold open.

## Method

Throwaway spike outside the repository, built on `wry` 0.57 and `tao` 0.37. These are
the WebView and windowing layers of Tauri 2. WebView2's own start-up dominates the
cost, so it is the same either way.

- **Page:** a static page shaped like the planned overview, with no external
  resources. "First frame" is the moment the page reports, over IPC, that two
  animation frames have passed after load.
- **Timing:** from process creation (`GetProcessTimes`) and from the window-creation
  call, to that IPC message.
- **Memory:** private bytes of the probe and every process started below it (the
  WebView2 browser, GPU, renderer and utility processes), 1.5 seconds after the first
  frame.
- **Runs:**
  1. ten separate launches, the first with no WebView2 profile, so it is created;
  2. one process that opens, closes and reopens a window five times with no pause;
  3. one process that does the same three times, with 10 seconds between close and
     reopen.

## Results

| Scenario | First frame | Memory while open |
|---|---|---|
| First launch, profile created | 422 ms after process creation | 170.5 MiB in 7 processes |
| Later launches (9) | 326 to 355 ms after process creation, median 345 ms | 166 to 170 MiB in 7 processes |
| Reopen in the same process, no pause (4) | 112 to 117 ms after the window call | 181 to 184 MiB in 7 processes |
| Reopen in the same process, 10 s after close (2) | 297 to 301 ms after the window call | 168 to 170 MiB in 7 processes |

- **Launch overhead:** process start to `main` took 15 to 21 ms. Almost all of the time
  is WebView2 start-up.
- **Closing does not free memory at once:** right after closing, 158 to 162 MiB stayed
  resident in 6 processes. The WebView2 browser process outlives the last window for a
  short time.
- **After 10 seconds it is gone:** only the probe remained, at 4.6 to 4.9 MiB, and the
  next open paid the full start-up cost of about 300 ms again.

## Verdict

- **Cold open:** below 1.5 s is met, with a worst case of 422 ms.
- **Warm open:** below 300 ms is met when the WebView2 environment is kept alive: 112
  to 117 ms. When it has been freed, a reopen takes about 300 ms, at the limit.
- **Memory:** an open window costs about 170 MiB of private memory across WebView2's
  processes. Keeping it warm costs about 160 MiB while the window is closed.

This supports the window lifecycle in the design document and in
[ADR 0006](../adr/0006-tauri-and-svelte-for-the-desktop-ui.md): free the WebView on
close by default, and offer "keep warm" as an explicit trade of about 160 MiB for
openings that are about 185 ms faster.

## Limits

- **Static page.** The real Svelte UI adds JavaScript parse and start-up time, plus a
  first data fetch from the daemon, on top of these numbers.
- **No truly cold start.** The runtime's files were already in the file cache, because
  other applications on this machine use WebView2 continuously. A first launch after
  a reboot was not measured.

## Open

- Measure the real UI shell in Phase 1 against the same budgets.
- Measure a first launch after a reboot.
