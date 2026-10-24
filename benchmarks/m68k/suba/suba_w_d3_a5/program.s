; Subtract sign-extended register words from independent address registers.
entry:
    rept {{instruction_count}}
    suba.w d4,a0
    move.l d0,d1
    suba.w d5,a1
    move.l d0,d1
    suba.w d6,a2
    move.l d0,d1
    suba.w d7,a3
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
