; Sign-extend a register word into an address register.
entry:
    rept {{instruction_count}}
    movea.w d4,a0
    move.l d0,d1
    movea.w d5,a1
    move.l d0,d1
    movea.w d6,a2
    move.l d0,d1
    movea.w d7,a3
    move.l d0,d1
    endr
    dc.b "M68K_END_MARKER!"
