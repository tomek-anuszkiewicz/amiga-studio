; Move l operands from address_displacement to data_register.
entry:
    rept {{instruction_count}}
    move.l 6(a1),d5
    move.l d3,d4
    move.l 6(a1),d5
    move.l d3,d4
    move.l 6(a1),d5
    move.l d3,d4
    move.l 6(a1),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
