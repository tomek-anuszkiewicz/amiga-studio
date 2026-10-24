; Move w operands from absolute_long to stack_postincrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w $500000.l,(a7)+
    move.l d3,d4
    move.w $500000.l,(a7)+
    move.l d3,d4
    move.w $500000.l,(a7)+
    move.l d3,d4
    move.w $500000.l,(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
