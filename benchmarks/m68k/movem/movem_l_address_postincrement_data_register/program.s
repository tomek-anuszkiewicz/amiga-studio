; Transfer 1 l register(s) through address_postincrement.
operand_data:
    dcb.w 200000,$1234
entry:
    rept {{instruction_count}}
    movem.l (a3)+,d2
    move.l d7,d0
    movem.l (a3)+,d2
    move.l d7,d0
    movem.l (a3)+,d2
    move.l d7,d0
    movem.l (a3)+,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
