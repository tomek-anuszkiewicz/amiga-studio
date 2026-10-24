; Measure the SUB.W source effective address with four interleaved targets.
entry:
    rept {{instruction_count}}
    sub.w $1000.w,d2
    move.l d7,d0
    sub.w $1000.w,d2
    move.l d7,d0
    sub.w $1000.w,d2
    move.l d7,d0
    sub.w $1000.w,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
