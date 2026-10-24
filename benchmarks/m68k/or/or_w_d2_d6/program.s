; Restore the destination before each OR so its bits do not accumulate toward ones.
entry:
    rept {{instruction_count}}
    or.w d1,d0
    move.l d5,d2
    or.w d3,d2
    move.l d7,d4
    or.w d5,d4
    move.l d1,d6
    or.w d7,d6
    move.l d3,d0
    endr
    dc.b "M68K_END_MARKER!"
