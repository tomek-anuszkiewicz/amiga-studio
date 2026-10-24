; Move b operands from pc_displacement to absolute_short.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.b *-2(pc),$2000.w
    move.l d3,d4
    move.b *-2(pc),$2000.w
    move.l d3,d4
    move.b *-2(pc),$2000.w
    move.l d3,d4
    move.b *-2(pc),$2000.w
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
