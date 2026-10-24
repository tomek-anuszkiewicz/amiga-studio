; Load A7 from the selected source EA, then restore it.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    movea.l d0,a7
    move.l d7,d1
    movea.l #$00a52000,a7
    movea.l d2,a7
    move.l d7,d1
    movea.l #$00a52000,a7
    movea.l d4,a7
    move.l d7,d1
    movea.l #$00a52000,a7
    movea.l d0,a7
    move.l d7,d1
    movea.l #$00a52000,a7
    endr
    dc.b "M68K_END_MARKER!"
