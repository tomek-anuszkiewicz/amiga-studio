; Transfer 1 l register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.l $1000.w,d2
    move.l d7,d0
    movem.l $1000.w,d2
    move.l d7,d0
    movem.l $1000.w,d2
    move.l d7,d0
    movem.l $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
