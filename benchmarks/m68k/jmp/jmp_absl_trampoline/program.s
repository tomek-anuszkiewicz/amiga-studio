; Four JMP $380000.l targets return through an RTS trampoline.
entry:
    rept {{instruction_count}}
    jmp $380000.l
    jmp $380000.l
    jmp $380000.l
    jmp $380000.l
    endr
    dc.b "M68K_END_MARKER!"
