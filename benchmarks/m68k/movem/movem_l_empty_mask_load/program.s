; Empty-mask MOVEM load has no register transfers.
entry:
    rept {{instruction_count}}
    movem.l (a3),#0
    move.l d7,d0
    movem.l (a3),#0
    move.l d7,d0
    movem.l (a3),#0
    move.l d7,d0
    movem.l (a3),#0
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
