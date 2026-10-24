; Move l operands from address_displacement to stack_predecrement.
destination_data:
    dcb.w 200000,$a55a
entry:
    rept {{instruction_count}}
    move.l 6(a1),-(a7)
    move.l d3,d4
    move.l 6(a1),-(a7)
    move.l d3,d4
    move.l 6(a1),-(a7)
    move.l d3,d4
    move.l 6(a1),-(a7)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
