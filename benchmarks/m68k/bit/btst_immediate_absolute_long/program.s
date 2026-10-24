; Measure BTST with a immediate bit number and absolute_long destination.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    btst #5,$200000.l
    move.l d7,d0
    btst #5,$200000.l
    move.l d7,d0
    btst #5,$200000.l
    move.l d7,d0
    btst #5,$200000.l
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
