; Move l operands from absolute_long to data_register.
entry:
    rept {{instruction_count}}
    move.l $500000.l,d5
    move.l d3,d4
    move.l $500000.l,d5
    move.l d3,d4
    move.l $500000.l,d5
    move.l d3,d4
    move.l $500000.l,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
