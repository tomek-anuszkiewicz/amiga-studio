; Transfer 1 w register(s) through pc_indexed.
    dc.w $2e47
entry:
    rept {{instruction_count}}
    movem.w *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.w *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.w *-10(pc,d6.w),a7
    movea.l d7,a7
    movem.w *-10(pc,d6.w),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
