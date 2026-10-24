; Odd word destination takes Vector 3 before the postincrement.
address_error_handler:
    addq.l #8,a7
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
