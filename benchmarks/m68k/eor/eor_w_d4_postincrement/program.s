; Apply EOR to independent destination words without guest setup instructions.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    eor.w d4,(a3)+
    move.l d7,d0
    eor.w d4,(a3)+
    move.l d7,d0
    eor.w d4,(a3)+
    move.l d7,d0
    eor.w d4,(a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
