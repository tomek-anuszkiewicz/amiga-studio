; Restore A3 after each address-register quick operation.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    addq.l #5,a3
    move.l d0,d1
    movea.l d7,a3
    addq.l #5,a3
    move.l d0,d1
    movea.l d7,a3
    addq.l #5,a3
    move.l d0,d1
    movea.l d7,a3
    addq.l #5,a3
    move.l d0,d1
    movea.l d7,a3
    endr
    dc.b "M68K_END_MARKER!"
