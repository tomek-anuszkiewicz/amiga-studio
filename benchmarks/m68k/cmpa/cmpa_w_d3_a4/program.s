; Compare a sign-extended register word with an address register.
entry:
    rept {{instruction_count}}
    cmpa.w d4,a0
    move.l d0,d1
    cmpa.w d5,a1
    move.l d0,d1
    cmpa.w d6,a2
    move.l d0,d1
    cmpa.w d7,a3
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
