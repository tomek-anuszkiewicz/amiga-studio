; Move w operands from absolute_short to address_predecrement.
destination_data:
    dcb.w 100000,$a55a
entry:
    rept {{instruction_count}}
    move.w $1000.w,-(a2)
    move.l d3,d4
    move.w $1000.w,-(a2)
    move.l d3,d4
    move.w $1000.w,-(a2)
    move.l d3,d4
    move.w $1000.w,-(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
