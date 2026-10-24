; Move w operands from data_register to address_displacement.
entry:
    rept {{instruction_count}}
    move.w d2,6(a2)
    move.l d3,d4
    move.w d2,6(a2)
    move.l d3,d4
    move.w d2,6(a2)
    move.l d3,d4
    move.w d2,6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
