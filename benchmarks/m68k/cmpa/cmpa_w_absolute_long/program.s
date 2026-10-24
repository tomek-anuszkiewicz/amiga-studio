; Compare absolute_long source with A6; CMPA leaves A6 unchanged.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    cmpa.w $200000.l,a6
    move.l d3,d4
    cmpa.w $200000.l,a6
    move.l d3,d4
    cmpa.w $200000.l,a6
    move.l d3,d4
    cmpa.w $200000.l,a6
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
