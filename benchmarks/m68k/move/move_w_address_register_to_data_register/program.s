; Move w operands from address_register to data_register.
entry:
    rept {{instruction_count}}
    move.w a0,d5
    move.l d3,d4
    move.w a0,d5
    move.l d3,d4
    move.w a0,d5
    move.l d3,d4
    move.w a0,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
