; Move w operands from immediate to absolute_long.
entry:
    rept {{instruction_count}}
    move.w #$5a3c,$500100.l
    move.l d3,d4
    move.w #$5a3c,$500100.l
    move.l d3,d4
    move.w #$5a3c,$500100.l
    move.l d3,d4
    move.w #$5a3c,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
