; Measure SUB.W D1,(A0)+ within the repeated workload.
; The seeded operand data precedes the unrolled instruction block.
; The runner supplies the data and instruction count before assembly.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.w d1,(a0)+
    move.l d1,d2
    sub.w d1,(a0)+
    move.l d1,d2
    sub.w d1,(a0)+
    move.l d1,d2
    sub.w d1,(a0)+
    move.l d1,d2
    endr
    dc.b "M68K_END_MARKER!"
