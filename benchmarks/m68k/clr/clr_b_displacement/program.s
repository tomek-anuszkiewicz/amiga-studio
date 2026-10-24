; Measure CLR.B with the selected destination mode.
    dc.w $1357
    dc.w $2469
    dc.w $357B
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    clr.b 6(a3)
    move.l d7,d0
    clr.b 6(a3)
    move.l d7,d0
    clr.b 6(a3)
    move.l d7,d0
    clr.b 6(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
