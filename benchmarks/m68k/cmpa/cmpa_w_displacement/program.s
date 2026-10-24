; Compare address_displacement source with A6; CMPA leaves A6 unchanged.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    cmpa.w 6(a3),a6
    move.l d7,d0
    cmpa.w 6(a3),a6
    move.l d7,d0
    cmpa.w 6(a3),a6
    move.l d7,d0
    cmpa.w 6(a3),a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
