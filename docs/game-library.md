# Game library and manual setup

The desktop app opens to the game library. Select the library's root directory.
Refresh the library after adding a game directory. To start standalone disks,
use manual setup. The browser opens to manual setup. Choose a named
configuration, select a supported 256 KiB Kickstart and any standard 880 KiB DD
ADF images, then choose Start. You can leave drives empty.

Each configuration combines one of the three supported memory presets with a
PAL/NTSC setting, ROM, attached drives and input mappings. DF0 is always attached.
You can attach external drives as needed. Configure the devices before you choose
their host input sources. Keyboard mappings use physical Amiga key positions in
hexadecimal. By default, the joystick uses the arrow keys, numpad 0 and numpad
decimal. When you select a controller, the app binds that specific connection.
If the connection is unavailable, reassign it. After reconnecting a controller
or after the app loses focus, release any held controller inputs before using
them again.

The keyboard works as soon as the running session is active. Click the Amiga
display to capture the mouse. The middle mouse button frees the mouse while
play continues. You can assign a mouse release key in Configurations, where
it starts unassigned. F12 switches between Game and Debug views. F5 pauses or resumes, F6 saves to slot 1, and F9 loads slot 1. You can
reassign those three keys. Use the interface to reset. Reset has no keyboard
shortcut. Escape and F2 otherwise reach the Amiga. The browser still uses Escape
to release pointer lock. Host text editors and dialogs take priority over
guest input.

Gamepads work in an active, running session even without capture. Configuration
screens and inactive app windows disable gameplay input. The right player panel
shows activity and has four disk dropdowns, plus controls for ejecting,
pausing/resuming, resetting and saving/loading. Small windows move these controls
below the display. Debug uses the same machine.

## Add or edit a desktop game

Choose a library directory and click **Add game**. Enter the name that the app
will use for both the game and its folder, then select a named configuration.
You can add a description. Click **Add disk...** once per ADF. Under
**Startup disks**, choose a disk or **Empty** for each drive. All four drives
start set to **Empty**. After adding a disk, select it for a connected drive
if you want it inserted at startup. **Save game** copies the imported ADFs into
the game's `disks/` folder and keeps the original files intact.

Select an existing entry and click **Edit game** to change its name, description
or configuration, append more disks or change its startup assignments.
Renaming moves the whole folder, including saved progress and any initial state,
and preserves disk IDs. Cancel discards the draft without writing files. Use
the player panel to switch or eject disks during play.

Click **Start game** in the library. The app loads the game's initial state if
one is configured. Otherwise it boots normally with your saved startup disks.
If the initial state fails to load, choose **Boot normally without initial
state** to start with those disks. Loading an initial state or saved progress
restores the inserted disks and empty drives recorded in that save.

**Reset** in the player panel inserts the active game's saved startup disks,
clears drives set to **Empty** and boots with the current hardware and ROM. If
all four choices are **Empty**, normal boot and game Reset leave every drive
empty. Editing startup choices during a session takes effect at the next Reset
or normal boot. In manual sessions, Reset keeps the disks currently inserted.
A successful Reset starts execution even if you had paused. If the player cannot
prepare an assigned disk or the drive is disconnected, it reports the problem
and keeps the session's current disks and execution state.

For a cover, place **cover.jpg**, **cover.jpeg** or **cover.png** next to game.json
and click **Refresh**. No manifest edit is needed. A declared artwork path takes
precedence over this filename convention. JPEG and PNG covers are supported.

## Prepare a game manually

Create one subdirectory per game and place a `game.json` inside it.

```json
{
  "id": "example-game",
  "title": "Example Game",
  "configuration": "standard",
  "disks": [
    {"id": "disk-1", "label": "Disk 1", "path": "disks/disk-1.adf"},
    {"id": "disk-2", "label": "Disk 2", "path": "disks/disk-2.adf"}
  ],
  "mounted": ["disk-1", null, null, null],
  "initial_state": null,
  "artwork": "cover.png",
  "description": "description.md"
}
```

Give each disk a unique ID within its game. Asset paths are relative to the game
directory. The four `mounted` positions specify the startup disk IDs for DF0
through DF3. Use `null` for an empty drive. You can launch without artwork or a
description, even if the optional files are missing. Use Edit game to select a
configuration and Save game to write that association. You can rename
configurations while keeping their IDs. Moving a game directory keeps its
relative media paths valid. Another installation must
supply its own configuration and ROM.

Save creates a new file in the game's `saves/` directory with a preview and a UTC
creation time. Continue restores the saved hardware, ROM and current media
directly and resumes the session. It bypasses boot and the current hardware
preset. Host input still needs a compatible configuration. Slot saves also
persist per game.

When an Amiga program writes to disk A, you can switch to disk B and reinsert
A later in the same session. A still contains the program's changes, and the
original ADF file stays unchanged. A save file carries the disks inserted when
you save. If you want to open a changed disk in a later session, insert it
before saving. A quick load during the session restores the slot's mounted
disks and keeps the other disks you have used in that session. On game Reset,
the app uses matching copies from the session when available. If no matching
copy is available for a startup disk, it prepares one from the source ADF
bytes already loaded for the session.

To prepare an initial state, start its game and save progress. Copy that save
outside `saves/` and set `initial_state` to the relative filename. Ordinary saves
leave the initial state intact.

## Browser storage

The current browser build supports manual setup, file save/import/export and
session quick slots. Browser storage retains named settings. Select ROM and disk
files again after reopening because the browser build keeps their bytes only for
the session. This iteration has no browser library, bundle loader or persistent
browser save catalog.

Saves explicitly use schema 15. The app leaves earlier snapshots in place and
cannot load or migrate them.
