; Restore each destination word before adding a stable address-register word.
entry:
    rept {{instruction_count}}
    sub.w a0,d4
    move.l d1,d5
    sub.w a1,d5
    move.l d2,d6
    sub.w a2,d6
    move.l d3,d7
    sub.w a3,d7
    move.l d0,d4
    endr
    dc.b "M68K_END_MARKER!"
