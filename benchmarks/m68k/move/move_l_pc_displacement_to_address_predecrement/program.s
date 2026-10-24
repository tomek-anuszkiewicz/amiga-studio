; Move l operands from pc_displacement to address_predecrement.
destination_data:
    dcb.w 200000,$a55a
    dc.w $fffa,$2803
entry:
    rept {{instruction_count}}
    move.l *-4(pc),-(a2)
    move.l d3,d4
    move.l *-4(pc),-(a2)
    move.l d3,d4
    move.l *-4(pc),-(a2)
    move.l d3,d4
    move.l *-4(pc),-(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
