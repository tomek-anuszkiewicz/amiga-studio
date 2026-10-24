; Subtract with extend through the register form.
entry:
    rept {{instruction_count}}
    subx.b d0,d1
    move.l d6,d7
    subx.b d2,d3
    move.l d6,d7
    subx.b d4,d5
    move.l d6,d7
    subx.b d0,d1
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
