; Measure OR.W D0,6(A5) in the repeated workload.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    or.w d0,6(a5)
    move.l d5,d1
    or.w d1,6(a5)
    move.l d6,d2
    or.w d2,6(a5)
    move.l d7,d3
    or.w d3,6(a5)
    move.l d4,d0
    endr
    dc.b "M68K_END_MARKER!"
