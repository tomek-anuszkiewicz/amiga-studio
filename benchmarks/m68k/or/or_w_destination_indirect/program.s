; Measure OR.W D6,(A4) in the repeated workload.
operand_data:
    dc.w $1234
entry:
    rept {{instruction_count}}
    or.w d6,(a4)
    move.l d7,d0
    or.w d6,(a4)
    move.l d7,d0
    or.w d6,(a4)
    move.l d7,d0
    or.w d6,(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
