; Store D2 and supervisor A7 through a fixed address.
entry:
    rept {{instruction_count}}
    movem.l d2/a7,(a3)
    move.l d7,d0
    movem.l d2/a7,(a3)
    move.l d7,d0
    movem.l d2/a7,(a3)
    move.l d7,d0
    movem.l d2/a7,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
