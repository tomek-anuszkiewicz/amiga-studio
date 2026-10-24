; Move w operands from address_displacement to address_predecrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w 6(a1),-(a2)
    move.l d3,d4
    move.w 6(a1),-(a2)
    move.l d3,d4
    move.w 6(a1),-(a2)
    move.l d3,d4
    move.w 6(a1),-(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
