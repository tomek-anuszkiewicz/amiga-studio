# Amiga 500 player guide

Run Amiga disks and programs on an emulated Amiga 500. The desktop opens the
game library. The browser opens Manual setup.

## Start a game

On desktop, open Configurations. Under Application settings, choose your
library directory and select Save. Cancel keeps the previous directory.
The library picks up file and cover changes automatically. Filter games
matches part of the displayed name.

Use Configurations to choose the hardware, drives and controls for a named
profile. Keep desktop Kickstart files in the library and enter their paths
relative to the library directory. Use Add game or Edit game to choose a
configuration, import ADF disks from files or ZIP and 7z archives, and set
which disks are inserted at startup. You can add a description and cover.
Select a game and choose Start game.

In Manual setup, choose a configuration, select a 256 KiB Kickstart and add
any optional disk images. Choose Start. You can leave drives empty. In the
browser, select the Kickstart file again after reopening the app.

## Play and save

Use the DF0 through DF3 selectors to insert or eject disks. Reset boots a
library game with its startup disks and resumes execution. In a manual
session, Reset boots with the disks already inserted.

Pause stops at the end of the current video field. Resume continues from
there. Use Save and Load in the player controls to keep or restore progress.
For a desktop library game, Load opens its saves. In a manual session, Save
exports a snapshot file and Load imports one. Keep the source Kickstart and
disk images for your saves. After reopening a manual session, select the
matching files in Manual setup, choose Return to session, and choose Load.
The State menu also provides quick saves and loads. The configured quick
shortcuts use slot 1.

For a desktop library game, choose Add to start states next to a save or
quicksave to keep a named copy. Select it under the game's Start states to
resume from that point. Screenshot saves the displayed image to the game
folder. Cover uses the image as the game's library cover.

## Use the keyboard, mouse and controller

Click the Amiga display to capture the mouse. Middle mouse frees the mouse
while the keyboard and controller keep working. You can configure a mouse
release key. Choose a key other than Escape, which the browser can use to
release the pointer.

Open Configurations to see and edit the actual bindings. Choose a keyboard
joystick preset or record a mapping for a connected controller. You can also
remap Amiga keyboard keys and change emulator shortcuts, including Pause /
Resume and the quick actions. Select a binding and press the new key or use
the assigned controller when prompted. Cancel recording keeps the current
binding. To map a host key to an Amiga key, select the Amiga key, choose Add
key override and press an unused host key. Remove the override to restore
the original mapping. Save applies the edits. Cancel discards the draft.

From the setup screens, choose Return to session to open the same machine.
In the session, press F12 to switch between Game view and Developer Studio.
Entering Developer Studio pauses the machine for inspection.
