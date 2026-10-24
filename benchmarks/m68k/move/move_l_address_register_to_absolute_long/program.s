; Move l operands from address_register to absolute_long.
entry:
    rept {{instruction_count}}
    move.l a0,$500100.l
    move.l d3,d4
    move.l a0,$500100.l
    move.l d3,d4
    move.l a0,$500100.l
    move.l d3,d4
    move.l a0,$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
