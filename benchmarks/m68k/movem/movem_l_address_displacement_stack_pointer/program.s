; Transfer 1 l register(s) through address_displacement.
entry:
    rept {{instruction_count}}
    movem.l 6(a3),a7
    movea.l d7,a7
    movem.l 6(a3),a7
    movea.l d7,a7
    movem.l 6(a3),a7
    movea.l d7,a7
    movem.l 6(a3),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
