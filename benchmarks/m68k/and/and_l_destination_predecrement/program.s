; Measure AND.L D0,-(A0) in the repeated workload.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.l d0,-(a0)
    move.l d5,d1
    and.l d1,-(a1)
    move.l d6,d2
    and.l d2,-(a2)
    move.l d7,d3
    and.l d3,-(a3)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
