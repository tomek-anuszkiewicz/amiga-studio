; Each RTE reads a prepared frame and returns to its MOVE helper.
entry:
    rept {{instruction_count}}
    rte
    move.l d6,d7
    rte
    move.l d6,d7
    rte
    move.l d6,d7
    rte
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
