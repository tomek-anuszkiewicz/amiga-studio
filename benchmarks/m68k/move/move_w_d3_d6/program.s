; MOVE preserves its source, so no guest reset instruction is needed.
entry:
    rept {{instruction_count}}
    move.w d0,d1
    move.l d7,d6
    move.w d2,d3
    move.l d7,d6
    move.w d4,d5
    move.l d7,d6
    move.w d0,d1
    move.l d7,d6
    endr
    dc.b "M68K_END_MARKER!"
