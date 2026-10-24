; Move w operands from address_indexed to stack_postincrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w 6(a1,d6.w),(a7)+
    move.l d3,d4
    move.w 6(a1,d6.w),(a7)+
    move.l d3,d4
    move.w 6(a1,d6.w),(a7)+
    move.l d3,d4
    move.w 6(a1,d6.w),(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
