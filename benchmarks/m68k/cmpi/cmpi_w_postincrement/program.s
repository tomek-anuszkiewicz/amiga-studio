; Compare the immediate with independent memory words.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpi.w #$1357,(a5)+
    move.l d7,d0
    cmpi.w #$1357,(a5)+
    move.l d7,d0
    cmpi.w #$1357,(a5)+
    move.l d7,d0
    cmpi.w #$1357,(a5)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
