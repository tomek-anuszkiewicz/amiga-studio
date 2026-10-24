; Move w operands from address_indirect to absolute_long.
entry:
    rept {{instruction_count}}
    move.w (a1),$500100.l
    move.l d3,d4
    move.w (a1),$500100.l
    move.l d3,d4
    move.w (a1),$500100.l
    move.l d3,d4
    move.w (a1),$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
