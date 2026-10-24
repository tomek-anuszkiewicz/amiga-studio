; Measure the SUB.W source effective address with four interleaved targets.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    sub.w $200000.l,d2
    move.l d3,d4
    sub.w $200000.l,d2
    move.l d3,d4
    sub.w $200000.l,d2
    move.l d3,d4
    sub.w $200000.l,d2
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
