; Reload register destinations from D7 so each logical target uses a fresh operand.
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
    or.w 6(a1,d6.w),d0
    move.l d7,d0
    or.w 6(a1,d6.w),d0
    move.l d7,d0
    or.w 6(a1,d6.w),d0
    move.l d7,d0
    or.w 6(a1,d6.w),d0
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
