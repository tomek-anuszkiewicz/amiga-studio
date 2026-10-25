; Transfer 1 l register(s) through pc_indexed.
    dc.w $2e47
entry:
    rept {{instruction_count}}
    movem.l *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.l *-10(pc,d6.w),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
