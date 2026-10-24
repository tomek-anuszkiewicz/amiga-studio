; Compare address_indexed source with A6; CMPA leaves A6 unchanged.
    dc.w $1357
    dc.w $246B
    dc.w $369D
    dc.w $48AF
    dc.w $5AC1
    dc.w $6BD3
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    cmpa.l 6(a1,d6.w),a6
    move.l d7,d1
    cmpa.l 6(a1,d6.w),a6
    move.l d7,d1
    cmpa.l 6(a1,d6.w),a6
    move.l d7,d1
    cmpa.l 6(a1,d6.w),a6
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
