; Move b operands from absolute_long to absolute_short.
entry:
    rept {{instruction_count}}
    move.b $500000.l,$2000.w
    move.l d3,d4
    move.b $500000.l,$2000.w
    move.l d3,d4
    move.b $500000.l,$2000.w
    move.l d3,d4
    move.b $500000.l,$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
