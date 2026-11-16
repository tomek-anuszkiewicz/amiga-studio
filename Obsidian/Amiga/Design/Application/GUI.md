---
title: "Amiga 500 GUI"
aliases: ["Frontend Architecture", "Developer Studio GUI", "Amiga 500 Developer GUI Specification"]
tags: ["amiga", "design", "gui", "egui", "eframe"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-05
updated: 2026-11-16
related: ["[egui Guidelines.md](../egui%20Guidelines.md)", "[Debugger.md](Debugger.md)", "[General Architecture.md](../General%20Architecture.md)", "[Keyboard.md](../Keyboard.md)", "[Mouse.md](../Mouse.md)", "[Audio.md](../Audio.md)", "[Denise.md](../Denise.md)", "[RTC.md](../RTC.md)"]
tracked_paths:
  - "crates/gui"
---
# Amiga 500 GUI

The GUI is the host-facing layer of the emulator. It presents the Amiga display and debugger state, receives host input, and is intended to play the machine's audio. The emulation core owns the machine's behavior; the GUI presents its outputs and translates user actions into machine input without making the core depend on a window or browser.

The same frontend targets a native desktop application and a browser build. It presents the same emulated machine in two views:

- **Game view** gives the framed Amiga screen the available space beside a compact player panel. File and State menus, Run/Stop, optional FPS display, speed ratio and Slower/Faster controls, theme, zoom and a Debug View switch remain available. Inspection panels and debugger shortcuts are disconnected.
- **Developer Studio** places the screen in a debugging workspace with execution controls and state inspection. The [debugger specification](Debugger.md) defines the behavior behind those controls.

Desktop startup opens the game library. Browser startup opens manual
setup, with no bundle probe or browser library. Global named configurations have
stable IDs and compose the supported hardware presets, ROM choice, connected
drives, both port devices and reusable host mappings. Games reference one
configuration and own ordered disks, descriptions, artwork, initial state and
player saves. Configuration editing has its own screen.

The desktop library provides Add game and Edit game forms for a name, optional
description, named configuration and individually selected ADF imports. Saving
copies imported disks into the game directory. The game name is also its folder
name; renaming preserves game/disk IDs, saves, initial states and other files.
Invalid names and existing destination folders are rejected without overwriting
another game. Cancel leaves files unchanged. A running session remains the same
machine when its library metadata changes.

Optional covers use the declared artwork path, or cover.jpg, cover.jpeg or
cover.png in the game directory when no path is declared. JPEG and PNG are
supported. The editor explains this convention; Refresh reloads cover changes.

Manual setup accepts a supported ROM and zero to four disks. Launch validates
all assets before replacing the active machine. Returning to setup or library
pauses and releases capture. A hardware/ROM replacement requires confirmation.
The player panel has four disk dropdowns with eject, connection/activity status,
pause/resume, reset, save/load, Library/Setup and Debug access. Narrow windows
collapse the same controls. Debug does not duplicate this panel. Screen scaling
continues to preserve the complete framebuffer aspect ratio.

The views share one machine. F2 or F12 switches between them by default; Escape releases capture so player controls remain reachable. Frontend bindings are configurable. Returning requests a pause at the field boundary and keeps instrumentation disconnected until that pause completes. Game execution bypasses breakpoints, disassembly, trace collection, CPU history and memory diff snapshots. Previously collected history remains available when returning, but no automatic snapshots or progress recording run in Game view. Manual State operations remain available. A stopped CPU still allows chip clocks, video and interrupts to advance in both views; a halted CPU ends Run.

Continuous Developer Run retains the last prepared inspector contents, including register and CPU status values, disassembly, memory, history and toolbar CCK/instruction labels. The Amiga image, FPS and execution controls remain live. Editing, browsing new inspection data and history navigation are inactive until paused. Repaints and resizing lay out retained data without rereading the machine or rebuilding diagnostic text. Stop pending retains the same freeze; a real stop refreshes current data. Explicit instruction, CCK and frame steps retain their diagnostic collection and show their resulting state. Reset, state loading and paused edits refresh after their coherent owner replies. Enabled breakpoints use current CPU state and stop before their matching instruction, even before a field completes.

## Application and debugger layout

The Developer Studio has a top bar and three main areas: a left dock for CPU state, a central workspace for the screen and code, and a right dock for memory and tools. The width of each side dock can be changed. On a wide window, the central workspace separates disassembly from the screen: the screen and history controls sit above the execution trace. On narrower windows, the screen, history controls, and disassembly stack vertically in the central area; the trace moves into the right dock's tools section. In either layout, the monitor image supplied by the frame builder is centered in the available display area. Both axes use the selected image scale, preserving the monitor image's width-to-height ratio instead of imposing 4:3.

| Area | Available information and controls |
| --- | --- |
| Top bar | Load a binary into memory or reset the machine; save and load state through files or quick slots; run, pause at a video field boundary, step one field, instruction or Color Clock, and step backward through recorded history. Measured video FPS, speed controls and CCK/instruction counters sit beside theme, zoom, and game-view controls. |
| Left dock | CPU data and address registers, program counter, status register and condition flags, followed by engine status such as bus availability and counters. An optional microcode inspector shows the current execution phase and intermediate CPU state; F8 toggles it. |
| Central workspace | The framed Amiga screen, a history bar for recording and browsing CPU snapshots by step, frame, slider, or CCK position, and a disassembly listing. The listing can follow the current PC or jump to an address, set breakpoints, change the PC, and edit an instruction in place. On wide windows, the trace below the screen lists recent execution; the split between code and screen and the split above the trace are draggable. |
| Right dock | A hex and ASCII memory viewer with address navigation and byte editing. Below it are memory search and controls for PC breakpoints, register conditions, and read/write watchpoint ranges. On narrower windows, the execution trace is also here. |

The execution trace and the optional snapshot history serve different purposes: the trace lists recent instructions, while the history bar browses recorded CPU states. History recording starts disabled, and browsing a snapshot does not rewind memory or the rest of the machine. Watchpoint stops remain incomplete; see the [debugger work](../../../../ROADMAP.md#debugger-execution-control). See [Debugger](Debugger.md) for these behavior limits and the [Debugger Guide](../../../../docs/debugger.md) for user-facing shortcuts.

Binary loading into the Kickstart address window accepts only a 256 KiB image. A rejected load leaves the machine unchanged and keeps the load dialog available for correction. File and snapshot loading errors remain visible until dismissed, including the expected and actual ROM byte counts for an invalid image.

File also offers **Load ROM from File** on native desktop and WASM. A valid 256 KiB image is installed as Kickstart, followed by a cold reset with the ROM overlay and reset vectors active; execution remains paused until Run. The toolbar displays the loaded filename. Invalid images preserve both machine state and the previous filename. Loading a state replaces this label with a snapshot source because the snapshot includes its own ROM. Browser files are selected explicitly by the user; no local directory scanning is required. Browser file selection and download are host operations, separate from synchronous machine execution.

File remembers separate last-used folders for binaries and ROMs and shows up to five recent files in each category, newest first. On desktop, clicking a recent entry reads the original file again; binary entries reuse their last successful target address and PC setting. Successful loads add or reorder entries. Selecting a file that no longer exists removes its recent entry and displays an explanatory message. Other load failures and canceled selections retain the entry. Desktop file pickers start in the nearest existing parent when a remembered folder is missing. Paths and load options persist alongside the GUI preferences, separately from machine snapshots.

In browsers with the File System Access API, each file category has its own picker directory. Clicking a recent entry opens the picker in that file's folder and requires explicit selection. A missing file or folder removes the entry from IndexedDB and triggers a browser message after the picker closes, including when selection is canceled. An unavailable starting folder falls back to the browser's default location. Names and file handles persist in IndexedDB; file contents are not cached. Other browsers retain file selection and recent names but cannot check whether those files still exist or be directed to a particular local folder. The browser controls directory availability and persistence; denied access alone does not establish that a file is missing. [Host file actions](../../../../crates/gui/src/file_actions.rs) keep these differences outside emulation.

## Input and output

Host physical key positions map to the [Amiga keyboard](../Keyboard.md), relative mouse movement and buttons reach the [mouse and game port](../Mouse.md), and a host gamepad supplies digital [joystick](../Joystick.md) directions and two fire buttons. Desktop input uses blocking rdev keyboard/mouse and gilrs controller listeners outside the machine owner. Browser DOM callbacks and standard-gamepad sampling remain on the main thread; the gamepad timer targets three samples per field independently of repaint. The basic adapter forwards host-reported transitions; it does not reconstruct missing host events or A500 matrix ghosting. Original keyboard combination fidelity remains [6.2 work](../../../../ROADMAP.md#task-6-2).

Clicking the Amiga image captures relative pointer input and gives it keyboard
focus. Capture release remains available. Focused host text controls take
priority, then configured frontend actions, keyboard joystick bindings, and
remaining guest keys. Consumed keys never also reach the guest keyboard. Both
hosts use this resolution policy. Each event carries its destination port.
A configuration selects mouse, joystick or disconnected independently per port,
then the source: one system pointer, a reusable keyboard mapping or a selected
individual gamepad. Two system mice are not offered. Controller connections are
explicit; unavailable or ambiguous identities require reassignment. Reconnection
must not replay stale held input. Configuration/source changes, disconnect,
focus/capture loss, pause, reset and load release affected controls. Ordered, bounded queues publish input immediately, and the owner admits it at three evenly spaced opportunities per emulated field, derived from current PAL/NTSC and interlace geometry. Initialization aligns the first grid with the receiver field origin; synchronization suppressed by guest code has a bounded nominal-period fallback. Guest beam writes that prolong a field retain nominal-period input opportunities. Every delivered edge gets a CCK sample before its successor; a short electrical pulse does not guarantee a guest software read. Keyboard bytes still traverse the controller queue and serial pins, and mouse quadrature continues at its existing CCK rate without smoothing.

Capture loss, pause and queue overflow discard pending input and release controls. Reset, successful load and keyboard reset invalidate the old input generation. Native backend timestamps reject events older than capture; fresh controller events re-establish its state after resume. Host queues and listeners are runtime handles outside serialized machine state. Physical controller freshness and Linux/macOS execution remain unverified; the interactive checks belong to [8.2](../../../../ROADMAP.md#task-8-2).

For video, [Denise](../Denise.md) builds a rendered frame in the [frame buffer](../Frame%20Buffer.md). Run at 1x follows the configured PAL or NTSC color clock using host elapsed time; emulation and rendering time count toward each deadline. Slower and Faster controls select 0.25x, 0.5x, 1x, 2x or 4x; a 1x control restores normal speed. At normal or slower speed, each publication selects one completed field. If a bounded slice ends before field completion, the viewport retains the previous image.

Above 1x, both views execute batches of complete fields and upload only the latest completed image. Presentation targets 50 Hz for PAL or 60 Hz for NTSC, independently of the requested emulation speed. Each batch ends when its emulated time budget is consumed or the host presentation deadline is reached, checked after each bounded field slice. Presentation deadlines accumulate, preserving fractional overruns; expired presentation slots are skipped without building an unbounded catch-up backlog. Host capacity can limit both achieved emulation speed and presentation rate. FPS measures all completed emulated video fields per host second, including fields omitted from presentation; it does not count texture uploads or GUI repaints. Here, Step Frame and FPS count fields, including each field of an interlaced image.

Ordinary pause finishes the current field, and Step Frame advances to the next completed field, including while the CPU is stopped. Instruction/CCK stepping, breakpoints and CPU halts can still expose a partial field. Snapshot restoration replaces the displayed pixels on the next UI frame. Host timing, texture allocation and upload belong to the frontend. For sound, the GUI is intended to deliver samples produced by the [audio subsystem](../Audio.md) to the host playback device. Emulated audio generation remains in the core.

The shared `live_stripes` guest program remains available for manual observation
without firmware. Host pacing, FPS and graphical guest output are evaluated
manually; their dedicated automated execution and presentation campaigns have
been removed. Independent correctness tests can still use guest programs as
fixtures. Input correctness and host command/lifecycle checks remain automated. Broader
display comparisons remain in [native disk verification](../../../../ROADMAP.md#native-disk-verification)
and its later subsystem suites. Host audio and advanced input mappings remain
incomplete; see [native test prerequisites](../../../../ROADMAP.md#native-disk-verification).

The [host runtime](../../../../crates/gui/src/host_runtime.rs) moves the complete
session to one native thread or browser Web Worker. This owner executes all CPU,
chip and peripheral work in the existing CCK order; GUI updates execute no guest
cycles. Commands and owned replies use bounded queues. GUI operations only try
shared locks and retain a rejected command for retry. The owner services commands
even when an unread image prevents further video publication. Reset and successful
loads advance a runtime generation. GUI image consumption waits for the reset/load
reply and validates the current generation before exposing pixels.

The [video exchange](../../../../crates/gui/src/host_video.rs) owns one raw image
allocation; the machine retains the other. Publication seeds and swaps them through
the [frame builder](../Frame%20Buffer.md), preserving partially drawn and opposite
interlaced rows. Atomic claims distinguish free, writing, ready and reading access.
A ready or reading image cannot be overwritten. Matching image metadata and owner
status become visible under the same host boundary. The GUI releases that status
lock before converting immutable pixels into its own `ColorImage`, then releases
the image slot before updating the GPU texture. It consumes at most one image per
UI frame, including when egui repeats layout. Paused state changes retain the
previous image until a matching CCK image arrives.

Image publication, command replies and shutdown wake the native GUI through
`request_repaint`; browser worker messages request the corresponding GUI repaint.

Fatal internal invariants use the shared
[`config::fatal_emulation_error`](../../../../crates/config/src/fatal.rs) helper.
On native hosts, one outer unwind boundary covers the complete owner loop,
including Run, commands, manual steps and replay. The boundary preserves the
first diagnostic, wakes the GUI and terminates the owner without publishing the
failed slice. Native builds require `panic=unwind` for this contract.

The browser retains `panic_abort`. Before the trap, the host panic hook forwards
its diagnostic to worker-local JavaScript storage. The outer worker handler
sends that text to the main thread and terminates the worker without Rust stack
or TLS teardown. Main-thread reporting does not acquire the failed owner's
locks. Both hosts retain the first diagnostic in the GUI, cancel queued commands
and reject further execution. Native closing does not wait for a Shutdown reply
from a failed owner. Browser failure leaves the error visible on the page.
Neither path resumes or saves the partially advanced machine as a valid snapshot.
Releasing an image slot wakes the producer. A paused GUI does not poll for frames
or repeatedly submit an unchanged UTC sample. Native workers park while waiting for events and use short sleeps between timed
execution deadlines. Browser workers return to their event loop between bounded slices;
message tasks continue immediately due work and timers handle future deadlines.
The eframe browser runner still schedules animation-frame callbacks while idle,
but it invokes GUI logic only when a repaint is due.

Developer inspection receives coherent, stopped CPU, physical memory, trace,
breakpoint and timeline data through the command boundary. The timeline transfers
PCs and CCK positions; historical CPU snapshots remain with the owner. Formatting
and visible-row preparation remain on the GUI thread. A stopped snapshot is
captured before Run; continuous Run retains prepared inspectors. Game does not
request inspector snapshots. Register edits, memory patches, breakpoints, history,
load/save and quick slots execute as owner commands. A binary load during Run
retains the prepared inspectors until stopping and waits for a completed field
before publishing if the current field is unfinished. File selection, host file I/O
and downloads remain host UI operations; filenames and recent-file records change
only after successful owner replies.

The browser uses one explicitly initialized main Wasm instance and one worker
instance sharing its memory. [The launcher](../../../../tools/web.ps1) rebuilds the
Rust standard library with atomics and exports thread-local initialization data.
Trunk provides the required isolation headers and disables its automatic second
initializer. Worker shutdown drops the owner, releases its stack and thread-local
storage, then acknowledges completion. Native close waits asynchronously for the
owner's final battery data before allowing normal application persistence.
Early wake messages remain represented in shared queues until the worker finishes
its single initialization. The [browser worker regression](../../../../crates/gui/tests/browser_worker_runtime.py)
checks ordinary and delayed startup, shared memory and idle notifications against
the served application.

The frontend refreshes UTC seconds for the [RTC](../RTC.md) on the first UI
pass when its latest coherent sample differs, including paused inspection and
resuming execution. It retains one outstanding clock command at a time. Native uses the system
UTC clock; the browser uses `Date.now()`. Both round down to whole Unix seconds,
including fractional timestamps before the epoch. The RTC battery offset is loaded and
saved under the independent `rtc_battery` storage key through the normal
application persistence lifecycle. Machine snapshots do not contain this data.

## Scaling and appearance

The interface uses logical sizes so controls and text remain readable at different desktop DPI settings and browser zoom levels. Interface zoom is separate from the Amiga image scale.

Both views offer one **Image scale** selector: **Fit (smooth)**, **0.5x**, **1x**, **2x**, and **4x**. Fit is the default; it uniformly fits the complete image into the display area with bilinear filtering. Fixed scales use nearest-neighbor filtering and count physical host pixels per framebuffer sample, independently of interface zoom and host DPI. Their origin aligns to the host pixel grid. An image larger than its display area stays at the selected scale, centered and clipped to that area; it does not shrink automatically or add scrollbars. The selected scale persists with the other GUI preferences. There is no separate filter control.

Changing image scale takes effect while paused or running. It changes presentation only: the machine state and framebuffer samples remain unchanged, and a filter change during an unfinished field retains the previously presented image.

The GUI offers dark and light themes, plus a Workbench-inspired theme. A theme changes the host interface's appearance without changing emulation state. The same view and scaling principles apply in the native application and the browser.

## References

- [General Architecture](../General%20Architecture.md): Machine ownership and the boundary between emulation and host presentation.
- [Debugger](Debugger.md): Execution and inspection behavior behind the debugger view.
- [egui Guidelines](../egui%20Guidelines.md): Frontend implementation and interaction guidance.
- [Host input](../../../../crates/gui/src/host_input.rs): Ordered admission and peripheral delivery.
- [Host key mapping](../../../../crates/gui/src/host_keys.rs): Physical positions to Amiga codes.
- [GUI source](../../../../crates/gui/src/gui.rs): Native and browser entry points, view state, and layout modules.
