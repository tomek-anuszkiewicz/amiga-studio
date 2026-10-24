; Transfer 1 w register(s) through address_displacement.
entry:
    rept {{instruction_count}}
    movem.w d2,6(a3)
    move.l d7,d0
    movem.w d2,6(a3)
    move.l d7,d0
    movem.w d2,6(a3)
    move.l d7,d0
    movem.w d2,6(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
