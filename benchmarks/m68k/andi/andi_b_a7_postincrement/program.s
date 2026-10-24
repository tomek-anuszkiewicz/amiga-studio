; Measure ANDI.B immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    andi.b #$57,(a7)+
    move.l d7,d0
    andi.b #$57,(a7)+
    move.l d7,d0
    andi.b #$57,(a7)+
    move.l d7,d0
    andi.b #$57,(a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
