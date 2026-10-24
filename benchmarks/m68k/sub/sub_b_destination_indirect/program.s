; Measure SUB.B D6,(A4) within the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    sub.b d6,(a4)
    move.l d7,d0
    sub.b d6,(a4)
    move.l d7,d0
    sub.b d6,(a4)
    move.l d7,d0
    sub.b d6,(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
