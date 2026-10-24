; Measure ADDI.W immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    addi.w #$1357,$1000.w
    move.l d7,d0
    addi.w #$1357,$1000.w
    move.l d7,d0
    addi.w #$1357,$1000.w
    move.l d7,d0
    addi.w #$1357,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
