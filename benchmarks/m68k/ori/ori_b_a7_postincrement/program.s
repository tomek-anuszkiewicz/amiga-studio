; Measure ORI.B immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    ori.b #$57,(a7)+
    move.l d7,d0
    ori.b #$57,(a7)+
    move.l d7,d0
    ori.b #$57,(a7)+
    move.l d7,d0
    ori.b #$57,(a7)+
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
