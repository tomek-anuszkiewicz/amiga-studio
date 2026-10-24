; Transfer 3 w register(s) through absolute_short.
entry:
    rept {{instruction_count}}
    movem.w $1000.w,d2/d5/a5
    move.l d7,d0
    movem.w $1000.w,d2/d5/a5
    move.l d7,d0
    movem.w $1000.w,d2/d5/a5
    move.l d7,d0
    movem.w $1000.w,d2/d5/a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
