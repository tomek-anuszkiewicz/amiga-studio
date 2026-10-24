; Transfer 3 w register(s) through address_postincrement.
operand_data:
    dcb.w 300000,$1234
entry:
    rept {{instruction_count}}
    movem.w (a3)+,d2/d5/a5
    move.l d7,d0
    movem.w (a3)+,d2/d5/a5
    move.l d7,d0
    movem.w (a3)+,d2/d5/a5
    move.l d7,d0
    movem.w (a3)+,d2/d5/a5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
