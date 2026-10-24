; Reload register destinations from D7 so each logical target uses a fresh operand.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    and.w $200000.l,d2
    move.l d7,d2
    and.w $200000.l,d2
    move.l d7,d2
    and.w $200000.l,d2
    move.l d7,d2
    and.w $200000.l,d2
    move.l d7,d2
    endr
    dc.b "M68K_END_MARKER!"
