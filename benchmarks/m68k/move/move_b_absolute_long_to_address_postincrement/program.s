; Move b operands from absolute_long to address_postincrement.
destination_data:
    dcb.w 50000,$a55a
entry:
    rept {{instruction_count}}
    move.b $500000.l,(a2)+
    move.l d3,d4
    move.b $500000.l,(a2)+
    move.l d3,d4
    move.b $500000.l,(a2)+
    move.l d3,d4
    move.b $500000.l,(a2)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
