; Measure BTST with a immediate bit number and address_indirect destination.
operand_data:
    dc.w $3457
entry:
    rept {{instruction_count}}
    btst #5,(a3)
    move.l d7,d0
    btst #5,(a3)
    move.l d7,d0
    btst #5,(a3)
    move.l d7,d0
    btst #5,(a3)
    move.l d7,d0
    endr
    dc.b "M68K_END_MARKER!"
