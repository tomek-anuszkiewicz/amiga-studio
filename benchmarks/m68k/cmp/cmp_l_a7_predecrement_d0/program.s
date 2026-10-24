; Measure CMP.L -(A7),D0 without changing the destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmp.l -(a7),d0
    move.l d7,d1
    cmp.l -(a7),d0
    move.l d7,d1
    cmp.l -(a7),d0
    move.l d7,d1
    cmp.l -(a7),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
