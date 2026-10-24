; Measure CMP.L (A1)+,D5 without changing the destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmp.l (a1)+,d5
    move.l d7,d0
    cmp.l (a1)+,d5
    move.l d7,d0
    cmp.l (a1)+,d5
    move.l d7,d0
    cmp.l (a1)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
