; Move b operands from immediate to data_register.
entry:
    rept {{instruction_count}}
    move.b #$5a,d5
    move.l d3,d4
    move.b #$5a,d5
    move.l d3,d4
    move.b #$5a,d5
    move.l d3,d4
    move.b #$5a,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
