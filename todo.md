# Bug Fixes & Improvements

---

## Round 2

### A — "Attempt N/M" in GUI during reconnect
Already works — `send_log("Info|Reconnecting... (attempt N/M)")` flows through `LogAppended`
into `status_message` automatically (app.rs line 698). No code change needed.

### B — "Max reconnect attempts reached" message
- [x] `session.rs`: log `Warn|All N reconnect attempts exhausted. Giving up.` before final break

### C — History silent corruption warning
- [x] `history.rs`: both `append_event` and `load_events` now `log::warn!` on JSON parse error

### D — Reconnect delay between attempts
- [x] `session.rs`: 3s interruptible delay (30 × 100ms with cancel-token check) before each retry

### E — Log escalation tool selection
- [x] `unix.rs`: `log::info!("Using escalation tool: {}", ...)` after tool is resolved

### F — Browser idle timeout not logged
- [x] `browser.rs`: `log::info!("Browser idle timeout: {}s ({})", ...)` on every launch

### G — Lock `.unwrap()` → `.expect("session mutex poisoned")`
- [x] `session.rs`: all `.lock().unwrap()` replaced via replace_all

### H — Tag Disconnected history event with reconnect attempt count
- [x] `session.rs`: `cleanup(reconnect_attempts)` now sets `event.message` when `attempts > 0`

### I — Show `prev:` label for Reconnected events in history
- [x] `view/history.rs`: Reconnected events show `· prev: Xm Ys` instead of `· Xm Ys`
- [x] `cli/main.rs`: same for CLI history output

### Session log preservation on auto-retry
- [x] `types.rs`: add `Message::AutoRetryConnect` variant
- [x] `app.rs`: `ConnectPressed` clears logs (user-initiated); `AutoRetryConnect` instead appends
  a separator banner `──── auto-retry: stale session cleared ────` and preserves history
- [x] `app.rs`: stale-session retry path changed from `ConnectPressed` → `AutoRetryConnect`
- [x] `app.rs`: `logs.clear()` moved out of `handle_connect_pressed()` into `ConnectPressed` handler

---

## Status Key
- [ ] Not started
- [x] Done

---

## Issue 3 — Windows error details not surfaced
**Files:** `openconnect/windows.rs`, `openconnect/mod.rs`, `session.rs`

Change `thread_failed: Arc<AtomicBool>` → `thread_failed_reason: Arc<Mutex<Option<String>>>` so the
actual failure reason (UAC denied, binary missing, etc.) is stored and shown to the user instead of a
generic "process exited" message.

- [x] `windows.rs`: store reason string in `thread_failed_reason`; update `vpn_process_alive`
- [x] `mod.rs`: rename enum field; add `failure_reason() -> Option<String>` method
- [x] `session.rs`: use `p.failure_reason()` when setting conn error

---

## Issue 4 — macOS utun detection false positives
**File:** `openconnect/unix.rs`

`is_vpn_interface_up_impl` on macOS only checked whether any non-CGNAT utun exists, not whether
openconnect was actually running. Gate the check on `is_openconnect_running()` to prevent other VPN
clients (Cisco AnyConnect, etc.) from causing false "connected" detections.

- [x] `unix.rs`: add `is_openconnect_running()` guard to `is_vpn_interface_up_impl` (macOS)

---

## Issue 6 — `connected_at` race condition on rapid reconnect
**File:** `session.rs`

**Investigated — not a real bug.** The code correctly overwrites `connected_at` in each new
`run_watchdog` call, and `prev_duration` is computed from the returned `Some(duration)` value before
reconnect. Brief outages missed between 1 s polls result in no reconnect (acceptable). No code change
needed.

---

## Issue 7 — Spinner `.unwrap()` in CLI
**File:** `kuvpn-cli/src/main.rs`

`.unwrap()` on a static template string will panic if the template is ever changed to be invalid.
Replace with `.expect("…")` to make the intent explicit and the panic message informative.

- [x] `main.rs`: `.unwrap()` → `.expect("spinner template is always valid")`

---

## Issue 8 — Log send failures silently ignored
**File:** `kuvpn/src/session.rs`

`let _ = tx.send(msg.into())` swallows the error completely. A dropped receiver indicates a logic
error (GUI/CLI disconnected the channel unexpectedly). Log a debug message so it's visible in
diagnostic runs.

- [x] `session.rs`: log debug message when log channel send fails

---

## Issue 9 — `tunnel_mode_val.round() as i32 == 2` fragility
**Files:** `kuvpn-gui/src/config.rs`, `app.rs`, `view/settings.rs`

The magic number `2` for Manual mode is scattered across three files. Adding an `is_manual_mode()`
helper to `GuiSettings` centralises the logic and makes call sites self-documenting.

- [x] `config.rs`: add `is_manual_mode() -> bool`
- [x] `app.rs`: replace raw comparison with `is_manual_mode()`
- [x] `view/settings.rs`: replace raw comparison with `is_manual_mode()`
