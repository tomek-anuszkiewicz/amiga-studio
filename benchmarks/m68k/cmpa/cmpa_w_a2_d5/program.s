; Compare address_register source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.w a0,a6
    move.l d1,d5
    cmpa.w a1,a6
    move.l d2,d6
    cmpa.w a2,a6
    move.l d3,d7
    cmpa.w a3,a6
    move.l d0,d4
    endr
    dc.b "M68K_END_MARKER!"
