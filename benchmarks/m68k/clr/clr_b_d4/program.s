; Measure CLR.B with the selected destination mode.
entry:
    rept {{instruction_count}}
    clr.b d4
    move.l d7,d0
    clr.b d4
    move.l d7,d0
    clr.b d4
    move.l d7,d0
    clr.b d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
