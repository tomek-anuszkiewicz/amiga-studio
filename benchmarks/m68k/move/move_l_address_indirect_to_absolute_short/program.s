; Move l operands from address_indirect to absolute_short.
entry:
    rept {{instruction_count}}
    move.l (a1),$2000.w
    move.l d3,d4
    move.l (a1),$2000.w
    move.l d3,d4
    move.l (a1),$2000.w
    move.l d3,d4
    move.l (a1),$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
