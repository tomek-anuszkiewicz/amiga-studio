; Compare stack_predecrement source with A6; CMPA leaves A6 unchanged.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpa.l -(a7),a6
    move.l d7,d1
    cmpa.l -(a7),a6
    move.l d7,d1
    cmpa.l -(a7),a6
    move.l d7,d1
    cmpa.l -(a7),a6
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
