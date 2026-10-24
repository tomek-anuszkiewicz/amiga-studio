; Measure NEG.L with the selected destination mode.
operand_data:
    dc.l $3457689B
entry:
    rept {{instruction_count}}
    neg.l $200000.l
    move.l d7,d0
    neg.l $200000.l
    move.l d7,d0
    neg.l $200000.l
    move.l d7,d0
    neg.l $200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
