; Restore A7 after each SUBA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    suba.l d0,a7
    move.l d7,d1
    movea.l d7,a7
    suba.l d2,a7
    move.l d7,d1
    movea.l d7,a7
    suba.l d4,a7
    move.l d7,d1
    movea.l d7,a7
    suba.l d0,a7
    move.l d7,d1
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
