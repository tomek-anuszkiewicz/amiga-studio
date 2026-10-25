; Four BSR.W targets jump to the following MOVE.L; return addresses stay on the stack.
entry:
    rept {{instruction_count}}
    bsr.w *+4
    move.l d6,d7
    bsr.w *+4
    move.l d6,d7
    bsr.w *+4
    move.l d6,d7
    bsr.w *+4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
