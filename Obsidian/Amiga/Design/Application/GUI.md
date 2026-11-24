---
title: "Amiga 500 GUI"
aliases: ["Frontend Architecture", "Developer Studio GUI", "Amiga 500 Developer GUI Specification"]
tags: ["amiga", "design", "gui", "egui", "eframe"]
category: "Design"
subsystem: "gui"
status: "active"
created: 2026-09-05
updated: 2026-11-24
related: ["[egui Guidelines.md](../egui%20Guidelines.md)", "[Debugger.md](Debugger.md)", "[General Architecture.md](../General%20Architecture.md)", "[Keyboard.md](../Keyboard.md)", "[Mouse.md](../Mouse.md)", "[Audio.md](../Audio.md)", "[Denise.md](../Denise.md)", "[RTC.md](../RTC.md)"]
tracked_paths:
  - "crates/gui"
---
# Amiga 500 GUI

The GUI presents the Amiga display and debugger state and translates actions
on the host into machine input. Playback of the machine's audio is planned.
The emulation core owns machine behavior and runs independently of the window
or browser.

The native desktop application and the browser build use the same frontend.
Both offer two views of the same emulated machine.

- **Game view** uses the available space for the framed Amiga screen beside a
  compact player panel. It offers the File and State menus, Run/Stop, optional
  FPS display, speed ratio and Slower/Faster controls, theme, zoom and the Debug
  View switch. Inspection panels and debugger shortcuts are inactive.
- **Developer Studio** places the screen in a debugging workspace with execution controls and state inspection. The [debugger specification](Debugger.md) defines the behavior behind those controls.

The desktop opens the game library at startup. The browser opens manual setup
without probing for a bundle or offering a game library. Each globally available
named configuration has a stable ID and combines the supported hardware
presets, the ROM choice, connected drives, both port devices and reusable host
mappings. A game references one configuration and owns its ordered disks,
description, artwork, initial state and player saves.

The configuration editor opens on its own screen and keeps a separate draft
of the complete catalog, including profile creation, selection and reusable
mappings. Switching profiles in that screen keeps all draft edits. When the
user selects Save, the editor first validates the edited scope. With execution
stopped, the machine owner applies the active session's controls. The editor
then publishes the committed settings. If Save fails, the draft stays
available and the committed catalog keeps its previous settings. Cancel
discards the draft, including invalid edits, and returns to the screen that
opened the editor. Leaving with unsaved edits offers Save, Discard and Keep
editing. With no unsaved edits, navigation proceeds directly. Shutdown persists
only the committed catalog.

Saving controls updates the active session's named configuration without a
restart. Selecting another profile selects it for a future launch only. The
machine remains paused until resumed. RAM, ROM, video and drive choices take
effect on explicit launch only.

When port devices change, the machine installs neutral devices only at the
changed endpoints. Unchanged mice keep their quadrature phase. Before execution
resumes, old input is released and invalidated while chip-side JOY/POT and CIA
state are preserved. Every user load uses the same policy for releasing controls
and attaching devices, including Developer Studio file loads and quick slots.
Capturing a save observes the machine without releasing controls or changing the
input generation.

The desktop library provides Add game and Edit game forms. The user enters a
name, an optional description and a named configuration, and selects ADF files
to import individually. **Startup disks** lets the user choose a game disk,
pending import or Empty for each drive from DF0 through DF3. All four choices
start empty for a new game. Importing disks leaves those choices as set by the
user. Saving copies the imported disks into the game directory, stores paths
relative to that directory and records the chosen disk IDs.
Assignments require drives connected in the chosen configuration. The game's
name is also its folder name. Renaming preserves game and disk IDs, saves,
initial states and other files. The library rejects invalid names and existing
destination folders without overwriting another game. Cancel leaves the files
as they were. Editing a running game's startup assignments leaves its current
media in place.

For optional covers, the library uses the declared artwork path. Without a
declared path, it looks for cover.jpg, cover.jpeg or cover.png in the game
directory. It supports JPEG and PNG. The editor explains the file convention,
and Refresh reloads changes to covers.

