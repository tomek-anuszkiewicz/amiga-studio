; Measure CLR.L with the selected destination mode.
entry:
    rept {{instruction_count}}
    clr.l $1000.w
    move.l d7,d0
    clr.l $1000.w
    move.l d7,d0
    clr.l $1000.w
    move.l d7,d0
    clr.l $1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
