; Move l operands from immediate to stack_postincrement.
destination_data:
    dcb.w 200000,$a55a
entry:
    rept {{instruction_count}}
    move.l #$5a3c2469,(a7)+
    move.l d3,d4
    move.l #$5a3c2469,(a7)+
    move.l d3,d4
    move.l #$5a3c2469,(a7)+
    move.l d3,d4
    move.l #$5a3c2469,(a7)+
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
