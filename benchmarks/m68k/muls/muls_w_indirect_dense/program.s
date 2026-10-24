; Multiply a fixed dense word through address indirect and restore D5.
operand_data:
    dc.w $6b5d
entry:
    rept {{instruction_count}}
    muls.w (a3),d5
    move.l d6,d5
    muls.w (a3),d5
    move.l d6,d5
    muls.w (a3),d5
    move.l d6,d5
    muls.w (a3),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
