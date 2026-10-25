; User-mode TRAP switches to SSP; RTE restores user mode before the next target.
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
