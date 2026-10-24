; Move b operands from data_register to data_register.
entry:
    rept {{instruction_count}}
    move.b d2,d5
    move.l d3,d4
    move.b d2,d5
    move.l d3,d4
    move.b d2,d5
    move.l d3,d4
    move.b d2,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
