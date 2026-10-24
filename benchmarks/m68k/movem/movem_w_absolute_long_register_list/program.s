; Transfer 3 w register(s) through absolute_long.
entry:
    rept {{instruction_count}}
    movem.w $500000.l,d2/d5/a5
    move.l d7,d0
    movem.w $500000.l,d2/d5/a5
    move.l d7,d0
    movem.w $500000.l,d2/d5/a5
    move.l d7,d0
    movem.w $500000.l,d2/d5/a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
