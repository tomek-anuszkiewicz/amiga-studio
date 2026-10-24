; Empty-mask MOVEM store has no register transfers.
entry:
    rept {{instruction_count}}
    movem.l #0,(a3)
    move.l d7,d0
    movem.l #0,(a3)
    move.l d7,d0
    movem.l #0,(a3)
    move.l d7,d0
    movem.l #0,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
