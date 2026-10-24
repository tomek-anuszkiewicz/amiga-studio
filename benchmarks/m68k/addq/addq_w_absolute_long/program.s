; Measure ADDQ.W #5 with this destination mode.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    addq.w #5,$200000.l
    move.l d7,d0
    addq.w #5,$200000.l
    move.l d7,d0
    addq.w #5,$200000.l
    move.l d7,d0
    addq.w #5,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
