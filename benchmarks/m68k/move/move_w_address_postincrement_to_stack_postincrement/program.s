; Move w operands from address_postincrement to stack_postincrement.
source_data:
    dcb.w 100000,$5a3c
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w (a1)+,(a7)+
    move.l d3,d4
    move.w (a1)+,(a7)+
    move.l d3,d4
    move.w (a1)+,(a7)+
    move.l d3,d4
    move.w (a1)+,(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
