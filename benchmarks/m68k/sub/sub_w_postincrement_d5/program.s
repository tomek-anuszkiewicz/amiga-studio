; Read each source word once through postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.w (a3)+,d5
    move.l d7,d0
    sub.w (a3)+,d5
    move.l d7,d0
    sub.w (a3)+,d5
    move.l d7,d0
    sub.w (a3)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
