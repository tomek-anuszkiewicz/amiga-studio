; Measure SUBQ.B #5 with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subq.b #5,(a3)+
    move.l d7,d0
    subq.b #5,(a3)+
    move.l d7,d0
    subq.b #5,(a3)+
    move.l d7,d0
    subq.b #5,(a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
