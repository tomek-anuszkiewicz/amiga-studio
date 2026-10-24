; Move l operands from pc_displacement to absolute_long.
    dc.w $fffa,$2803
entry:
    rept {{instruction_count}}
    move.l *-4(pc),$500100.l
    move.l d3,d4
    move.l *-4(pc),$500100.l
    move.l d3,d4
    move.l *-4(pc),$500100.l
    move.l d3,d4
    move.l *-4(pc),$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
