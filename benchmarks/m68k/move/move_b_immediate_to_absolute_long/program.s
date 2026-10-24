; Move b operands from immediate to absolute_long.
entry:
    rept {{instruction_count}}
    move.b #$5a,$500100.l
    move.l d3,d4
    move.b #$5a,$500100.l
    move.l d3,d4
    move.b #$5a,$500100.l
    move.l d3,d4
    move.b #$5a,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
