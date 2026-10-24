; Transfer 1 l register(s) through stack_postincrement.
; An inert MOVE.L precedes each address-register restoration.
operand_data:
    dcb.w 200000,$1234
entry:
    rept {{instruction_count}}
    movem.l (a7)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l (a7)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l (a7)+,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l (a7)+,a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
