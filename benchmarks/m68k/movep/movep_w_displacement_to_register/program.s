; Read a word from spaced bytes at a displaced address.
operand_data:
    dc.w $1234,$5678,$9ABC,$DEF0,$1357,$2468,$ACE1
entry:
    rept {{instruction_count}}
    movep.w 6(a3),d5
    move.l d7,d0
    movep.w 6(a3),d5
    move.l d7,d0
    movep.w 6(a3),d5
    move.l d7,d0
    movep.w 6(a3),d5
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
