; Measure SUBQ.L #5 with this destination mode.
    dc.w $1357
    dc.w $2469
    dc.w $357B
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    subq.l #5,6(a3)
    move.l d7,d0
    subq.l #5,6(a3)
    move.l d7,d0
    subq.l #5,6(a3)
    move.l d7,d0
    subq.l #5,6(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
