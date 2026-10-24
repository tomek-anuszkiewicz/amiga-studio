; Write a longword to four spaced memory bytes.
operand_data:
    dc.w $1234,$5678,$9ABC,$DEF0,$1357,$2468,$ACE1
entry:
    rept {{instruction_count}}
    movep.l d3,6(a4)
    move.l d7,d0
    movep.l d3,6(a4)
    move.l d7,d0
    movep.l d3,6(a4)
    move.l d7,d0
    movep.l d3,6(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
