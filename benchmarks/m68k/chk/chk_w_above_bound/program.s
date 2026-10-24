; CHK traps on each target; the supervisor handler returns to its MOVE helper.
chk_handler:
    rte
entry:
    rept {{instruction_count}}
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    chk.w d3,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
