; Move l operands from address_register to address_displacement.
entry:
    rept {{instruction_count}}
    move.l a0,6(a2)
    move.l d3,d4
    move.l a0,6(a2)
    move.l d3,d4
    move.l a0,6(a2)
    move.l d3,d4
    move.l a0,6(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
