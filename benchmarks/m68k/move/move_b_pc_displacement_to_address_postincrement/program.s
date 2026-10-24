; Move b operands from pc_displacement to address_postincrement.
destination_data:
    dcb.w 50000,$a55a
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.b *-2(pc),(a2)+
    move.l d3,d4
    move.b *-2(pc),(a2)+
    move.l d3,d4
    move.b *-2(pc),(a2)+
    move.l d3,d4
    move.b *-2(pc),(a2)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
