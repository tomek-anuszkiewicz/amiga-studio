; Move w operands from pc_displacement to address_indirect.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-2(pc),(a2)
    move.l d3,d4
    move.w *-2(pc),(a2)
    move.l d3,d4
    move.w *-2(pc),(a2)
    move.l d3,d4
    move.w *-2(pc),(a2)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
