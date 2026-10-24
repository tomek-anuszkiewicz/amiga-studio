; Move w operands from absolute_short to absolute_long.
entry:
    rept {{instruction_count}}
    move.w $1000.w,$500100.l
    move.l d3,d4
    move.w $1000.w,$500100.l
    move.l d3,d4
    move.w $1000.w,$500100.l
    move.l d3,d4
    move.w $1000.w,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
