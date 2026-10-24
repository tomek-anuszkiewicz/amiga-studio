; Measure ADDQ.B #5 with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    addq.b #5,(a3)
    move.l d7,d0
    addq.b #5,(a3)
    move.l d7,d0
    addq.b #5,(a3)
    move.l d7,d0
    addq.b #5,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
