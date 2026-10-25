; Transfer 1 l register(s) through pc_displacement.
    dc.w $2e47
entry:
    rept {{instruction_count}}
    movem.l *-2(pc),a7
    movea.l d7,a7
    movem.l *-2(pc),a7
    movea.l d7,a7
    movem.l *-2(pc),a7
    movea.l d7,a7
    movem.l *-2(pc),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
