; Repeat the register form four times with a MOVE helper after each target.
entry:
    rept {{instruction_count}}
    eori.w #$5A3C,d3
    move.l d7,d0
    eori.w #$5A3C,d3
    move.l d7,d0
    eori.w #$5A3C,d3
    move.l d7,d0
    eori.w #$5A3C,d3
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
