; Measure CLR.L with the selected destination mode.
entry:
    rept {{instruction_count}}
    clr.l d4
    move.l d7,d0
    clr.l d4
    move.l d7,d0
    clr.l d4
    move.l d7,d0
    clr.l d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
