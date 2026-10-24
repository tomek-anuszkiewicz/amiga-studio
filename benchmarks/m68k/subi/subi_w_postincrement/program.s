; Subtract the immediate from each independent memory word.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subi.w #$1357,(a5)+
    move.l d7,d0
    subi.w #$1357,(a5)+
    move.l d7,d0
    subi.w #$1357,(a5)+
    move.l d7,d0
    subi.w #$1357,(a5)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
