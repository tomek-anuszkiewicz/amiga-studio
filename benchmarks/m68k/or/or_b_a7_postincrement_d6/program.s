; Walk initialized word data forward with user A7 byte postincrement.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    or.b (a7)+,d6
    move.l d7,d0
    or.b (a7)+,d6
    move.l d7,d0
    or.b (a7)+,d6
    move.l d7,d0
    or.b (a7)+,d6
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
