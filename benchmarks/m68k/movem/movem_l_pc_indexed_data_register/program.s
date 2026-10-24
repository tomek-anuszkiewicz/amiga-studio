; Transfer 1 l register(s) through pc_indexed.
    dc.w $2007
entry:
    rept {{instruction_count}}
    movem.l *-10(pc,d6.w),d2
    move.l d7,d0
    movem.l *-10(pc,d6.w),d2
    move.l d7,d0
    movem.l *-10(pc,d6.w),d2
    move.l d7,d0
    movem.l *-10(pc,d6.w),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
