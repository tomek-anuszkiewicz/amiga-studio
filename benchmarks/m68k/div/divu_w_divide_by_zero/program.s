; The supervisor handler uses SSP through A7 to skip each faulting DIVU target.
divide_by_zero_handler:
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    divu.w d3,d1
    move.l d7,d0
    divu.w d3,d1
    move.l d7,d0
    divu.w d3,d1
    move.l d7,d0
    divu.w d3,d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
