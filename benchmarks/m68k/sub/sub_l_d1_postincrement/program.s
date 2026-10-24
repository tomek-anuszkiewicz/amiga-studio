; Measure SUB.L D1,(A0)+ within the repeated workload.
; The seeded operand data precedes the unrolled instruction block.
; The runner supplies the data and instruction count before assembly.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    sub.l d1,(a0)+
    move.l d1,d2
    sub.l d1,(a0)+
    move.l d1,d2
    sub.l d1,(a0)+
    move.l d1,d2
    sub.l d1,(a0)+
    move.l d1,d2
    endr
    dc.b "M68K_END_MARKER!"
