; Measure EOR.L D0,$200000.L in the repeated workload.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    eor.l d0,$200000.l
    move.l d5,d1
    eor.l d1,$200000.l
    move.l d6,d2
    eor.l d2,$200000.l
    move.l d7,d3
    eor.l d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
