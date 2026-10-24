; Move w operands from address_indirect to address_displacement.
entry:
    rept {{instruction_count}}
    move.w (a1),6(a2)
    move.l d3,d4
    move.w (a1),6(a2)
    move.l d3,d4
    move.w (a1),6(a2)
    move.l d3,d4
    move.w (a1),6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
