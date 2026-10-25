; The target writes the first byte of the image through an absolute long address.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    sle $200000.l
    move.l d6,d7
    sle $200000.l
    move.l d6,d7
    sle $200000.l
    move.l d6,d7
    sle $200000.l
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
