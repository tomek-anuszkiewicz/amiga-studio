; User-mode RESET traps; the handler advances the saved PC to its MOVE helper.
privilege_handler:
    addq.l #2,2(a7)
    rte
entry:
    rept {{instruction_count}}
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    reset
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
