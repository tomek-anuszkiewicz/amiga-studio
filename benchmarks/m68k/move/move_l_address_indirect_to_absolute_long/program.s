; Move l operands from address_indirect to absolute_long.
entry:
    rept {{instruction_count}}
    move.l (a1),$500100.l
    move.l d3,d4
    move.l (a1),$500100.l
    move.l d3,d4
    move.l (a1),$500100.l
    move.l d3,d4
    move.l (a1),$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
