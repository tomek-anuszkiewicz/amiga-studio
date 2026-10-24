; Measure AND.W D0,$200000.L in the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    and.w d0,$200000.l
    move.l d5,d1
    and.w d1,$200000.l
    move.l d6,d2
    and.w d2,$200000.l
    move.l d7,d3
    and.w d3,$200000.l
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
