; Load A6 from the selected source EA, then restore it.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    movea.w #$1234,a6
    move.l d7,d0
    movea.l d7,a6
    movea.w #$1234,a6
    move.l d7,d0
    movea.l d7,a6
    movea.w #$1234,a6
    move.l d7,d0
    movea.l d7,a6
    movea.w #$1234,a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
