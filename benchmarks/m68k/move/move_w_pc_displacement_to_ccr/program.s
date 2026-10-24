; Move the selected source word to CCR in supervisor mode.
    dc.w $2803
entry:
    rept {{instruction_count}}
    move.w *-2(pc),ccr
    move.l d3,d4
    move.w *-2(pc),ccr
    move.l d3,d4
    move.w *-2(pc),ccr
    move.l d3,d4
    move.w *-2(pc),ccr
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
