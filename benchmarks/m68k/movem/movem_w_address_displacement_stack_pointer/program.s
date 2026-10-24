; Transfer 1 w register(s) through address_displacement.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    movem.w 6(a3),a7
    move.l d0,d1
    movea.l d7,a7
    movem.w 6(a3),a7
    move.l d0,d1
    movea.l d7,a7
    movem.w 6(a3),a7
    move.l d0,d1
    movea.l d7,a7
    movem.w 6(a3),a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
