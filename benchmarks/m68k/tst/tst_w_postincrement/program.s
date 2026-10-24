; Test independent memory words without a conditional branch.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    tst.w (a3)+
    move.l d7,d0
    tst.w (a3)+
    move.l d7,d0
    tst.w (a3)+
    move.l d7,d0
    tst.w (a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
