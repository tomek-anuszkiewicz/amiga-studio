; Measure SUB.L D0,$200000.L within the repeated workload.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    sub.l d0,$200000.l
    move.l d5,d1
    sub.l d1,$200000.l
    move.l d6,d2
    sub.l d2,$200000.l
    move.l d7,d3
    sub.l d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
