; Move w operands from data_register to absolute_long.
entry:
    rept {{instruction_count}}
    move.w d2,$500100.l
    move.l d3,d4
    move.w d2,$500100.l
    move.l d3,d4
    move.w d2,$500100.l
    move.l d3,d4
    move.w d2,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
