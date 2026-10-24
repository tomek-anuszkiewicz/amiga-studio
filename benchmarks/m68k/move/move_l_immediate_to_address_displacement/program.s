; Move l operands from immediate to address_displacement.
entry:
    rept {{instruction_count}}
    move.l #$5a3c2469,6(a2)
    move.l d3,d4
    move.l #$5a3c2469,6(a2)
    move.l d3,d4
    move.l #$5a3c2469,6(a2)
    move.l d3,d4
    move.l #$5a3c2469,6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
