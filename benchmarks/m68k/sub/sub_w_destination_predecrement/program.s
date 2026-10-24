; Measure SUB.W D0,-(A0) within the repeated workload.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.w d0,-(a0)
    move.l d5,d1
    sub.w d1,-(a1)
    move.l d6,d2
    sub.w d2,-(a2)
    move.l d7,d3
    sub.w d3,-(a3)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
