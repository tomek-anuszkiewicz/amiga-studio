; Restore the destination before each AND so its bits do not accumulate toward zero.
entry:
    rept {{instruction_count}}
    and.w d1,d0
    move.l d5,d2
    and.w d3,d2
    move.l d7,d4
    and.w d5,d4
    move.l d1,d6
    and.w d7,d6
    move.l d3,d0
    endr
    dc.b "M68K_END_MARKER!"
