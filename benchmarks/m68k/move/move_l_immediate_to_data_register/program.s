; Move l operands from immediate to data_register.
entry:
    rept {{instruction_count}}
    move.l #$5a3c2469,d5
    move.l d3,d4
    move.l #$5a3c2469,d5
    move.l d3,d4
    move.l #$5a3c2469,d5
    move.l d3,d4
    move.l #$5a3c2469,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
