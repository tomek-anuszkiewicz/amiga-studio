; Measure the SUB.L source effective address with four interleaved targets.
    dc.w $1357
    dc.w $246B
    dc.w $369D
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    sub.l 6(a3),d2
    move.l d7,d0
    sub.l 6(a3),d2
    move.l d7,d0
    sub.l 6(a3),d2
    move.l d7,d0
    sub.l 6(a3),d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
