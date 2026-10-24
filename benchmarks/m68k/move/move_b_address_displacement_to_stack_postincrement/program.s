; Move b operands from address_displacement to stack_postincrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.b 6(a1),(a7)+
    move.l d3,d4
    move.b 6(a1),(a7)+
    move.l d3,d4
    move.b 6(a1),(a7)+
    move.l d3,d4
    move.b 6(a1),(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
