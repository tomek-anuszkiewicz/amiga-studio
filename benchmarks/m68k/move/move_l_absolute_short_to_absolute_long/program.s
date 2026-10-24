; Move l operands from absolute_short to absolute_long.
entry:
    rept {{instruction_count}}
    move.l $1000.w,$500100.l
    move.l d3,d4
    move.l $1000.w,$500100.l
    move.l d3,d4
    move.l $1000.w,$500100.l
    move.l d3,d4
    move.l $1000.w,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
