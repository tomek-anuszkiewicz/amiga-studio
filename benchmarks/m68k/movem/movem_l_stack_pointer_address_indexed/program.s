; Transfer 1 l register(s) through address_indexed.
entry:
    rept {{instruction_count}}
    movem.l a7,6(a3,d6.w)
    move.l d7,d0
    movem.l a7,6(a3,d6.w)
    move.l d7,d0
    movem.l a7,6(a3,d6.w)
    move.l d7,d0
    movem.l a7,6(a3,d6.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
