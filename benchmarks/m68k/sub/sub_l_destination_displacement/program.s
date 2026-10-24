; Measure SUB.L D0,6(A5) within the repeated workload.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    sub.l d0,6(a5)
    move.l d5,d1
    sub.l d1,6(a5)
    move.l d6,d2
    sub.l d2,6(a5)
    move.l d7,d3
    sub.l d3,6(a5)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
