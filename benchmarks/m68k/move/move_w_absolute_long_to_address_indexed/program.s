; Move w operands from absolute_long to address_indexed.
entry:
    rept {{instruction_count}}
    move.w $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.w $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.w $500000.l,6(a2,d1.w)
    move.l d3,d4
    move.w $500000.l,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
