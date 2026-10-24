; Move w operands from pc_displacement to data_register.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-2(pc),d5
    move.l d3,d4
    move.w *-2(pc),d5
    move.l d3,d4
    move.w *-2(pc),d5
    move.l d3,d4
    move.w *-2(pc),d5
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
