# Game library and manual setup

The desktop app opens to the game library. In **Configurations**, use
**Choose library directory...** under **Application settings** and click
**Save**. **Cancel** keeps the previous directory.
The library updates automatically when files change. Type part of a game's name
in **Filter games** to find it, and clear the field to show all games again.
To start standalone disks, choose **Manual setup**. The browser opens there.
Add a 256 KiB image with **Add Kickstart** in **Configurations**. In Manual
setup, choose a named configuration and an available Kickstart. Select any
standard 880 KiB DD ADF images, then choose **Start**. You can leave drives empty.

Each configuration combines one of the three supported memory presets with a
PAL/NTSC setting, ROM, attached drives and input mappings. DF0 is always attached.
You can attach external drives as needed. Choose a device and its host source
for each port. Use the [player controls guide](player/README.md) to record
bindings and choose a keyboard joystick preset. The app binds the controller
connection you select. If it becomes unavailable, select an available
controller. After reconnecting or returning focus to the app, release any
held controller inputs before using them again.

On desktop, each library keeps its profiles and reusable mappings in
`configurations.json`. **Add Kickstart** copies the file into `kickstarts/`.
Save a firmware choice for each profile. An ordinary game boot uses that
profile's choice. **Return to session**, **Resume** and **Reset** keep the
current machine's ROM. Copy the entire library to bring the profiles, games
and assets to another installation. The library directory and recent picker
directories belong to your user account.

**Manage Kickstarts** lists the library's ROM copies. Change any saved profile
listed when you try to delete an entry, then confirm **Delete Kickstart**.
The original file and the current machine remain available. Keep the exact ROM
bytes for existing saves. Use **Supply Kickstart for restore** in
**Configurations** to offer an external ROM for a file load.

Switching libraries changes the profiles available for future launches. You can
return to the live session and continue using its controls and saves in its
original library. Finish or discard any profile edits before switching.

Keyboard and controller input work as soon as the running session is active.
Click the Amiga display to capture the mouse. The middle mouse button frees
the mouse while play continues. Configuration screens, text editors and
dialogs take priority over guest input. The player panel shows drive activity
and has four disk dropdowns, Pause, Resume, Reset, Quick save and Quick load.
Drag the divider to resize the panel. Small windows put the
same controls under **Game controls** below the display. Open **Instructions**
there to read the [player guide](player/README.md) without leaving the session.
Use **Manual setup** in the toolbar to reach **Configurations** or start another
machine. **Return to session** opens the current machine. Press F12 to switch
between Game and Developer Studio.

If the keyboard joystick uses the arrows, choose a different port source or
change its arrow bindings. This frees the arrows for the Amiga keyboard. In
**Amiga keyboard remapping**, you can assign keypad **(** and **)** to spare
keys. You can also assign **Menu**, if your host delivers it. Select **Save**
to keep these assignments.

Escape follows its Amiga mapping while the desktop app has the mouse
captured, even if an older profile lists Escape as its release shortcut.
Use middle mouse or assign another release key. The Windows app requests
logo-key suppression while the running session has focus and mouse capture.
The browser can release pointer lock on Escape and can only pass on keys it
receives. Use a spare key if Windows or your browser handles a key first.

## Add or edit a desktop game

Choose a library directory and click **Add game**. Enter the name that the app
will use for both the game and its folder, then select a named configuration.
You can add a description. Click **Add disks...** to select one or more ADF,
ZIP or 7z files. The app lists supported ADFs from those files and archives
in the draft. You can open the picker again to add more. The app remembers
the last confirmed disk directory across desktop restarts.

**Disk already added** means the same image is already pending or stored for
the game, even if you selected it under another name. A filename collision
means different images would need the same destination name. The app keeps
the earlier disk and imports other valid selections. Remove a conflicting
pending import or choose a differently named source if you need the other image.

