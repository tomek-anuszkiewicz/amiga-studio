; Invert independent memory words.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    not.w (a3)+
    move.l d7,d0
    not.w (a3)+
    move.l d7,d0
    not.w (a3)+
    move.l d7,d0
    not.w (a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
