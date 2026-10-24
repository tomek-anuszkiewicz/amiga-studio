; Measure EOR.B D1,(A0)+ in the repeated workload.
; The seeded operand data precedes the unrolled instruction block.
; The runner supplies the data and instruction count before assembly.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    eor.b d1,(a0)+
    move.l d1,d2
    eor.b d1,(a0)+
    move.l d1,d2
    eor.b d1,(a0)+
    move.l d1,d2
    eor.b d1,(a0)+
    move.l d1,d2
    endr
    dc.b "M68K_END_MARKER!"
