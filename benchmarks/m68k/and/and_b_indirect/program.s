; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    and.b (a3),d1
    move.l d7,d1
    and.b (a3),d1
    move.l d7,d1
    and.b (a3),d1
    move.l d7,d1
    and.b (a3),d1
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
