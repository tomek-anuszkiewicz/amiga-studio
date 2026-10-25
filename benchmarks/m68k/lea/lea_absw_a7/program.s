; Four LEA $1000.w targets, each followed by a MOVE helper.
entry:
    rept {{instruction_count}}
    lea $1000.w,a7
    move.l d6,d7
    lea $1000.w,a7
    move.l d6,d7
    lea $1000.w,a7
    move.l d6,d7
    lea $1000.w,a7
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
