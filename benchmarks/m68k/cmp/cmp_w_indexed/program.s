; Measure CMP.W 6(A1,D6.W),D0 without changing the destination.
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
    cmp.w 6(a1,d6.w),d0
    move.l d7,d1
    cmp.w 6(a1,d6.w),d0
    move.l d7,d1
    cmp.w 6(a1,d6.w),d0
    move.l d7,d1
    cmp.w 6(a1,d6.w),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
