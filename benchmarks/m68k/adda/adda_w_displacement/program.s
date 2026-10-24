; Restore A6 after each ADDA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    adda.w 6(a3),a6
    move.l d7,d0
    movea.l d7,a6
    adda.w 6(a3),a6
    move.l d7,d0
    movea.l d7,a6
    adda.w 6(a3),a6
    move.l d7,d0
    movea.l d7,a6
    adda.w 6(a3),a6
    move.l d7,d0
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
