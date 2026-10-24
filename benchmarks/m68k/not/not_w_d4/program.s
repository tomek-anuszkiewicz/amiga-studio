; Invert a register word repeatedly.
entry:
    rept {{instruction_count}}
    not.w d4
    move.l d7,d0
    not.w d4
    move.l d7,d0
    not.w d4
    move.l d7,d0
    not.w d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
