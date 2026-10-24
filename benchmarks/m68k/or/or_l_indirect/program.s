; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    or.l (a3),d1
    move.l d7,d1
    or.l (a3),d1
    move.l d7,d1
    or.l (a3),d1
    move.l d7,d1
    or.l (a3),d1
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
