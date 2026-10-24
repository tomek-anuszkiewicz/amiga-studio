; Move w operands from absolute_long to data_register.
entry:
    rept {{instruction_count}}
    move.w $500000.l,d5
    move.l d3,d4
    move.w $500000.l,d5
    move.l d3,d4
    move.w $500000.l,d5
    move.l d3,d4
    move.w $500000.l,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
