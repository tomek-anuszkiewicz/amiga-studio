; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.l (a1)+,d5
    move.l d7,d5
    and.l (a1)+,d5
    move.l d7,d5
    and.l (a1)+,d5
    move.l d7,d5
    and.l (a1)+,d5
    move.l d7,d5
    endr
    dc.b "M68K_END_MARKER!"
