; Transfer 3 l register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.l $1000.w,d2/d5/a5
    move.l d7,d0
    movem.l $1000.w,d2/d5/a5
    move.l d7,d0
    movem.l $1000.w,d2/d5/a5
    move.l d7,d0
    movem.l $1000.w,d2/d5/a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
