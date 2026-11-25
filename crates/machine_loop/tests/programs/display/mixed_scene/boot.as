; Boot block and staged takeover for the unchanged program at $1000.
; The packager writes the boot checksum and appends the program at $500.
BOOT_SIZE       equ $400
READ_SIZE       equ $600
STAGING         equ $74000
PROGRAM_ENTRY   equ $1000
PROGRAM_BYTES   equ 1268
ALLOC_ABS       equ -204
FREE_MEM        equ -210
DO_IO           equ -456
SUPERVISOR      equ -30
IO_COMMAND      equ 28
IO_LENGTH       equ 36
IO_DATA         equ 40
IO_OFFSET       equ 44

    dc.l $444f5300,0,880
boot:
    move.l 4.w,a6
    move.l a1,-(sp)
    move.l #READ_SIZE,d0
    move.l #STAGING,a1
    jsr ALLOC_ABS(a6)
    move.l (sp)+,a1
    tst.l d0
    beq.s failed
    move.w #2,IO_COMMAND(a1)       ; CMD_READ
    move.l #READ_SIZE,IO_LENGTH(a1)
    move.l d0,IO_DATA(a1)
    move.l #BOOT_SIZE,IO_OFFSET(a1)
    jsr DO_IO(a6)
    tst.l d0
    bne.s release_staging
    move.l #STAGING,a0              ; Continuation after firmware closes the device.
    moveq #0,d0
    rts
release_staging:
    move.l #STAGING,a1
    move.l #READ_SIZE,d0
    jsr FREE_MEM(a6)
failed:
    moveq #-1,d0
    rts

    org BOOT_SIZE
continuation:
    move.l 4.w,a6
    lea takeover(pc),a5
    jmp SUPERVISOR(a6)
takeover:
    move.w #$2700,sr
    lea payload(pc),a0
    lea PROGRAM_ENTRY,a1
    move.w #PROGRAM_BYTES/4-1,d0
copy:
    move.l (a0)+,(a1)+
    dbra d0,copy
    jmp PROGRAM_ENTRY

    org $500
payload:
