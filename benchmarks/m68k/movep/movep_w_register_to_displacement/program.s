; Write a word to spaced memory bytes at zero displacement.
operand_data:
    dc.w $1234,$5678
entry:
    rept {{instruction_count}}
    movep.w d3,0(a4)
    move.l d7,d0
    movep.w d3,0(a4)
    move.l d7,d0
    movep.w d3,0(a4)
    move.l d7,d0
    movep.w d3,0(a4)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
