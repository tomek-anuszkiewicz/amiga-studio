# Amiga 500 player controls

Open **Configurations** and choose the configuration you want to edit. Click
the current binding and press a key or use the selected controller when
prompted. **Cancel recording** keeps the old binding. Choose **Save** to apply
your changes or **Cancel** to discard the draft.

- **Reusable joystick mappings** drive a joystick attached to an Amiga port.
  Choose **WASD** for W/S/A/D and J/K for Fire 1/2, or **Arrow keys** for
  the arrows and Z/X for Fire 1/2. A connected controller can use its buttons
  and stick axes. To delete a mapping, first reassign the listed configurations
  and active session.
- **Amiga keyboard remapping** sends a host key to a different Amiga key.
  For example, select **Escape**, click **Add key override**, then press an
  unused host key such as V. The Amiga receives Escape when you press V.
  To use V as usual again, remove the override and save. A key already used
  for a joystick or emulator shortcut must be freed before you can assign it.
  If a joystick uses the arrows, change its bindings or choose a different
  port source to use all four as Amiga arrow keys. Select **Keypad (** or
  **Keypad )** to assign a spare host key to either keypad parenthesis.
  You can record the host **Menu** key when the app receives it.
- **Emulator shortcuts** control the app. Pause / Resume starts at F5, Quick
  save at F6, and Quick load at F9. Quick actions use slot 1. Hover a
  session control to see its current keyboard equivalent. F12 switches
  between Game and Developer Studio.
- **Controller shortcuts** give a connected controller access to emulator
  actions. The app keeps assignments when no controller is detected.
  Select an available controller if the saved connection is unavailable.

Click the Amiga display to capture the mouse. Middle mouse frees the mouse
while the keyboard and controller keep working. You can also assign a mouse
release key. During captured desktop gameplay, Escape follows its Amiga
mapping even if an older configuration lists it as a release shortcut.
The Windows app requests logo-key suppression while focused, running and
captured so the Windows keys can act as Amiga modifiers. In a browser,
Escape can release pointer lock independently of its Amiga mapping. If your
host handles a key first, assign a spare key to the Amiga key you need.

For disk selection, startup, reset and saved progress, see
[Game library and manual setup](../game-library.md).
