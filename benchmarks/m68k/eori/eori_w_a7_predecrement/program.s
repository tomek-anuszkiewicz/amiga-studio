; Measure EORI.W immediate operand with this destination mode.
operand_data:
{{operand_data}}
entry:
    rept {{instruction_count}}
    eori.w #$1357,-(a7)
    move.l d7,d0
    eori.w #$1357,-(a7)
    move.l d7,d0
    eori.w #$1357,-(a7)
    move.l d7,d0
    eori.w #$1357,-(a7)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
