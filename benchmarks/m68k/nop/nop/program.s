; Measure the no-operation instruction in a pure stream.
entry:
    rept {{instruction_count}}
    nop
    move.l d7,d0
    nop
    move.l d7,d0
    nop
    move.l d7,d0
    nop
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
