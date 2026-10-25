; STOP needs special handling here: the runner raises IPL 2 after each halt.
; The level-two autovector handler returns to the next STOP with RTE.
interrupt_handler:
    rte
entry:
    rept {{instruction_count}}
    stop #$2000
    stop #$2000
    stop #$2000
    stop #$2000
    endr
    dc.b "M68K_END_MARKER!"
