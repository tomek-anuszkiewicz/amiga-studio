; Four PEA 14(A3) targets accumulate addresses on the stack.
entry:
    rept {{instruction_count}}
    pea 14(a3)
    move.l d6,d7
    pea 14(a3)
    move.l d6,d7
    pea 14(a3)
    move.l d6,d7
    pea 14(a3)
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
