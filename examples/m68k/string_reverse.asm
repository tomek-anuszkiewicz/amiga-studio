; ==============================================================================
; String Reversal & Palindrome Checker (Motorola 68000)
; ==============================================================================
; Performs two string manipulation operations:
; 1. Copies "AMIGA 500 RULEZ!" to $002000, then reverses it into $002040:
;    "!ZELUR 005 AGIMA"
; 2. Tests whether "RACECAR" is a palindrome, setting D0 = 1 (true) or 0 (false).
;
; Target Load Address: $001000
; Execution Mode:      Pure CPU (no Amiga OS or custom chip DMA required)
;
; Memory Layout:
;   $002000..$00200F: Original copied string ("AMIGA 500 RULEZ!")
;   $002040..$00204F: Reversed string ("!ZELUR 005 AGIMA")
;
; Register Usage:
;   A0 - Source pointer for string copy
;   A1 - Pointer to original string / end pointer for reversal
;   A2 - Destination pointer for reversed string
;   A3 - Left pointer for palindrome scan
;   A4 - Right pointer for palindrome scan
;   D0 - Loop counter / final palindrome result flag (1 = true, 0 = false)
;   D1 - Left character during palindrome comparison
;   D2 - Right character during palindrome comparison
;
; Debugger Tips:
;   - Toggle the ASCII character column in Memory Hex Viewer (checkbox in upper right).
;   - Inspect address $002000 and $002040 to read both strings in clear ASCII text!
;   - Check D0 in the CPU Register panel at halt: D0 = 1 confirms palindrome.
; ==============================================================================

            ORG     $001000

start:
            ; --- Step 1: Copy String 1 to working RAM at $002000 ---
            LEA     str1(PC), A0        ; A0 = pointer to "AMIGA 500 RULEZ!"
            LEA     $002000, A1         ; A1 = working RAM destination
            MOVEQ   #15, D0             ; 16 bytes to copy (counter 15 down to 0)

copy1:
            MOVE.B  (A0)+, (A1)+        ; Copy character and post-increment
            DBRA    D0, copy1

            ; --- Step 2: Reverse String 1 into $002040 ---
            LEA     $002010, A1         ; A1 points one byte PAST the end of string 1
            LEA     $002040, A2         ; A2 = destination for reversed string
            MOVEQ   #15, D0             ; 16 bytes to reverse

rev_loop:
            MOVE.B  -(A1), (A2)+        ; Pre-decrement source, post-increment destination
            DBRA    D0, rev_loop

            ; --- Step 3: Check if String 2 ("RACECAR") is a palindrome ---
            LEA     str2(PC), A3        ; A3 points to start of "RACECAR"
            LEA     7(A3), A4           ; A4 points past end of 7-character string
            MOVEQ   #2, D0              ; Compare 3 pairs: (0,6), (1,5), (2,4)

pal_loop:
            MOVE.B  (A3)+, D1           ; D1 = left char, advance A3
            MOVE.B  -(A4), D2           ; D2 = right char, retreat A4
            CMP.B   D1, D2              ; Compare characters
            BNE.S   not_pal             ; Mismatch -> not a palindrome
            DBRA    D0, pal_loop

            ; Palindrome confirmed!
            MOVEQ   #1, D0              ; D0 = 1 (true)
            BRA.S   halt

not_pal:
            MOVEQ   #0, D0              ; D0 = 0 (false)

halt:
            BRA.S   halt                ; Finished! Spin in idle loop ($60FE)

            ; --- Data Section ---
str1:
            DC.B    "AMIGA 500 RULEZ!"  ; 16 ASCII bytes ($1044)
str2:
            DC.B    "RACECAR", 0        ; 8 bytes with null terminator ($1054)
