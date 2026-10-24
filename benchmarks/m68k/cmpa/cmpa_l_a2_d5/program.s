; Compare address_register source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.l a0,a6
    move.l d1,d5
    cmpa.l a1,a6
    move.l d2,d6
    cmpa.l a2,a6
    move.l d3,d7
    cmpa.l a3,a6
    move.l d0,d4
    endr
    dc.b "M68K_END_MARKER!"
