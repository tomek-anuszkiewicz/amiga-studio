; Transfer 1 l register(s) through address_postincrement.
operand_data:
    dcb.w 200000,$1234
entry:
    rept {{instruction_count}}
    movem.l (a3)+,a7
    movea.l d7,a7
    movem.l (a3)+,a7
    movea.l d7,a7
    movem.l (a3)+,a7
    movea.l d7,a7
    movem.l (a3)+,a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
