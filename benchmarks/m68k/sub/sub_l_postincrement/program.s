; Measure the SUB.L source effective address with four interleaved targets.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.l (a1)+,d5
    move.l d7,d0
    sub.l (a1)+,d5
    move.l d7,d0
    sub.l (a1)+,d5
    move.l d7,d0
    sub.l (a1)+,d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
