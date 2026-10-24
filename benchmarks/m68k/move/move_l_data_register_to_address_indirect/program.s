; Move l operands from data_register to address_indirect.
entry:
    rept {{instruction_count}}
    move.l d2,(a2)
    move.l d3,d4
    move.l d2,(a2)
    move.l d3,d4
    move.l d2,(a2)
    move.l d3,d4
    move.l d2,(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
