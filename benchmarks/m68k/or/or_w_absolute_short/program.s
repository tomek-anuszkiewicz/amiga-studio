; Reload register destinations from D7 so each logical target uses a fresh operand.
entry:
    rept {{instruction_count}}
    or.w $1000.w,d2
    move.l d7,d2
    or.w $1000.w,d2
    move.l d7,d2
    or.w $1000.w,d2
    move.l d7,d2
    or.w $1000.w,d2
    move.l d7,d2
    endr
    dc.b "M68K_END_MARKER!"
