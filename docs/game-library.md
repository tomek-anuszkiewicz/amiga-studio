# Game library and manual setup

Desktop opens the library. Select a root directory, then refresh after adding a
game directory. Manual setup starts standalone disks and is also the browser's
entry screen. Choose a named configuration, select a supported 256 KiB Kickstart
and any standard 880 KiB DD ADF images, then Start. Empty drives are valid.

Configurations combine one of the three supported memory presets with PAL/NTSC,
ROM selection, attached drives and input mappings. DF0 is always attached;
external drives are optional. Configure devices before their host sources.
Keyboard mappings use physical Amiga key positions in hexadecimal. The default
joystick uses arrows, numpad 0 and numpad decimal. Controller choices name a
specific connection; unavailable choices require reassignment. After reconnect
or application focus loss, release held controller inputs before using them again.

Click the Amiga display to capture keyboard and mouse input. The middle mouse
button always releases capture without pausing. An optional release key can be
assigned in Configurations; it starts empty. F12 always switches Game/Debug and
cannot be reassigned. F5 pauses/resumes and F6/F9 save/load slot 1; these three
bindings are configurable. Reset is available through the interface without a
keyboard shortcut. Escape and F2 otherwise reach the Amiga. Browser pointer lock
retains its own Escape release behavior. Editing a host text field takes priority
over guest input. Gamepads work in an active, running session without capture;
configuration screens and inactive application windows disable gameplay input. The right player panel supplies four
disk dropdowns, eject, activity, pause/resume, reset and save/load. Small windows
collapse these controls below the display. Debug uses the same machine.

## Add or edit a desktop game

Choose a library directory, then use **Add game**. Enter a name and select a
named configuration. The name is also the game's folder name. A description is
optional. Use **Add disk...** once per ADF; **Save game** copies those files into
the game folder and assigns the first disk to DF0. Original files stay untouched.
An empty disk list is valid for a ROM-only machine.

Select an existing entry and use **Edit game** to change its name, description
or configuration and append more disks. Renaming moves the whole folder,
including saved progress and any initial state. Disk IDs and assignments stay
unchanged. Cancel discards the draft without writing files. Existing media are
retained; switch or eject them in the player panel.

For a cover, place **cover.jpg**, **cover.jpeg** or **cover.png** next to game.json
and click **Refresh**. No manifest edit is needed. A declared artwork path takes
precedence over this filename convention. JPEG and PNG covers are supported.

## Prepare a game manually

Create one subdirectory per game and place a `game.json` inside it:

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

Disk IDs must be unique within the game. Asset paths are relative to its
directory. Artwork and description are optional; missing optional files do not
prevent launch. Use Edit game to select a configuration, then Save game to write
that association. Configuration names can change without changing their IDs.
Moving a game directory preserves its relative media paths; another installation
must supply its own configuration and ROM.

Save creates a new file in the game's `saves/` directory, with a preview and UTC
creation time. Continue restores saved hardware, ROM and current media before
resuming. It does not boot first or apply the current hardware preset. A compatible
configuration is still needed for host input. Slot saves also persist per game.

To prepare an initial state, start the associated game, save progress, copy that
save outside `saves/`, and set `initial_state` to the relative filename. Ordinary
saving never overwrites it. A broken initial state shows an error; Boot normally
without initial state is an explicit alternative.

## Browser storage

The current browser build has manual setup, file save/import/export and session
quick slots. Named settings persist in browser storage. ROM and disk bytes do
not: select those files again after reopening. There is no browser library,
bundle loader or persistent browser save catalog in this iteration.

Save schema 10 is explicit. Earlier snapshot files are unsupported; no migration
or automatic deletion occurs. Media writes and ADF write-back remain unavailable.
