; Move b through ordered supervisor A7 source and destination accesses.
stack_data:
    dcb.w 200000,$5a3c
entry:
    rept {{instruction_count}}
    move.b (a7)+,(a7)+
    move.l d3,d4
    move.b (a7)+,(a7)+
    move.l d3,d4
    move.b (a7)+,(a7)+
    move.l d3,d4
    move.b (a7)+,(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