Under **Startup disks**, choose a disk or **Empty** for each drive. All four
drives start set to **Empty**. Select a disk for a connected drive if you want
it inserted at startup. **Save game** copies the ADFs into the game's
`disks/` folder using their original filenames. You can move the source
files and archives after saving.

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
    {"id": "disk-1", "filename": "Example Game (Disk 1 of 2).adf"},
    {"id": "disk-2", "filename": "Example Game (Disk 2 of 2).adf"}
  ],
  "mounted": ["disk-1", null, null, null],
  "initial_state": null,
  "artwork": "cover.png",
  "description": "description.md"
}
```

Give each disk a unique ID within its game. Each disk has exactly `id` and
`filename`. Place the file in `disks/` and use its basename, including the
ADF extension, in `filename`. The four `mounted` positions specify the startup disk IDs for DF0
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

**Quick save** creates a file in the game's `saves/` directory. The new
quicksave appears at the top of the saved progress list. Earlier quicksaves
stay in the list as ordinary saves. Each save shows its original local date
and time and a preview when available. Use **Edit description** to change its
label while keeping the same file and creation time. You can use the preview
and edit the description while the original images are offline.

**Quick load** restores the designated quicksave. To open any compatible save,
choose **Load state** on its row. Both actions verify the matching Kickstart
and ADF sources before restoring the captured machine and resuming play with
the current committed controls. An incompatible or damaged save
shows its filename and the reason it cannot load. Use **Delete save** to remove
a save. **Undo** keeps it if chosen within ten seconds. Closing the app during
that window also keeps it. You can rename the game after all its pending save
deletions end or are undone. If a save is removed but its **Quick load**
designation cannot be cleared, the error dialog explains what happened. The
**Quick load** shortcut reports **Saved game state not found** if the
designation still points to that missing file. Choose **Import state** or
**Export state** in the game controls to work with a snapshot file directly.

When an Amiga program writes to disk A, you can switch to disk B and reinsert
A later in the same session. A still contains the program's changes, and the
original ADF file stays unchanged. Opening a save file, including a durable
quicksave or named start state, restores each mounted disk from its matching
ADF. In a manual session, **Quick save** keeps the mounted disk contents in
memory. **Quick load** during a session also keeps other disks already used
in that session. On game Reset, the app uses matching copies from the session
when available. If no matching copy is available for a startup disk, it
prepares one from the source ADF bytes already loaded for the session.

## Keep named start states

Choose **Add to start states** beside a save or the marked **Quicksave** in
the library or in **Saved progress**. Enter a name such as Intro or Level 1
and click **Add start state**. The game keeps its own copy in
`initial_states/`. You can remove the source save or replace your quicksave
and open the copy later.

Select the game in the library and choose **Load** beside a name to restore
that entry's machine and mounted disks. A missing, damaged or incompatible
snapshot disables **Load** and shows the cause. If the matching Kickstart is
missing, you can still choose **Load** and the app reports the missing image.
**Rename** changes only the label.
**Delete** gives you ten seconds to **Undo**, even if you leave the game
listing. Closing the app within that window keeps the entry and its file.
Named start states only load from their own rows. **Reset** uses the game's
startup disks.

## Browser storage

Both desktop and browser manual setup accept ADF files and ADFs in ZIP or 7z
archives. When a selection contains several usable images, choose the one to
insert. Standard ZIP compression and 7z LZMA or LZMA2 are supported. Choose an
unencrypted archive.

The browser build supports manual setup, file import and export, and one
in-memory quicksave per open app. Browser storage retains named settings.
On a fresh page, add the matching Kickstart in **Configurations** and select
the disk files in **Manual setup**. Choose **Return to session**, then **Import
state** to restore a file. Use **Manage Kickstarts** to remove a supplied ROM
from the current page after changing any saved profiles that use it. This
iteration has no browser library, bundle loader or persistent browser save
catalog.

Saves use schema 17. The desktop catalog keeps older files visible as
incompatible. Opening an older file reports its unsupported version.
