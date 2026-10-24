; Measure CMP.W -(A6),D0 without changing the destination.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmp.w -(a6),d0
    move.l d7,d1
    cmp.w -(a6),d0
    move.l d7,d1
    cmp.w -(a6),d0
    move.l d7,d1
    cmp.w -(a6),d0
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
