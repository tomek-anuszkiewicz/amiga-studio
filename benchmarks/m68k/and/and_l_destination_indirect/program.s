; Measure AND.L D6,(A4) in the repeated workload.
operand_data:
    dc.l $12345679
entry:
    rept {{instruction_count}}
    and.l d6,(a4)
    move.l d7,d0
    and.l d6,(a4)
    move.l d7,d0
    and.l d6,(a4)
    move.l d7,d0
    and.l d6,(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
