; Transfer 3 w register(s) through pc_indexed.
    dc.w $2007
entry:
    rept {{instruction_count}}
    movem.w *-10(pc,d6.w),d2/d5/a5
    move.l d7,d0
    movem.w *-10(pc,d6.w),d2/d5/a5
    move.l d7,d0
    movem.w *-10(pc,d6.w),d2/d5/a5
    move.l d7,d0
    movem.w *-10(pc,d6.w),d2/d5/a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
