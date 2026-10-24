; Compare stable registers without a conditional branch.
entry:
    rept {{instruction_count}}
    cmp.w d0,d1
    move.l d6,d7
    cmp.w d2,d3
    move.l d6,d7
    cmp.w d4,d5
    move.l d6,d7
    cmp.w d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
