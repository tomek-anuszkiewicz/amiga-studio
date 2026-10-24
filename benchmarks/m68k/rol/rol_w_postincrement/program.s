; Rotate each independent memory word left by one bit.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    rol.w (a3)+
    move.l d7,d0
    rol.w (a3)+
    move.l d7,d0
    rol.w (a3)+
    move.l d7,d0
    rol.w (a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
