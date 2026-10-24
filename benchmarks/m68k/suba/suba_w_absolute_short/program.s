; Restore A6 after each SUBA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
entry:
    rept {{instruction_count}}
    suba.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $1000.w,a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
