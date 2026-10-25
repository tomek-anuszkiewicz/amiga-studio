; Restore A7 after each ADDA target; source EA remains in range.
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
    adda.w 6(a1,d6.w),a7
    movea.l d7,a7
    adda.w 6(a1,d6.w),a7
    movea.l d7,a7
    adda.w 6(a1,d6.w),a7
    movea.l d7,a7
    adda.w 6(a1,d6.w),a7
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
