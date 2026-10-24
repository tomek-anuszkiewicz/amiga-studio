; Move w operands from absolute_long to absolute_long.
entry:
    rept {{instruction_count}}
    move.w $500000.l,$500100.l
    move.l d3,d4
    move.w $500000.l,$500100.l
    move.l d3,d4
    move.w $500000.l,$500100.l
    move.l d3,d4
    move.w $500000.l,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
