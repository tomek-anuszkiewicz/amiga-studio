; Move l operands from absolute_long to stack_postincrement.
destination_data:
    dcb.w 200000,$a55a
entry:
    rept {{instruction_count}}
    move.l $500000.l,(a7)+
    move.l d3,d4
    move.l $500000.l,(a7)+
    move.l d3,d4
    move.l $500000.l,(a7)+
    move.l d3,d4
    move.l $500000.l,(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
