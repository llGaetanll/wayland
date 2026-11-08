# Notifications

A notification daemon in the bar: eww draws the toasts, `module-notify` owns the
rules, and zbus lives entirely in the IO layer.

**Status: not started.** No code exists yet; every stage below is open. The
unrelated uncommitted work in `crates/module-battery` and `crates/module-spaces`
predates this plan and has nothing to do with it.

Nothing on this system currently owns `org.freedesktop.Notifications`, so every
`notify-send` fails silently today. There is nothing to migrate and no existing
daemon to displace.

## The shape

zbus is IO like any other probe. Inbound D-Bus traffic becomes events; outbound
signals are actions the runtime carries out. The machine stays synchronous and
never learns that D-Bus exists.

```
Notify / CloseNotification  ->  event  ->  machine  ->  action  ->  NotificationClosed / ActionInvoked
```

This is the `hypr.rs` arrangement — a thread owning a connection, pushing `Msg`
into the runtime channel, reconnecting on its own — with one difference that
shapes the whole design, below.

## Decisions already taken

**The edge allocates notification ids.** `Notify` is a method call: the caller
blocks until it gets a `u32` back. If that id came from the machine, the IO
thread would have to send an event and then block waiting for the runtime loop
to come round, which puts loop latency in the path of every app that notifies
and wedges all of them when the loop is busy. So the D-Bus layer holds the
counter, replies immediately, and forwards the event with the id already in it —
the machine receives it the way it receives `now`, a value chosen at the edge.
Replay stays honest because the id is on the event.

`replaces_id` does not break this: per spec a non-zero value means reuse that
id, so the edge allocates only when it is `0`. What replacement *means* — which
entry is overwritten, whether the timer restarts — stays in the machine.

**`GetCapabilities` and `GetServerInformation` never reach the machine.** They
are constants. The IO layer answers them alone.

**The close reason travels on the action.** The spec wants a reason code (1
expired, 2 dismissed by user, 3 via `CloseNotification`, 4 undefined). The
machine is the only thing that knows which happened, so it goes out on
`EmitClosed` rather than being guessed at the edge.

**zbus blocking, not tokio.** The blocking API keeps the executor inside the IO
thread and leaves the workspace without an async runtime. zbus defaults to
`async-io`; do not enable the tokio feature.

**No D-Bus activation `.service` file.** Activation would start a second
eww-state; the daemon is already launched at login from `hyprland.lua`. The
consequence is that notifications exist only while the bar runs, which is
always. Name acquisition still has to handle "already owned" and name-lost as
events, with the reconnect loop `hypr.rs` already models.

**No `JobId` anywhere in this module.** Nothing the machine asks for awaits a
reply — every inbound event is unsolicited. `api-core/jobs.rs` exists for the
wifi and bluetooth case where a module has to recognise the answer to a question
it asked, and that case does not arise here. Do not reach for it.

**Expiry needs no new mechanism.** `Event::Clock(EventClock::Tick { now })`
already arrives every second from the clock thread in `daemon/src/ticker.rs`,
and `module-wifi/src/state.rs` already does the deadline arithmetic a toast
wants — store a `Millis` when the notification lands, compare with
`self.now.since(that) >= timeout`. Copy that, do not invent a timer.

**One window, not one per toast.** `defwindow` is static, so the stack is a
single window holding a vertical box over a JSON var — the `dock_apps` and
`search_hits` arrangement.

## Traps

**Notification text is attacker-controlled.** Any app on the session bus can put
arbitrary bytes in `summary` and `body`. Two consequences: text reaches eww only
as a JSON var, never interpolated into a yuck literal or a handler; and every
`onclick` passes an id, never any part of the content. `ipc.rs` already base64
encodes names that come from the network for exactly this reason — the same rule
applies here, and harder, because the source is local and unprivileged rather
than merely untrusted.

**`panic = "abort"` is set in the release profile.** A panic inside a zbus
interface method takes the bar down with it. The no-unwrap rule applies with no
slack at all on the `Notify` argument path, since every field there came from
another process.