In manual setup, the user can select a supported ROM and zero to four disks.
Launch validates every asset before replacing the active machine. Returning to
setup or the library pauses the machine and releases capture. A hardware/ROM
replacement requires confirmation. The player panel provides four disk
dropdowns with eject, connection/activity status, pause/resume, reset,
save/load, Library/Setup and Debug access. Narrow windows collapse the same
controls. The panel is present once, in the player view. Screen scaling
preserves the aspect ratio of the complete framebuffer.

**Reset** in the player panel cold-resets the current machine and starts
execution, including when the player paused before clicking it. For a library
game, the owner first restores the active game's startup assignments, including
empty drives. It prepares the complete set before changing any drive. The
current hardware configuration stays in effect and each assigned drive must
already be connected. A failure preserves the current media and execution
state and reports the cause. A manual session keeps its inserted media.

In the library, **Start game** loads a configured initial state and boots
normally when no initial state is configured. If loading the initial state
fails, the library reports the error and offers **Boot normally without initial
state**. Normal boot uses the game's saved startup assignments. If all four
assignments are empty, normal boot and player Reset leave all drives empty.
Loading an initial state or progress restores the inserted disks and empty
drives recorded in that snapshot. Player Reset boots from the current ROM with
the prepared media. Debugger cold and warm resets, guest RESET and keyboard
reset retain their existing hardware behavior.

