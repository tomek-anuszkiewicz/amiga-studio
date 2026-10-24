; Use the same quick value on independent memory words.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subq.w #7,(a4)+
    move.l d7,d0
    subq.w #7,(a4)+
    move.l d7,d0
    subq.w #7,(a4)+
    move.l d7,d0
    subq.w #7,(a4)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
