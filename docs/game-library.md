# Game library and manual setup

The desktop app opens to the game library. In **Configurations**, use
**Choose library directory...** under **Application settings** and click
**Save**. **Cancel** keeps the previous directory.
The library updates automatically when files change. Type part of a game's name
in **Filter games** to find it, and clear the field to show all games again.
To start standalone disks, use manual setup. The browser opens to manual setup.
Choose a named configuration, select a supported 256 KiB Kickstart and any
standard 880 KiB DD ADF images, then choose Start. You can leave drives empty.

Each configuration combines one of the three supported memory presets with a
PAL/NTSC setting, ROM, attached drives and input mappings. DF0 is always attached.
You can attach external drives as needed. Choose a device and its host source
for each port. Use the [player controls guide](player/README.md) to record
bindings and choose a keyboard joystick preset. The app binds the controller
connection you select. If it becomes unavailable, select an available
controller. After reconnecting or returning focus to the app, release any
held controller inputs before using them again.

On desktop, each library keeps its profiles and reusable mappings in
`configurations.json`. Keep the Kickstart files inside the library and enter
their paths relative to the library directory. Copy the entire library to bring
these profiles, games and assets to another installation. Your library directory
choice and interface preferences belong to your user account.

Switching libraries changes the profiles available for future launches. You can
return to the live session and continue using its controls and saves in its
original library. Finish or discard any profile edits before switching.

Keyboard and controller input work as soon as the running session is active.
Click the Amiga display to capture the mouse. The middle mouse button frees
the mouse while play continues. Configuration screens, text editors and
dialogs take priority over guest input. The player panel shows drive activity
and has four disk dropdowns and controls to eject disks, pause, resume, reset,
save and load. Small windows move these controls below the display. Developer
Studio uses the same machine.

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
Renaming moves the whole folder, including saved progress and start states,
and preserves disk IDs. Cancel discards the draft without writing files. Use
the player panel to switch or eject disks during play.

Click **Start game** in the library to boot with your saved startup disks.
If an older game manifest points to a legacy initial state, **Start game**
opens that snapshot. Choose **Boot normally without initial state** to use
the startup disks, including when the legacy snapshot is unavailable.

If the game shows **Configuration missing**, select a profile from the current
library and click **Save reference and continue**. The app writes that choice
to the game before it attempts to launch. If there are no profiles to choose
from, create one in Configurations. Cancel keeps the game's manifest as it was.

To remove a profile, select it in **Configurations** and click **Delete
configuration**. Review which games use it and choose a replacement when the list
contains games. The app checks those games' disks and startup drives, saves and
verifies their new references, then deletes the profile. If an update fails, the
app keeps the old profile and reports which references it already wrote. Open
the deletion dialog again to check the remaining references.

**Reset** in the player panel inserts the active game's saved startup disks,
clears drives set to **Empty** and boots with the current hardware and ROM. If
all four choices are **Empty**, normal boot and game Reset leave every drive
empty. Editing startup choices during a session takes effect at the next Reset
or normal boot. In manual sessions, Reset keeps the disks currently inserted.
A successful Reset starts execution even if you had paused. If the player cannot
prepare an assigned disk or the drive is disconnected, it reports the problem
and keeps the session's current disks and execution state.

Under **Cover image** in Add game or Edit game, click **Choose image...** to
select a JPEG or PNG, or copy an image and press **Ctrl+V** (**Command+V** on
macOS). The editor shows a proportional preview. Text pastes normally into
text fields. **Save game** stores a copy in the game folder and shows the new
cover on the game's tile when the Library returns. You can then move or delete
the source file or replace the clipboard contents. **Cancel** leaves the game's
files as they were.

During a library game, use **Cover** beside **Screenshot** to set the running
game's thumbnail from the complete displayed image. Each capture replaces
`cover.png` beside `game.json`. Return to the library to see the new thumbnail.
**Screenshot** creates a separate numbered file in `Screenshots/`. Saved
progress carries its own preview.

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

If `artwork` is `null`, the library checks `cover.jpg`, `cover.jpeg` and
`cover.png` beside `game.json` in that order. A declared JPEG or PNG path takes
precedence. A missing or unreadable cover shows a blank outlined square above
the game's selectable name. You can still start the game. The library picks up
cover changes automatically.

Save creates a new file in the game's `saves/` directory with a preview and a UTC
creation time. **Load state** restores the saved hardware, ROM and current media
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

## Keep named start states

Choose **Add to start states** beside a save or the marked **Quicksave** in
the library or in **Saved progress**. Enter a name such as Intro or Level 1
and click **Add start state**. The game keeps its own copy in
`initial_states/`. You can remove the source save or replace your quicksave
and open the copy later.

Select the game in the library and choose **Load** beside a name to restore
that entry's machine and mounted disks. Unavailable entries show the cause
and leave the other entries available. **Rename** changes only the label.
**Delete** gives you ten seconds to **Undo**, even if you leave the game
listing. Closing the app finalizes pending deletions. Named start states only
load from their own rows. **Reset** uses the game's startup disks.

## Browser storage

The current browser build supports manual setup, file save/import/export and
session quick slots. Browser storage retains named settings. Select ROM and disk
files again after reopening because the browser build keeps their bytes only for
the session. This iteration has no browser library, bundle loader or persistent
browser save catalog.

Saves explicitly use schema 15. The app leaves earlier snapshots in place and
cannot load or migrate them.
