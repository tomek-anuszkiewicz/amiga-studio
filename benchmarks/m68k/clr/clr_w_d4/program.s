; Clear a data register word.
entry:
    rept {{instruction_count}}
    clr.w d4
    move.l d7,d0
    clr.w d4
    move.l d7,d0
    clr.w d4
    move.l d7,d0
    clr.w d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
