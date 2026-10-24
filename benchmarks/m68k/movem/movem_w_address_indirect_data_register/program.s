; Transfer 1 w register(s) through address_indirect.
entry:
    rept {{instruction_count}}
    movem.w (a3),d2
    move.l d7,d0
    movem.w (a3),d2
    move.l d7,d0
    movem.w (a3),d2
    move.l d7,d0
    movem.w (a3),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
