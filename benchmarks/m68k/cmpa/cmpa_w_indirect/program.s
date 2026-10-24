; Compare address_indirect source with A6; CMPA leaves A6 unchanged.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    cmpa.w (a3),a6
    move.l d7,d0
    cmpa.w (a3),a6
    move.l d7,d0
    cmpa.w (a3),a6
    move.l d7,d0
    cmpa.w (a3),a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
