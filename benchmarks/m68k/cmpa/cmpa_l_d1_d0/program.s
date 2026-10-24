; Compare data_register source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.l d0,a6
    move.l d6,d7
    cmpa.l d2,a6
    move.l d6,d7
    cmpa.l d4,a6
    move.l d6,d7
    cmpa.l d0,a6
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
