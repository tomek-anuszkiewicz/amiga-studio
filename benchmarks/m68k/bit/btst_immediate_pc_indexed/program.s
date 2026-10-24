; Read the preceding helper opcode as a stable PC-relative byte operand.
    dc.w $2803
entry:
    rept {{instruction_count}}
    btst #5,*-10(pc,d6.w)
    move.l d3,d4
    btst #5,*-10(pc,d6.w)
    move.l d3,d4
    btst #5,*-10(pc,d6.w)
    move.l d3,d4
    btst #5,*-10(pc,d6.w)
    move.l d3,d4
    endr
    dc.b "M68K_END_MARKER!"
