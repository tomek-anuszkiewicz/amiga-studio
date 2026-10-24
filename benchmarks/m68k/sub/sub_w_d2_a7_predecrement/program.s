; Measure SUB.W D2,-(A7) within the repeated workload.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.w d2,-(a7)
    move.l d7,d1
    sub.w d2,-(a7)
    move.l d7,d1
    sub.w d2,-(a7)
    move.l d7,d1
    sub.w d2,-(a7)
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
