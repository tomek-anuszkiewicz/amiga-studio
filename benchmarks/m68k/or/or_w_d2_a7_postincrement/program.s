; Measure OR.W D2,(A7)+ in the repeated workload.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    or.w d2,(a7)+
    move.l d7,d1
    or.w d2,(a7)+
    move.l d7,d1
    or.w d2,(a7)+
    move.l d7,d1
    or.w d2,(a7)+
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
