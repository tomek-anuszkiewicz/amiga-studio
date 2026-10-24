; Restore A7 after each ADDA target; source EA remains in range.
; An inert MOVE.L precedes each address-register restoration.
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
    adda.l 6(a1,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    adda.l 6(a1,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    adda.l 6(a1,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    adda.l 6(a1,d6.w),a7
    move.l d7,d0
    movea.l d7,a7
    endr
    dc.b "M68K_END_MARKER!"
