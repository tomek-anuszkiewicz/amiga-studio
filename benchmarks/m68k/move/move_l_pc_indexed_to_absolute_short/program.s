; Move l operands from pc_indexed to absolute_short.
    dc.w $fffa,$2803
entry:
    rept {{instruction_count}}
    move.l *-12(pc,d6.w),$2000.w
    move.l d3,d4
    move.l *-12(pc,d6.w),$2000.w
    move.l d3,d4
    move.l *-12(pc,d6.w),$2000.w
    move.l d3,d4
    move.l *-12(pc,d6.w),$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
