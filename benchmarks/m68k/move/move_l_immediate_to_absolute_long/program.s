; Move l operands from immediate to absolute_long.
entry:
    rept {{instruction_count}}
    move.l #$5a3c2469,$500100.l
    move.l d3,d4
    move.l #$5a3c2469,$500100.l
    move.l d3,d4
    move.l #$5a3c2469,$500100.l
    move.l d3,d4
    move.l #$5a3c2469,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
