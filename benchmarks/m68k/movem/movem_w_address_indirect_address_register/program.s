; MOVEM.W loads A5 from (A3); MOVEA.L restores A5 between targets.
entry:
    rept {{instruction_count}}
    movem.w (a3),a5
    movea.l d7,a5
    movem.w (a3),a5
    movea.l d7,a5
    movem.w (a3),a5
    movea.l d7,a5
    movem.w (a3),a5
    movea.l d7,a5
    endr
    dc.b "M68K_END_MARKER!"
