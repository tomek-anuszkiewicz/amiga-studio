; Four TRAP #3 targets, each followed by the handler's RTE.
trap_handler:
    rte
entry:
    rept {{instruction_count}}
    trap #3
    trap #3
    trap #3
    trap #3
    endr
    dc.b "M68K_END_MARKER!"
