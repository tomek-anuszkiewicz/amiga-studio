; Consume two varied words for each longword moved into the data register.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    move.l (a5)+,d2
    move.l d7,d0
    move.l (a5)+,d2
    move.l d7,d0
    move.l (a5)+,d2
    move.l d7,d0
    move.l (a5)+,d2
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
