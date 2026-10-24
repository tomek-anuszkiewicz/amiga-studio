; Move w operands from immediate to data_register.
entry:
    rept {{instruction_count}}
    move.w #$5a3c,d5
    move.l d3,d4
    move.w #$5a3c,d5
    move.l d3,d4
    move.w #$5a3c,d5
    move.l d3,d4
    move.w #$5a3c,d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
