; Four JMP (0,A5,D2.W) targets, each landing on the next MOVEA setup.
entry:
    rept {{instruction_count}}
    movea.l #*+10,a5
    jmp (0,a5,d2.w)
    movea.l #*+10,a5
    jmp (0,a5,d2.w)
    movea.l #*+10,a5
    jmp (0,a5,d2.w)
    movea.l #*+10,a5
    jmp (0,a5,d2.w)
    endr
    dc.b "M68K_END_MARKER!"
