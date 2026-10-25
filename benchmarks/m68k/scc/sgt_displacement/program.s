; The target writes a byte through address register displacement.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    sgt 6(a3)
    move.l d6,d7
    sgt 6(a3)
    move.l d6,d7
    sgt 6(a3)
    move.l d6,d7
    sgt 6(a3)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
