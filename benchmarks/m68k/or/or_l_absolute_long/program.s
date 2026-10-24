; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    or.l $200000.l,d2
    move.l d7,d2
    or.l $200000.l,d2
    move.l d7,d2
    or.l $200000.l,d2
    move.l d7,d2
    or.l $200000.l,d2
    move.l d7,d2
    endr
    dc.b "M68K_END_MARKER!"
