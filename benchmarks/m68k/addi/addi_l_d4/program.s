; Measure ADDI.L immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    addi.l #$13572469,d4
    move.l d7,d0
    addi.l #$13572469,d4
    move.l d7,d0
    addi.l #$13572469,d4
    move.l d7,d0
    addi.l #$13572469,d4
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
