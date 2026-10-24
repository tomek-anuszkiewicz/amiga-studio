; Multiply four independent memory words by register-held destination words.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    muls.w (a3)+,d2
    move.l d4,d2
    muls.w (a3)+,d2
    move.l d5,d2
    muls.w (a3)+,d2
    move.l d6,d2
    muls.w (a3)+,d2
    move.l d7,d2
    endr
    dc.b "M68K_END_MARKER!"
