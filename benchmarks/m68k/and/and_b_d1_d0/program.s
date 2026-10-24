; Reload register destinations from D7 so each logical target uses a fresh operand.
entry:
    rept {{instruction_count}}
    and.b d0,d1
    move.l d7,d1
    and.b d2,d3
    move.l d7,d3
    and.b d4,d5
    move.l d7,d5
    and.b d0,d1
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
