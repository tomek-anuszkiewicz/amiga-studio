; Measure SUBI.W immediate operand with this destination mode.
entry:
    rept {{instruction_count}}
    subi.w #$1357,$1000.w
    move.l d7,d0
    subi.w #$1357,$1000.w
    move.l d7,d0
    subi.w #$1357,$1000.w
    move.l d7,d0
    subi.w #$1357,$1000.w
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
