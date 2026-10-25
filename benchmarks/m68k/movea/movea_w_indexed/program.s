; Load A6 from the selected source EA, then restore it.
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
    movea.w 6(a1,d6.w),a6
    movea.l d7,a6
    movea.w 6(a1,d6.w),a6
    movea.l d7,a6
    movea.w 6(a1,d6.w),a6
    movea.l d7,a6
    movea.w 6(a1,d6.w),a6
    movea.l d7,a6
    endr
    dc.b "M68K_END_MARKER!"
