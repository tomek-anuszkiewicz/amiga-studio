; Measure the SUB.B source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.b $1000.w,d2
    move.l d7,d0
    sub.b $1000.w,d2
    move.l d7,d0
    sub.b $1000.w,d2
    move.l d7,d0
    sub.b $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
