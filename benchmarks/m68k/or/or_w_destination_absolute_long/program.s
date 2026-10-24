; Measure OR.W D0,$200000.L in the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    or.w d0,$200000.l
    move.l d5,d1
    or.w d1,$200000.l
    move.l d6,d2
    or.w d2,$200000.l
    move.l d7,d3
    or.w d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
