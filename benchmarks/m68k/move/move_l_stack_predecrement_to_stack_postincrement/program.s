; Move l through ordered supervisor A7 source and destination accesses.
entry:
    rept {{instruction_count}}
    move.l -(a7),(a7)+
    move.l d3,d4
    move.l -(a7),(a7)+
    move.l d3,d4
    move.l -(a7),(a7)+
    move.l d3,d4
    move.l -(a7),(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
