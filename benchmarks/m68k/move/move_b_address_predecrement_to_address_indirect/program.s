; Move b operands from address_predecrement to address_indirect.
source_data:
    dcb.w 50000,$5a3c
entry:
    rept {{instruction_count}}
    move.b -(a1),(a2)
    move.l d3,d4
    move.b -(a1),(a2)
    move.l d3,d4
    move.b -(a1),(a2)
    move.l d3,d4
    move.b -(a1),(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
