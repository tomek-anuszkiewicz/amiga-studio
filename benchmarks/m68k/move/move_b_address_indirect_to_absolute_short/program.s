; Move b operands from address_indirect to absolute_short.
entry:
    rept {{instruction_count}}
    move.b (a1),$2000.w
    move.l d3,d4
    move.b (a1),$2000.w
    move.l d3,d4
    move.b (a1),$2000.w
    move.l d3,d4
    move.b (a1),$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
