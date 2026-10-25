; Transfer 1 w register(s) through address_displacement.
entry:
    rept {{instruction_count}}
    movem.w 6(a3),a7
    movea.l d7,a7
    movem.w 6(a3),a7
    movea.l d7,a7
    movem.w 6(a3),a7
    movea.l d7,a7
    movem.w 6(a3),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
