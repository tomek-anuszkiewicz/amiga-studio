; Measure AND.W D2,(A7)+ in the repeated workload.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    and.w d2,(a7)+
    move.l d7,d1
    and.w d2,(a7)+
    move.l d7,d1
    and.w d2,(a7)+
    move.l d7,d1
    and.w d2,(a7)+
    move.l d7,d1
    endr
    dc.b "M68K_END_MARKER!"
