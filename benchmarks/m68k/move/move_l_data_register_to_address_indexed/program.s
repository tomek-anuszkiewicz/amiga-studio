; Move l operands from data_register to address_indexed.
entry:
    rept {{instruction_count}}
    move.l d2,6(a2,d1.w)
    move.l d3,d4
    move.l d2,6(a2,d1.w)
    move.l d3,d4
    move.l d2,6(a2,d1.w)
    move.l d3,d4
    move.l d2,6(a2,d1.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
