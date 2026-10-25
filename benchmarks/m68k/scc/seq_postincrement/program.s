; Z stays clear: SEQ writes $00 and SNE writes $FF.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    seq (a2)+
    move.l d6,d7
    seq (a2)+
    move.l d6,d7
    seq (a2)+
    move.l d6,d7
    seq (a2)+
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
