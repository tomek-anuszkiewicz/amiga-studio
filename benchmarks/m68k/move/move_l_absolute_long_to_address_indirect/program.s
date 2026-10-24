; Move l operands from absolute_long to address_indirect.
entry:
    rept {{instruction_count}}
    move.l $500000.l,(a2)
    move.l d3,d4
    move.l $500000.l,(a2)
    move.l d3,d4
    move.l $500000.l,(a2)
    move.l d3,d4
    move.l $500000.l,(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