When a guest writes to a player disk, the new contents follow that disk through
ejection and reinsertion during the session. The original ADF stays unchanged.
The player uses the disk ID when selecting a detached copy. It prefers the copy
last removed from the requested drive. A single detached copy can move to
another connected drive. If several copies are available from other drives,
the player reports the ambiguity and leaves all drives as they were. Copies
mounted simultaneously are independent. On player Reset, the owner first looks
for matching mounted or detached copies. Copies already in their requested drive
or last removed from it take priority. A remaining copy can move when its source
and destination are unambiguous. When no matching copy is available, the owner
prepares the disk from the source ADF bytes already loaded for the session.
The [save state specification](SaveState.md#player-progress)
defines what a quick load or saved file restores.

In a native library session, click **Screenshot** beside **Save** and **Load**
in the player panel to save the Amiga image displayed at the time of the click.
The player saves the whole image at its source dimensions and colors, including
edges clipped by the viewport. Each click creates a new PNG in `Screenshots/`
under the active game's directory, even after browsing another game. The player
publishes the PNG only after the whole file is written. A successful capture
displays no notification. If capture fails, the player reports the error, the
game remains usable, and existing PNGs stay intact. Manual and browser sessions
do not offer this action. Screenshot has no keyboard shortcut.

Each connected drive has a fixed-position activity light beside its DF label.
Effective CIA-B selection or motor changes, active STEP edges, disk DMA reads,
consumption of fresh DSKBYTR bytes and serialized write words extend the
120 ms event indication. A connected and selected drive stays active while its
latched motor is enabled, even without new events. Deselected rotation, raw
magnetic transitions and debugger peeks alone leave the light off once the
hold expires. Ending sustained activity starts the same 120 ms hold. Control
activity can occur without inserted media. Each transfer is attributed to
every drive that is selected, running, connected and contains media at the
event boundary. This preserves the captured connector phase.

The frontend defines the light as player feedback with its own rules,
independent of the physical drive LED. Wrapping source sequences preserve
short events between reports from the machine owner and GUI updates. The
owner publishes observations with completed fields and explicit steps or
commands. Loading starts a new event baseline and cancels the previous hold.
A restored drive that is selected and running may immediately show sustained
activity. Reset stops the motors and cancels the indication. Write activity
shows that the controller produced output. Establishing whether the disk
image changed or was saved requires separate evidence.
[Floppy](../Floppy.md#autonomous-adf-reads-and-continuation)

The views share one machine. Use F12 or the toolbar to switch views.
Switching from a running Developer Studio session to Game keeps the machine
running in the same input generation.
The middle mouse button releases the mouse without pausing. The guest never
receives that button. The optional binding for mouse release starts unassigned,
and Escape and F2 otherwise remain guest keys.
Pause/resume and quick save/load are configurable and start at F5, F6 and F9.
Reset has no keyboard shortcut. Debugger stepping shortcuts are fixed and work
only in Developer Studio.

Returning to Developer Studio requests a pause at the field boundary. The
debugger stays disconnected until that pause completes. In Game view, execution
bypasses breakpoints, disassembly, trace collection, CPU history and memory
diff snapshots. On return, previously collected history is still available.
Game view performs no automatic snapshots or progress recording, and manual
State operations remain available. In both views, chip clocks, video and
interrupts keep advancing when the CPU is stopped. A CPU halt ends Run.

During continuous Run in Developer Studio, inspectors keep the last prepared
register and CPU status values, disassembly, memory, history and toolbar
CCK/instruction labels. The Amiga image, FPS and execution controls remain
live. Editing, browsing new inspection data and navigating history are inactive
until the machine pauses. Repaints and resizing lay out the retained data
without rereading the machine or rebuilding diagnostic text. If Stop is
pending, the inspectors keep showing the prepared data. Once execution stops,
they refresh from current state.

Explicit instruction, CCK and frame steps still collect diagnostics and show
the resulting state. Reset, state loading and paused edits refresh the view
when the owner replies with coherent data. Enabled breakpoints check the
current CPU state and stop before the matching instruction, even before a
field completes.

## Application and debugger layout

Developer Studio places the top bar above a left dock for CPU state, a central
workspace for the screen and code, and a right dock for memory and tools. Both
side docks can be resized. In a wide window, the central workspace displays
the disassembly separately from the screen, with the screen and history
controls above the execution trace. In a narrower window, the screen, history
controls and disassembly stack vertically in the central workspace, and the
trace moves to the right dock's tools section. Both layouts center the monitor
image from the frame builder and apply the selected scale equally to both axes.
This preserves the image's own width-to-height ratio without forcing 4:3.

| Area | Available information and controls |
| --- | --- |
| Top bar | Load a binary into memory or reset the machine; save and load state through files or quick slots; run, pause at a video field boundary, step one field, instruction or Color Clock, and step backward through recorded history. Measured video FPS, speed controls and CCK/instruction counters sit beside theme, zoom, and game-view controls. |
| Left dock | CPU data and address registers, program counter, status register and condition flags, followed by engine status such as bus availability and counters. An optional microcode inspector shows the current execution phase and intermediate CPU state; F8 toggles it. |
| Central workspace | The framed Amiga screen, a history bar for recording and browsing CPU snapshots by step, frame, slider, or CCK position, and a disassembly listing. The listing can follow the current PC or jump to an address, set breakpoints, change the PC, and edit an instruction in place. On wide windows, the trace below the screen lists recent execution; the split between code and screen and the split above the trace are draggable. |
| Right dock | A hex and ASCII memory viewer with address navigation and byte editing. Below it are memory search and controls for PC breakpoints, register conditions, and read/write watchpoint ranges. On narrower windows, the execution trace is also here. |

The execution trace lists recent instructions. The history bar lets the
developer browse CPU states from optional snapshot recording. Recording starts
disabled, and browsing a snapshot leaves memory and the rest of the machine at
their current state. Stops triggered by watchpoints remain incomplete.
[debugger work](../../../../ROADMAP.md#debugger-execution-control)

The debugger specification describes these behavior limits, and the guide
lists the shortcuts. [Debugger](Debugger.md), [Debugger Guide](../../../../docs/debugger.md)

Loading a binary into the Kickstart address window requires a 256 KiB image.
If the load is rejected, the machine stays as it was and the dialog stays open
for correction. File and snapshot load errors remain visible until dismissed.
For an invalid ROM image, the error shows the expected and actual byte counts.

File offers **Load ROM from File** on native desktop and WASM. With a valid
256 KiB image, the emulator installs it as Kickstart and performs a cold reset
with the ROM overlay and reset vectors active. Execution stays paused until
Run. The toolbar displays the loaded filename. A rejected image preserves the
machine state and previous filename. Loading a state replaces this label with
the snapshot source because the snapshot includes its own ROM. In the browser,
the user explicitly selects files without any required scan of local
directories. Selection and download are host operations outside synchronous
machine execution.

File remembers the last folder used for each of binaries and ROMs and shows
up to five recent files per category, newest first. On desktop, clicking a
recent entry reads the original file again. A binary entry reuses its last
successful target address and PC setting. A successful load adds or reorders
the entry. If the selected file has disappeared, the GUI removes its entry
and explains why. Other load failures and canceled selections keep the entry.
If a remembered folder is missing, the desktop picker opens in the nearest
existing parent folder. Paths and load options persist with the GUI
preferences, outside machine snapshots.

In browsers with the File System Access API, each file category has its own
picker directory. Clicking a recent entry opens the picker in that file's
folder and requires the user to select a file explicitly. If the file or folder
is missing, the frontend removes the entry from IndexedDB and shows a browser
message after the picker closes, even if selection is canceled. If a starting
folder is unavailable, the picker uses the browser's default location.
IndexedDB retains names and file handles only, without caching file contents.

Other browsers still offer file selection and retain recent names. They
cannot check whether those files still exist or direct the picker to a specific
local folder. The browser controls directory availability and persistence.
Access denial alone leaves the file's existence unknown. These file actions
take place outside emulation. [Host file actions](../../../../crates/gui/src/file_actions.rs)

## Edit input bindings

Open **Configurations** to choose the keyboard, gamepad and emulator
shortcuts for a configuration. Each binding button shows its current host
input. Click it and press a supported key or move the selected controller
input. The editor records one binding at a time. Use **Cancel recording** to
keep the current binding. Focus loss or a selected controller disconnect also
ends recording. Recording consumes the press and release, ignores repeated
events and waits for a held controller input to return to neutral before
recording it. A rejected input explains what to change so the player can
try another. **Save** validates and publishes the configuration draft.

**Emulator shortcuts** lists F12 for switching views and middle mouse for
releasing the mouse. A new configuration has no additional mouse release
key. F12 is reserved. The editor rejects Escape as a new mouse release key
and explains this choice when a saved configuration still names it. The
stored binding remains until the player reassigns or unassigns it.
**Controller shortcuts** shows controller, button and action names. Without
an available controller, the section explains why editing is disabled and
retains the saved assignments. When a controller is available, the player
can select it in place of an unavailable saved connection. Quick save and
Quick load use slot 1 for both controller and keyboard shortcuts.

**Amiga keyboard remapping** sends an alternate host key to a named Amiga key
in one direction. For example, select Amiga Escape and record a spare key.
A source already assigned to a joystick or emulator shortcut must be freed
first. Multiple host keys can send the same guest key. The guest receives one
press and a release when the last source is released. Unused host positions
follow the ordinary translation.

**Reusable joystick mappings** offers the WASD preset with J/K for Fire 1/2
and the Arrow keys preset with Z/X for Fire 1/2. The editor names the six
digital contacts and records the horizontal and vertical controller axes
with the existing activation threshold. Existing customized mappings and
configuration selections persist. To delete a mapping, first reassign or
remove all uses. The editor lists each named configuration and retained
active session that still uses it. Deleting or creating a mapping in a draft
takes effect on Save.

Button descriptions show the applicable fixed combinations and the current
committed keyboard shortcuts. They reflect the current Pause or Resume
action and retain the description and any disabled reason.

## Input and output

Host physical key positions map to Amiga keys. Relative mouse movement and
buttons reach the mouse and game port. A host gamepad supplies digital
joystick directions and two fire buttons. [Amiga keyboard](../Keyboard.md), [mouse and game port](../Mouse.md), [joystick](../Joystick.md)

On desktop, the frontend receives keyboard events through eframe's public
external-event-loop API before egui simplifies the key identities. Pointer
input also uses application window events. gilrs supplies controller events
outside the machine owner. Desktop window keys and browser key events use the
same routing policy to choose a frontend command or guest input. There is no
global keyboard or mouse listener.

Browser DOM callbacks and standard-gamepad sampling run on the main thread.
The gamepad timer targets three samples per field independently of repaint.
The basic adapter forwards transitions reported by the host. Reconstructing
missing host events or A500 matrix ghosting is outside its scope. Fidelity for
original keyboard combinations remains pending. [6.2 work](../../../../ROADMAP.md#task-6-2)

Game and Developer Studio sessions accept physical gameplay keys while the
machine is running in an active window. On the web, the document must have
focus and be visible. Clicking the Amiga image captures relative mouse
movement and buttons. The keyboard works before capture and after mouse
release. The session shows the mouse capture state and how to release it.
The browser can also release pointer lock with Escape.

Switching views releases held Amiga keys, mouse buttons, keyboard joystick
contacts and selected gamepad contacts. Keys held through the switch must be
released and freshly pressed. A held gamepad must return to neutral before
a new direction or button press reaches the guest. Selecting the current
view preserves held input.

Host text editors and dialogs own keyboard input while in use. Otherwise the
frontend gives application commands priority, followed by keyboard joystick
bindings and the remaining guest keys. Accepted gameplay keys bypass widget
navigation and activation. Once a press triggers an application command, the
frontend consumes its repeats and release even if an editor or dialog gains
focus in the meantime. The guest keyboard receives keys that earlier handlers
did not consume. Frontend bindings resolve host positions
independently of guest keycodes, including keys with no Amiga counterpart.
Each event carries its destination port.

A configuration selects mouse, joystick or disconnected independently for
each port. The source is one system pointer, a reusable keyboard mapping or
a selected individual gamepad. The frontend supports only one system mouse.
The user explicitly assigns controllers and must reassign them if the identity
is unavailable or ambiguous. Reconnecting a controller must avoid replaying
stale held input. Changes to the configuration or source, disconnect, focus
loss, pause, reset and load release the affected controls.

Gamepads can drive an active, running session with the mouse free or
captured. Configuration screens, controller assignment, host editors and
dialogs suspend gameplay. Releasing the mouse clears its queued movement and
buttons while preserving quadrature phase, held keys, keyboard joystick
contacts and selected gamepad contacts.

On native hosts, Load opens the Saved progress dialog for the active library
game. Opening it releases mouse capture and gives the dialog control of input
while the machine keeps its current run or pause state. Close returns to the
session. Choosing a save opens the replacement confirmation.

Ordered, bounded queues publish input immediately. The owner admits it at
three evenly spaced times per emulated field, determined by the current
PAL/NTSC and interlace geometry. Initialization aligns the first input grid
with the receiver's field origin. If guest code suppresses synchronization,
the owner uses a bounded nominal-period fallback. If guest beam writes prolong
a field, nominal-period input opportunities continue. Every delivered edge gets
a CCK sample before its successor. Guest software may miss a short electrical
pulse. Keyboard bytes still pass through the controller queue and serial pins.
Mouse quadrature continues at the existing CCK rate without smoothing.

Keys received while host mapping or keyboard activation is pending stay
queued until the owner can accept them. Focus loss, editor or dialog
ownership, pause and queue overflow discard pending gameplay input and
release controls. Gameplay resumes on fresh physical presses after a key has
been released. Reset, successful load and keyboard reset
invalidate the old input generation. Window keyboard buffers and queued input
carry their session generation. Entries from stale generations are discarded.

On native hosts, timestamps let the controller input handler reject samples
older than controller activation. Fresh controller events re-establish state
after resume. Host queues and listeners are runtime handles outside serialized
machine state. Input freshness with physical controllers and execution on
Linux/macOS remain unverified. The interactive checks belong to [8.2](../../../../ROADMAP.md#task-8-2).

Denise builds the rendered frame in the frame buffer. [Denise](../Denise.md), [frame buffer](../Frame%20Buffer.md)

At 1x, Run follows the configured PAL or NTSC color clock using elapsed host
time. Time spent emulating and rendering counts toward each deadline. Slower
and Faster select 0.25x, 0.5x, 1x, 2x or 4x, and the 1x control restores normal
speed. At normal or slower speed, each publication selects one completed field.
If a bounded slice ends before the field completes, the viewport keeps the
previous image.

Above 1x, both views execute batches of complete fields and upload only the
latest completed image. Presentation targets 50 Hz for PAL or 60 Hz for NTSC
independently of the requested emulation speed. After each bounded field slice,
the owner checks whether the emulated time budget is used up or the host has
reached its presentation deadline. Either condition ends the batch. Presentation
deadlines accumulate and preserve fractional overruns. The frontend skips
expired presentation slots to avoid an unbounded backlog. Host capacity can
limit both achieved emulation speed and presentation rate.

FPS counts completed emulated video fields per host second, including fields
that were never presented. Texture uploads and GUI repaints do not enter that
measurement. Step Frame and FPS both count individual fields, including each
field of an interlaced image.

An ordinary pause finishes the current field. Step Frame advances to the next
completed field, including while the CPU is stopped. Instruction/CCK stepping,
breakpoints and CPU halts can expose partial fields. After snapshot restoration,
the displayed pixels are replaced on the next UI frame. The frontend owns host
timing, texture allocation and upload.

The emulation core generates audio samples. The GUI is intended to deliver
those samples to the host playback device. [audio subsystem](../Audio.md)

The shared `live_stripes` guest program is available for manual observation
without firmware. Host pacing, FPS and graphical guest output are evaluated
manually after removal of their dedicated automated execution and presentation
campaigns. Independent correctness tests can still use guest programs as
fixtures. Input correctness and host command/lifecycle checks remain automated.
Broader display comparisons remain part of native disk verification and its
later subsystem suites. [native disk verification](../../../../ROADMAP.md#native-disk-verification)

Host audio and advanced input mappings remain incomplete. [native test prerequisites](../../../../ROADMAP.md#native-disk-verification)

The host runtime moves the complete session to one native thread or browser
Web Worker. This owner executes all CPU, chip and peripheral work in the
existing CCK order. GUI updates run outside guest execution. Commands and
owned replies use bounded queues. GUI operations try to acquire shared locks
without waiting and retain a rejected command for retry. The owner still
services commands when an unread image prevents further video publication.
Reset and successful loads advance a runtime generation. Before showing
pixels, the GUI waits for the reset/load reply and validates the current
generation. [host runtime](../../../../crates/gui/src/host_runtime.rs)

The video exchange owns one raw image allocation, and the machine retains the
other. To publish, the frame builder seeds and swaps the two images, preserving
partially drawn rows and rows of the opposite interlaced field. Atomic claims
distinguish free, writing, ready and reading access. An image in the ready or
reading state is protected from overwrite. Matching image metadata and owner
status become visible at the same host boundary. [video exchange](../../../../crates/gui/src/host_video.rs), [frame builder](../Frame%20Buffer.md)

The GUI first reads the status and releases its lock. It then converts the
immutable pixels into its own `ColorImage` and releases the image slot before
updating the GPU texture. It consumes at most one image per UI frame, even
when egui repeats layout. After a paused state change, the GUI keeps showing
the previous image until an image matching the CCK arrives.

Image publication, command replies and shutdown wake the native GUI through
`request_repaint`. In the browser, worker messages request the corresponding
GUI repaint.

When the emulator detects a fatal violation of an internal invariant, it uses
the shared helper. [`config::fatal_emulation_error`](../../../../crates/config/src/fatal.rs)

On native hosts, one outer unwind boundary covers the complete owner loop,
including Run, commands, manual steps and replay. It preserves the first
diagnostic and wakes the GUI. The owner terminates without publishing the
failed slice. Native builds require `panic=unwind` for this behavior.

The browser retains `panic_abort`. Before the trap, the host panic hook sends
its diagnostic to worker-local JavaScript storage. The outer worker handler
forwards that text to the main thread and terminates the worker without
tearing down the Rust stack or TLS. The main thread reports the error without
acquiring the failed owner's locks.

Both hosts keep the first diagnostic visible in the GUI, cancel queued commands
and reject further execution. The native application can close without waiting
for a Shutdown reply from a failed owner. In the browser, the error stays
visible on the page. Both paths leave the partially advanced machine stopped
and prevent it from being saved as a valid snapshot.

Releasing an image slot wakes the producer. A paused GUI waits for frame
notifications without polling or repeatedly sending the same UTC sample.
Native workers park while waiting for events and use short sleeps between
timed execution deadlines. Browser workers return to their event loop between
bounded slices. Message tasks continue work that is due immediately, and
timers handle future deadlines. The eframe browser runner continues to schedule
animation-frame callbacks while idle and runs GUI logic only when a repaint
is due.

The GUI requests inspection data from the owner through the command queue.
The owner reads a stopped machine to return a coherent view of CPU state,
physical memory, trace, breakpoints and the timeline. The timeline data
includes PCs and CCK positions. Historical CPU snapshots stay with the owner.
The GUI thread formats the returned data and prepares the visible rows.
Before Run, the owner captures an inspection snapshot of the stopped machine.
During continuous Run, the inspectors keep showing the prepared data. Game
does not request inspector snapshots.

Register edits, memory patches, breakpoints, history, load/save and quick slots
execute as owner commands. A binary load during Run retains the prepared
inspectors until execution stops. If the current field is unfinished, the
load waits for a completed field before publishing. File selection, host file
I/O and downloads remain host UI operations. Filenames and recent-file records
change only after successful owner replies.

The browser explicitly initializes one main Wasm instance. One worker instance
shares its memory. The launcher rebuilds the Rust standard library with
atomics and exports thread-local initialization data. Trunk provides the
required isolation headers and disables its automatic second initializer.
Work signaled by early wake messages remains pending in the shared queues until
the worker finishes its single initialization. [The launcher](../../../../tools/web.ps1)

Worker shutdown drops the owner and releases its stack and thread-local
storage before acknowledging completion. On native close, the application
waits asynchronously for the owner's final battery data before normal
persistence. Against the served application, the browser worker regression
checks ordinary and delayed startup, shared memory and idle notifications.
[browser worker regression](../../../../crates/gui/tests/browser_worker_runtime.py)

The frontend refreshes the RTC's UTC seconds on the first UI pass when its
latest coherent sample changes. The same rule applies during paused inspection
and when execution resumes. The frontend keeps one outstanding clock command
at a time. Native uses the system UTC clock, and the browser uses `Date.now()`.
Both round down to whole Unix seconds, including fractional timestamps before
the epoch. The application loads and saves the RTC battery offset as part of
its normal persistence lifecycle, under the independent `rtc_battery` storage
key. The offset is stored outside machine snapshots. [RTC](../RTC.md)

## Scaling and appearance

The interface uses logical sizes so controls and text remain readable at different desktop DPI settings and browser zoom levels. Interface zoom is separate from the Amiga image scale.

Both views use one **Image scale** selector with **Fit (smooth)**, **0.5x**,
**1x**, **2x**, and **4x**. The default, Fit, scales the complete image uniformly
to fit the display area and uses bilinear filtering. Fixed scales use
nearest-neighbor filtering and count physical host pixels per framebuffer
sample, independently of interface zoom and host DPI. Their origin aligns to
the host pixel grid. If the image is larger than its display area, the GUI
keeps the selected scale, centers the image and clips it to that area. There
is no automatic shrinking, no scrollbars and no separate filter control.
The selected scale persists with the other GUI preferences.

Image scale changes take effect while paused or running and affect only host
presentation. The machine state and framebuffer samples stay as they were.
Changing the filter during an unfinished field keeps the previously presented
image.

The GUI offers dark and light themes, plus a Workbench-inspired theme. A theme changes the host interface's appearance without changing emulation state. The same view and scaling principles apply in the native application and the browser.

## References

- [General Architecture](../General%20Architecture.md): Machine ownership and the boundary between emulation and host presentation.
- [Debugger](Debugger.md): Execution and inspection behavior behind the debugger view.
- [egui Guidelines](../egui%20Guidelines.md): Frontend implementation and interaction guidance.
- [Host input](../../../../crates/gui/src/host_input.rs): Ordered admission and peripheral delivery.
- [Host key mapping](../../../../crates/gui/src/host_keys.rs): Physical positions to Amiga codes.
- [GUI source](../../../../crates/gui/src/gui.rs): Native and browser entry points, view state, and layout modules.
