; Subtract a stable register source from the destination word.
entry:
    rept {{instruction_count}}
    sub.w d0,d1
    move.l d6,d7
    sub.w d2,d3
    move.l d6,d7
    sub.w d4,d5
    move.l d6,d7
    sub.w d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
