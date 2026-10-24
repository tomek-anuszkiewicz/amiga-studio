; Move w operands from pc_indexed to address_predecrement.
destination_data:
    dcb.w 100000,$a55a
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-10(pc,d6.w),-(a2)
    move.l d3,d4
    move.w *-10(pc,d6.w),-(a2)
    move.l d3,d4
    move.w *-10(pc,d6.w),-(a2)
    move.l d3,d4
    move.w *-10(pc,d6.w),-(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
