; Compare immediate source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.l #$12345679,a6
    move.l d7,d1
    cmpa.l #$12345679,a6
    move.l d7,d1
    cmpa.l #$12345679,a6
    move.l d7,d1
    cmpa.l #$12345679,a6
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
