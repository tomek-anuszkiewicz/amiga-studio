; Empty-mask MOVEM store has no register transfers.
entry:
    rept {{instruction_count}}
    movem.w #0,(a3)
    move.l d7,d0
    movem.w #0,(a3)
    move.l d7,d0
    movem.w #0,(a3)
    move.l d7,d0
    movem.w #0,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
