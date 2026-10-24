; Measure OR.B D0,$200000.L in the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    or.b d0,$200000.l
    move.l d5,d1
    or.b d1,$200000.l
    move.l d6,d2
    or.b d2,$200000.l
    move.l d7,d3
    or.b d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
