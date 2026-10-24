; Measure CMPI.L immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    cmpi.l #$13572469,(a3)+
    move.l d7,d0
    cmpi.l #$13572469,(a3)+
    move.l d7,d0
    cmpi.l #$13572469,(a3)+
    move.l d7,d0
    cmpi.l #$13572469,(a3)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
