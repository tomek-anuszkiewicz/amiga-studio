; Transfer 1 w register(s) through address_postincrement.
; An inert MOVE.L precedes each address-register restoration.
operand_data:
    dcb.w 100000,$1234
entry:
    rept {{instruction_count}}
    movem.w (a3)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.w (a3)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.w (a3)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.w (a3)+,a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
