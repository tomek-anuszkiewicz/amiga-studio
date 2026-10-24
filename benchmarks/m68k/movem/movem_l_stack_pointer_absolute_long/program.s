; Transfer 1 l register(s) through absolute_long.
entry:
    rept {{instruction_count}}
    movem.l a7,$500000.l
    move.l d7,d0
    movem.l a7,$500000.l
    move.l d7,d0
    movem.l a7,$500000.l
    move.l d7,d0
    movem.l a7,$500000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
