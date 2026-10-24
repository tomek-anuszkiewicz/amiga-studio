; Read four fixed positive bounds without taking the CHK exception.
    dc.w $1357,$246B,$369D
    dc.w $7FFF,$7FFF,$7FFF,$7FFF
entry:
    rept {{instruction_count}}
    chk.w 6(a0,d0.w),d4
    move.l d6,d7
    chk.w 6(a2,d1.w),d4
    move.l d6,d7
    chk.w 6(a4,d2.w),d4
    move.l d6,d7
    chk.w 6(a6,d3.w),d4
    move.l d6,d7
    endr
    dc.b "M68K_END_MARKER!"
