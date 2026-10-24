; Transfer 1 l register(s) through address_displacement.
entry:
    rept {{instruction_count}}
    movem.l d2,6(a3)
    move.l d7,d0
    movem.l d2,6(a3)
    move.l d7,d0
    movem.l d2,6(a3)
    move.l d7,d0
    movem.l d2,6(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
