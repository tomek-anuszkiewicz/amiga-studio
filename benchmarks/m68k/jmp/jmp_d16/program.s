; Four JMP 6(A3) targets, each landing on the next MOVEA setup.
entry:
    rept {{instruction_count}}
    movea.l #*+4,a3
    jmp 6(a3)
    movea.l #*+4,a3
    jmp 6(a3)
    movea.l #*+4,a3
    jmp 6(a3)
    movea.l #*+4,a3
    jmp 6(a3)
    endr
    dc.b "M68K_END_MARKER!"
