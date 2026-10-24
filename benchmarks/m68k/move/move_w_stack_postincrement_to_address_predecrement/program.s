; Move w operands from stack_postincrement to address_predecrement.
source_data:
    dcb.w 100000,$5a3c
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w (a7)+,-(a2)
    move.l d3,d4
    move.w (a7)+,-(a2)
    move.l d3,d4
    move.w (a7)+,-(a2)
    move.l d3,d4
    move.w (a7)+,-(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
