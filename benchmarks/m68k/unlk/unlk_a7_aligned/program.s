; Supervisor A7 restores its own value from an aligned frame; MOVE.L separates targets.
entry:
    rept {{instruction_count}}
    unlk a7
    move.l d6,d7
    unlk a7
    move.l d6,d7
    unlk a7
    move.l d6,d7
    unlk a7
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
