; Transfer 1 l register(s) through absolute_short.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    movem.l $1000.w,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l $1000.w,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l $1000.w,a7
    move.l d0,d1
    movea.l d7,a7
    movem.l $1000.w,a7
    move.l d0,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
