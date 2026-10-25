; User-mode RTE traps before reading the exception frame.
privilege_handler:
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    rte
    rte
    rte
    rte
    endr
    dc.b "M68K_END_MARKER!"
