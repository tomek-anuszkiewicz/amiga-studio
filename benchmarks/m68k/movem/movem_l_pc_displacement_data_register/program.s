; Transfer 1 l register(s) through pc_displacement.
    dc.w $2007
entry:
    rept {{instruction_count}}
    movem.l *-2(pc),d2
    move.l d7,d0
    movem.l *-2(pc),d2
    move.l d7,d0
    movem.l *-2(pc),d2
    move.l d7,d0
    movem.l *-2(pc),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
