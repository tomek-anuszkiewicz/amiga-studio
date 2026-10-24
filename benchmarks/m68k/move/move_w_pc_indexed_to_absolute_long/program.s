; Move w operands from pc_indexed to absolute_long.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-10(pc,d6.w),$500100.l
    move.l d3,d4
    move.w *-10(pc,d6.w),$500100.l
    move.l d3,d4
    move.w *-10(pc,d6.w),$500100.l
    move.l d3,d4
    move.w *-10(pc,d6.w),$500100.l
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
