; Compare immediate source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.w #$1234,a6
    move.l d7,d1
    cmpa.w #$1234,a6
    move.l d7,d1
    cmpa.w #$1234,a6
    move.l d7,d1
    cmpa.w #$1234,a6
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
