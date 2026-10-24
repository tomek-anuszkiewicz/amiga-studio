; Multiply a fixed dense word from absolute long Fast RAM.
operand_data:
    dc.w $6b5d
entry:
    rept {{instruction_count}}
    muls.w $200000.l,d5
    move.l d6,d5
    muls.w $200000.l,d5
    move.l d6,d5
    muls.w $200000.l,d5
    move.l d6,d5
    muls.w $200000.l,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
