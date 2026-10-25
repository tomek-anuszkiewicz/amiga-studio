; Each RTR reads a prepared frame and returns to its MOVE helper.
entry:
    rept {{instruction_count}}
    rtr
    move.l d6,d7
    rtr
    move.l d6,d7
    rtr
    move.l d6,d7
    rtr
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
