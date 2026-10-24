; Consume varied source words so the result reaches its absorbing value.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    or.w (a4)+,d6
    move.l d7,d0
    or.w (a4)+,d6
    move.l d7,d0
    or.w (a4)+,d6
    move.l d7,d0
    or.w (a4)+,d6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
