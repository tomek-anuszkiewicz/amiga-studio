; Transfer 1 l register(s) through absolute_long.
entry:
    rept {{instruction_count}}
    movem.l $500000.l,a7
    movea.l d7,a7
    movem.l $500000.l,a7
    movea.l d7,a7
    movem.l $500000.l,a7
    movea.l d7,a7
    movem.l $500000.l,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
