; Measure the SUB.L source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.l $1000.w,d2
    move.l d7,d0
    sub.l $1000.w,d2
    move.l d7,d0
    sub.l $1000.w,d2
    move.l d7,d0
    sub.l $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
