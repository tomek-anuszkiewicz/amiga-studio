; The indexed destination is the data word at origin + 12.
    dc.w $1357
    dc.w $2469
    dc.w $357B
    dc.w $468D
    dc.w $579F
    dc.w $68A1
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    svs 6(a4,d2.w)
    move.l d6,d7
    svs 6(a4,d2.w)
    move.l d6,d7
    svs 6(a4,d2.w)
    move.l d6,d7
    svs 6(a4,d2.w)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
