; Reload register destinations from D7 so each logical target uses a fresh operand.
entry:
    rept {{instruction_count}}
    or.l d0,d1
    move.l d7,d1
    or.l d2,d3
    move.l d7,d3
    or.l d4,d5
    move.l d7,d5
    or.l d0,d1
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
