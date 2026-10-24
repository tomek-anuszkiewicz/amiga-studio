; Four dense immediate multiplies with a restored destination each time.
entry:
    rept {{instruction_count}}
    mulu.w #$7ffd,d5
    move.l d6,d5
    mulu.w #$7ffd,d5
    move.l d6,d5
    mulu.w #$7ffd,d5
    move.l d6,d5
    mulu.w #$7ffd,d5
    move.l d6,d5
    endr
    dc.b "M68K_END_MARKER!"
