; Move w operands from pc_displacement to stack_postincrement.
destination_data:
    dcb.w 100000,$a55a
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-2(pc),(a7)+
    move.l d3,d4
    move.w *-2(pc),(a7)+
    move.l d3,d4
    move.w *-2(pc),(a7)+
    move.l d3,d4
    move.w *-2(pc),(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
