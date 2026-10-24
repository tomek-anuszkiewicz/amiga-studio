; Read a fixed divisor through address displacement; MOVE restores D4 from data registers.
operand_data:
    dc.w $2345,$5679,$1357,$0137
entry:
    rept {{instruction_count}}
    divs.w 6(a3),d4
    move.l d0,d4
    divs.w 6(a3),d4
    move.l d1,d4
    divs.w 6(a3),d4
    move.l d2,d4
    divs.w 6(a3),d4
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
