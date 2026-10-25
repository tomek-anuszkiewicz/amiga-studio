; Four PEA (14,A5,D2.W) targets accumulate addresses on the stack.
entry:
    rept {{instruction_count}}
    pea (14,a5,d2.w)
    move.l d6,d7
    pea (14,a5,d2.w)
    move.l d6,d7
    pea (14,a5,d2.w)
    move.l d6,d7
    pea (14,a5,d2.w)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
