; Measure SUBQ.L #5 with this destination mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    subq.l #5,$200000.l
    move.l d7,d0
    subq.l #5,$200000.l
    move.l d7,d0
    subq.l #5,$200000.l
    move.l d7,d0
    subq.l #5,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
