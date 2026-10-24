; Each target shifts an independent memory word.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    asl.w (a7)+
    move.l d7,d0
    asl.w (a7)+
    move.l d7,d0
    asl.w (a7)+
    move.l d7,d0
    asl.w (a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
