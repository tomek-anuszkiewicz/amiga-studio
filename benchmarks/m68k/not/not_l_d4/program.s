; Measure NOT.L with the selected destination mode.
entry:
    rept {{instruction_count}}
    not.l d4
    move.l d7,d0
    not.l d4
    move.l d7,d0
    not.l d4
    move.l d7,d0
    not.l d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
