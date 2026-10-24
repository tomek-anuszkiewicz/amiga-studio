; Compare address_indirect source with A6; CMPA leaves A6 unchanged.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmpa.l (a3),a6
    move.l d7,d0
    cmpa.l (a3),a6
    move.l d7,d0
    cmpa.l (a3),a6
    move.l d7,d0
    cmpa.l (a3),a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
