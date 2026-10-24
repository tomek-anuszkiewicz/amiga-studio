; Measure SUBI.L immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    subi.l #$13572469,(a7)+
    move.l d7,d0
    subi.l #$13572469,(a7)+
    move.l d7,d0
    subi.l #$13572469,(a7)+
    move.l d7,d0
    subi.l #$13572469,(a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
