; Measure ORI.L immediate operand with this destination mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    ori.l #$13572469,$200000.l
    move.l d7,d0
    ori.l #$13572469,$200000.l
    move.l d7,d0
    ori.l #$13572469,$200000.l
    move.l d7,d0
    ori.l #$13572469,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
