; Compare address_postincrement source with A6; CMPA leaves A6 unchanged.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpa.l (a1)+,a6
    move.l d7,d0
    cmpa.l (a1)+,a6
    move.l d7,d0
    cmpa.l (a1)+,a6
    move.l d7,d0
    cmpa.l (a1)+,a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
