; Four JMP (A1) targets, each landing on the next MOVEA setup.
entry:
    rept {{instruction_count}}
    movea.l #*+8,a1
    jmp (a1)
    movea.l #*+8,a1
    jmp (a1)
    movea.l #*+8,a1
    jmp (a1)
    movea.l #*+8,a1
    jmp (a1)
    endr
    dc.b "M68K_END_MARKER!"
