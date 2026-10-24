; Odd MOVEM destination takes Vector 3; the handler discards its extra frame words.
address_error_handler:
    addq.l #8,a7
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    movem.w d2,(a3)
    move.l d7,d0
    movem.w d2,(a3)
    move.l d7,d0
    movem.w d2,(a3)
    move.l d7,d0
    movem.w d2,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