**The clock ticks once a second.** `CLOCK_INTERVAL` is 1s and only
`Event::Clock` carries `now` — per-module ticks do not. A typical
`expire_timeout` is ~5s, so a toast can outlive its deadline by up to a second.
Accepted for expiry. Anything wanting a fade or a countdown ring needs the clock
interval to become adaptive first, which is out of scope here.

## Stages

### 1. The machine, with no bus at all

`crates/module-notify`: `event.rs`, `action.rs`, `state.rs`, `model.rs`,
`wire_json.rs`. No zbus dependency in this stage — the rules are the bulk of the
work and none of them need a connection.

Events: a notification posted (id already assigned), a close requested by id, a
dismissal from a click, an action activated, and the clock. Actions: emit
closed, emit action-invoked, stage an image. Vars: the JSON stack and its
height.

**Proves itself** with `drive()` tests over the rules that have no IO in them:
a notification expires at its deadline and not before; `replaces_id` overwrites
in place; urgency `Critical` never expires; the stack caps and the oldest goes;
each of the four close reasons comes out on the right event. `wire_json.rs` gets
`serializes!` cases per STYLE.md.

### 2. On screen, still with no bus

Wire the module into the daemon and eww, and drive it from `ipc.rs` by hand.

- `ModuleId::Notify` — `ALL` goes from 9 to 10.
- `Window::Notifications` in `window.rs`, added to `all()`.
- Registration in the daemon crate; route `Event::Clock` down to it.
- `defwindow notifications` in `eww.yuck` plus the stack widget, styled in
  `eww.scss`. Reuse `shot-menu`'s anchoring — `top right`, overlay,
  `:focusable false` — and let the bar's exclusive zone supply the top offset.
- A `layer_rule` in `hyprland.lua` for blur and a slide animation, against a new
  `eww-notifications` namespace. Both files carry KEEP IN SYNC comments; add
  this namespace to them.
- `eww-state notif post <b64 summary> <b64 body> [...]` in `ipc.rs` as the
  stand-in for a real client.

**Proves itself** by hand: `eww-state notif post` draws a toast that expires on
time, a click dismisses it, three of them stack and stay legible. The entire UI
is finished before D-Bus is written, and `notif post` stays afterwards as the
way to reproduce a rendering bug without an app.

### 3. The bus, inbound

`daemon/src/dbus_notify.rs`, alongside `hypr.rs` because it is the same kind of
thing. Owns the bus name, holds the id counter, answers the two constant methods
itself, forwards `Notify` and `CloseNotification` as `Msg`.

**Proves itself** when `notify-send hello` draws a toast; when `notify-send`
returns promptly rather than waiting on the runtime loop (time it, and time it
again with the loop deliberately busy); when `busctl --user call` on
`GetServerInformation` answers without the machine ticking at all; and when
killing and restarting the daemon reacquires the name.

### 4. The bus, outbound

`NotificationClosed` and `ActionInvoked` as signals, which means
`runtime_perform.rs` needs a handle into the zbus thread. Action buttons in the
toast route through `ipc.rs` as `eww-state notif action <id> <key>`.

**Proves itself** against a scratch client that registers actions: clicking the
button delivers `ActionInvoked` with the right key; dismissing delivers
`NotificationClosed` reason 2; letting it lapse delivers reason 1; the client's
own `CloseNotification` delivers reason 3.

### 5. Icons and images

The `image-data` hint arrives as `(width, height, rowstride, has_alpha,
bits_per_sample, channels, bytes)` and GTK needs a file, so the IO layer encodes
a PNG into a new staging path on `Paths` and reaps it the way `module-shot`
reaps captures. `app_icon` is a themed name — `module-search` already resolves
those against `icon_dirs`; reuse it rather than writing a second lookup.

**Proves itself** with `notify-send -i` against a themed name, an absolute path,
and a missing name (which must fall back rather than leave a hole); and with a
client sending inline pixels both with and without alpha.

### 6. Do-not-disturb and history — deferred

A DND toggle and a notification centre are a bar dropdown like wifi and
bluetooth, and nothing in stages 1-5 should be shaped around them beyond
keeping dismissed notifications in state long enough to be listed. Worth
splitting into its own plan when it comes up.
