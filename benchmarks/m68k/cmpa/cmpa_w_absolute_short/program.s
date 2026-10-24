; Compare absolute_short source with A6; CMPA leaves A6 unchanged.
entry:
    rept {{instruction_count}}
    cmpa.w $1000.w,a6
    move.l d7,d0
    cmpa.w $1000.w,a6
    move.l d7,d0
    cmpa.w $1000.w,a6
    move.l d7,d0
    cmpa.w $1000.w,a6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
