; The supervisor handler skips each faulting signed division through SSP.
divide_by_zero_handler:
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    divs.w d3,d1
    move.l d7,d0
    divs.w d3,d1
    move.l d7,d0
    divs.w d3,d1
    move.l d7,d0
    divs.w d3,d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
