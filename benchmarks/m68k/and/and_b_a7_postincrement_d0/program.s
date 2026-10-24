; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.b (a7)+,d0
    move.l d7,d0
    and.b (a7)+,d0
    move.l d7,d0
    and.b (a7)+,d0
    move.l d7,d0
    and.b (a7)+,d0
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
