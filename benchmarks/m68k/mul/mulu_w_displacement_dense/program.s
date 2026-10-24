; A3 plus six reaches a fixed dense word.
    dc.w $1357,$246b,$369d
operand_data:
    dc.w $7ffd
entry:
    rept {{instruction_count}}
    mulu.w 6(a3),d5
    move.l d6,d5
    mulu.w 6(a3),d5
    move.l d6,d5
    mulu.w 6(a3),d5
    move.l d6,d5
    mulu.w 6(a3),d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
