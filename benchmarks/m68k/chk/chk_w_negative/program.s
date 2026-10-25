; CHK traps on each target; RTE returns to the next target.
chk_handler:
    rte
entry:
    rept {{instruction_count}}
    chk.w d3,d4
    chk.w d3,d4
    chk.w d3,d4
    chk.w d3,d4
    endr
    dc.b "M68K_END_MARKER!"
