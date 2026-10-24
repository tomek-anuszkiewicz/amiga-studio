; Measure SUB.B D0,$200000.L within the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    sub.b d0,$200000.l
    move.l d5,d1
    sub.b d1,$200000.l
    move.l d6,d2
    sub.b d2,$200000.l
    move.l d7,d3
    sub.b d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
