; Restore A6 after each SUBA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    suba.w $200000.l,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $200000.l,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $200000.l,a6
    move.l d7,d0
    movea.l d7,a6
    suba.w $200000.l,a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
