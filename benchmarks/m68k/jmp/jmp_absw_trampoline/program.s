; Four JMP $1000.w targets return through an RTS trampoline.
entry:
    rept {{instruction_count}}
    jmp $1000.w
    jmp $1000.w
    jmp $1000.w
    jmp $1000.w
    endr
    dc.b "M68K_END_MARKER!"
