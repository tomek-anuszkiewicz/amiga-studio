; Compare address_displacement source with A6; CMPA leaves A6 unchanged.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmpa.l 6(a3),a6
    move.l d7,d0
    cmpa.l 6(a3),a6
    move.l d7,d0
    cmpa.l 6(a3),a6
    move.l d7,d0
    cmpa.l 6(a3),a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
