; Transfer 3 w register(s) through absolute_long.
entry:
    rept {{instruction_count}}
    movem.w d2/d5/a5,$500000.l
    move.l d7,d0
    movem.w d2/d5/a5,$500000.l
    move.l d7,d0
    movem.w d2/d5/a5,$500000.l
    move.l d7,d0
    movem.w d2/d5/a5,$500000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
