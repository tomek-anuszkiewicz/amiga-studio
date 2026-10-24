; Measure ADDQ.L #5 with this destination mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    addq.l #5,(a3)
    move.l d7,d0
    addq.l #5,(a3)
    move.l d7,d0
    addq.l #5,(a3)
    move.l d7,d0
    addq.l #5,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
