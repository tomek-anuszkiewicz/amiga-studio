; Consume varied source words so the result reaches its absorbing value.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.w (a5)+,d0
    move.l d7,d1
    and.w (a5)+,d0
    move.l d7,d1
    and.w (a5)+,d0
    move.l d7,d1
    and.w (a5)+,d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
