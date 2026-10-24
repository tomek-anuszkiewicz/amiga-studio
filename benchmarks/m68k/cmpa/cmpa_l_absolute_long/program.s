; Compare absolute_long source with A6; CMPA leaves A6 unchanged.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmpa.l $200000.l,a6
    move.l d3,d4
    cmpa.l $200000.l,a6
    move.l d3,d4
    cmpa.l $200000.l,a6
    move.l d3,d4
    cmpa.l $200000.l,a6
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
