; Measure OR.W D2,6(A3,D5.W) in the repeated workload.
    dc.w $1357
    dc.w $246B
    dc.w $369D
    dc.w $48AF
    dc.w $5AC1
    dc.w $6BD3
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    or.w d2,6(a3,d5.w)
    move.l d7,d0
    or.w d2,6(a3,d5.w)
    move.l d7,d0
    or.w d2,6(a3,d5.w)
    move.l d7,d0
    or.w d2,6(a3,d5.w)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
