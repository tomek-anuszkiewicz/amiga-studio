; Restore A7 after each ADDA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    adda.l $200000.l,a7
    move.l d7,d0
    movea.l d7,a7
    adda.l $200000.l,a7
    move.l d7,d0
    movea.l d7,a7
    adda.l $200000.l,a7
    move.l d7,d0
    movea.l d7,a7
    adda.l $200000.l,a7
    move.l d7,d0
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
