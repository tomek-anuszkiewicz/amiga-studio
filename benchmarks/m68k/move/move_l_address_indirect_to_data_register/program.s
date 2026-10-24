; Move l operands from address_indirect to data_register.
entry:
    rept {{instruction_count}}
    move.l (a1),d5
    move.l d3,d4
    move.l (a1),d5
    move.l d3,d4
    move.l (a1),d5
    move.l d3,d4
    move.l (a1),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
