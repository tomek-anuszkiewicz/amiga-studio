; Transfer 1 w register(s) through absolute_long.
entry:
    rept {{instruction_count}}
    movem.w $500000.l,a7
    movea.l d7,a7
    movem.w $500000.l,a7
    movea.l d7,a7
    movem.w $500000.l,a7
    movea.l d7,a7
    movem.w $500000.l,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
