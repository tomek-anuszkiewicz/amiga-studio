; Compare independent memory words without consuming the flags.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmp.w (a3)+,d5
    move.l d7,d0
    cmp.w (a3)+,d5
    move.l d7,d0
    cmp.w (a3)+,d5
    move.l d7,d0
    cmp.w (a3)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
