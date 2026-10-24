; Move l operands from absolute_long to address_indexed.
entry:
    rept {{instruction_count}}
    move.l $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.l $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.l $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.l $500000.l,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
