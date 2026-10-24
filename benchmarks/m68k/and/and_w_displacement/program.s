; Reload register destinations from D7 so each logical target uses a fresh operand.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    and.w 6(a3),d2
    move.l d7,d2
    and.w 6(a3),d2
    move.l d7,d2
    and.w 6(a3),d2
    move.l d7,d2
    and.w 6(a3),d2
    move.l d7,d2
    endr
    dc.b "M68K_END_MARKER!"
