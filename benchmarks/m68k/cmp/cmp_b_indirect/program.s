; Measure CMP.B (A3),D1 without changing the destination.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    cmp.b (a3),d1
    move.l d7,d0
    cmp.b (a3),d1
    move.l d7,d0
    cmp.b (a3),d1
    move.l d7,d0
    cmp.b (a3),d1
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
