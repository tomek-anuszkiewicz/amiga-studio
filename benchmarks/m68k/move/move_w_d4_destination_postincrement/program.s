; Write each destination word once and advance A6 through varied RAM.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    move.w d4,(a6)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
