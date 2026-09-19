#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! =========================================================================================
//! ⚠️ ANTI-TAMPER POLICY & INVARIANCE CONTRACT:
//! DO NOT MODIFY GOLDEN_ROW_HASHES_324 TO "FIX" A FAILING TEST!
//! The 324 golden row hashes below represent verified hardware truth for benchmark CSV rows.
//! Blindly updating these constants to silence a test failure is strictly prohibited by
//! AGENTS.md and spec-compliance.md.
//! =========================================================================================

/// Exact 64-bit FNV-1a hashes for all 324 opcode row entries across Quick, Standard, and Thorough profiles.
/// Covers columns 1..=7: mnemonic,variant,mode,category,opcode,amiga_cck,total_ops.
pub const GOLDEN_ROW_HASHES_324: [u64; 324] = [
    // Profile 0: QUICK (total_ops = 31500)
    0x656ABD9B28AF2CFE, // [  0] NOP,NOP,Implied,Baseline,4E71,4,31500
    0x0B4FD1B81C03BFF5, // [  1] MOVE,MOVE.B D1  D0,DataRegDirect,DataMovement,1001,4,31500
    0xFC4558C91D7B0956, // [  2] MOVE,MOVE.W D1  D0,DataRegDirect,DataMovement,3001,4,31500
    0x2505EC522F120A00, // [  3] MOVE,MOVE.L D1  D0,DataRegDirect,DataMovement,2001,4,31500
    0xB9114C251FBE4401, // [  4] MOVE,MOVE.W (A0)  D0,AddrIndirect,DataMovement,3010,8,31500
    0x044324B5F5E96102, // [  5] MOVE,MOVE.W (A0)+  D0,PostIncrement,DataMovement,3018,8,31500
    0xB5660B3344C9AEBF, // [  6] MOVE,MOVE.W -(A0)  D0,PreDecrement,DataMovement,3020,10,31500
    0xF80BB349868749F1, // [  7] MOVE,MOVE.W 16(A0)  D0,Displacement,DataMovement,3028 0010,12,31500
    0x399874CD506ECC9A, // [  8] MOVE,MOVE.W 8(A0  D2.W)  D0,Index,DataMovement,3030 2008,14,31500
    0xDD0F48E1282FD3AC, // [  9] MOVE,MOVE.W ($2000).W  D0,AbsoluteShort,DataMovement,3038 2000,12,31500
    0xFD372C86729A0716, // [ 10] MOVE,MOVE.W ($00002000).L  D0,AbsoluteLong,DataMovement,3039 0000 2000,16,31500
    0x63F0E427CDE9E408, // [ 11] MOVE,MOVE.W 8(PC)  D0,PcDisplacement,DataMovement,303A 0008,12,31500
    0x88A63E27F737B721, // [ 12] MOVE,MOVE.W #$1234  D0,Immediate,DataMovement,303C 1234,8,31500
    0xE930C1907E1C37A6, // [ 13] MOVE,MOVE.W (A0)+  (A1)+,PostIncrement,DataMovement,32D8,12,31500
    0xFBA996A9743462FD, // [ 14] MOVEA,MOVEA.W D0  A0,DataRegDirect,DataMovement,3040,4,31500
    0x571D56BA3B82ABCF, // [ 15] MOVEA,MOVEA.L D0  A0,DataRegDirect,DataMovement,2040,4,31500
    0x6119D5E87C46596B, // [ 16] MOVEM,MOVEM.W D0-D3  -(SP),PreDecrement,DataMovement,48A7 000F,24,31500
    0xFD730281CE3D17EF, // [ 17] MOVEM,MOVEM.W (SP)+  D0-D3,PostIncrement,DataMovement,4CDF 000F,28,31500
    0x1748CEFDBF2B7690, // [ 18] EXG,EXG D0  D1,DataRegDirect,DataMovement,C141,6,31500
    0xDCF8F951F4BC4F49, // [ 19] EXG,EXG A0  A1,AddrRegDirect,DataMovement,C149,6,31500
    0x6C4DFF91C922987E, // [ 20] LEA,LEA 16(A0)  A1,Displacement,DataMovement,43E8 0010,8,31500
    0xF11B15AD2C59291F, // [ 21] PEA,PEA 16(A0),Displacement,DataMovement,4868 0010,16,31500
    0x07B38757B31020D0, // [ 22] LINK,LINK A6  #-16,Implied,DataMovement,4E56 FFF0,16,31500
    0x14E7A3873B1B28C0, // [ 23] UNLK,UNLK A6,Implied,DataMovement,4E5E,12,31500
    0xEBF118C037A4F3CD, // [ 24] ADD,ADD.B D1  D0,DataRegDirect,Arithmetic,D001,4,31500
    0x4D9041BE1D2F0314, // [ 25] ADD,ADD.W D1  D0,DataRegDirect,Arithmetic,D041,4,31500
    0x6A2AFEC004AB7A1B, // [ 26] ADD,ADD.L D1  D0,DataRegDirect,Arithmetic,D081,8,31500
    0x1607D901A37610D8, // [ 27] ADD,ADD.W (A0)+  D0,PostIncrement,Arithmetic,D058,8,31500
    0xC242D84E0944A879, // [ 28] ADD,ADD.W D0  (A0),AddrIndirect,Arithmetic,D150,12,31500
    0xF7F2D97A50E05E5E, // [ 29] ADDA,ADDA.W D0  A0,DataRegDirect,Arithmetic,D0C0,8,31500
    0x64F7AA79E1405907, // [ 30] ADDA,ADDA.L D0  A0,DataRegDirect,Arithmetic,D0E0,8,31500
    0xD110E871445E9D78, // [ 31] ADDQ,ADDQ.W #4  D0,Immediate,Arithmetic,5840,4,31500
    0x3AE16363116DA159, // [ 32] ADDX,ADDX.W D1  D0,DataRegDirect,Arithmetic,D141,4,31500
    0x5A8CB431272A9D13, // [ 33] ADDX,ADDX.W -(A1)  -(A0),PreDecrement,Arithmetic,D149,18,31500
    0xD4480B447C39DB5D, // [ 34] SUB,SUB.W D1  D0,DataRegDirect,Arithmetic,9041,4,31500
    0x10DCE224FACB8EB2, // [ 35] SUB,SUB.L D1  D0,DataRegDirect,Arithmetic,9081,8,31500
    0x6F3686CC177210E9, // [ 36] SUBA,SUBA.W D0  A0,DataRegDirect,Arithmetic,90C0,8,31500
    0xCA0A53D32B23CEF9, // [ 37] SUBQ,SUBQ.W #4  D0,Immediate,Arithmetic,5940,4,31500
    0xB57E2C1B3C5B196A, // [ 38] SUBX,SUBX.W D1  D0,DataRegDirect,Arithmetic,9141,4,31500
    0xB8866228D8BD80FA, // [ 39] MULU,MULU D1  D0,DataRegDirect,Arithmetic,C0C1,54,31500
    0x11B45160CD9C02AF, // [ 40] MULS,MULS D1  D0,DataRegDirect,Arithmetic,C1C1,54,31500
    0x83A0BAC84587A801, // [ 41] DIVU,DIVU D1  D0,DataRegDirect,Arithmetic,80C1,140,31500
    0x8CC5C64B0DD2F2CE, // [ 42] DIVU,DIVU D1  D0 (Overflow),DataRegDirect,Arithmetic,80C1,10,31500
    0x9FE033F1CF03E916, // [ 43] DIVU,DIVU #0  D0 (Vector 5),Immediate,Arithmetic,80FC 0000,42,31500
    0x4D523526139587F7, // [ 44] DIVS,DIVS D1  D0,DataRegDirect,Arithmetic,81C1,158,31500
    0x7BC7BBC5F3C7AE4B, // [ 45] DIVS,DIVS D1  D0 (Overflow),DataRegDirect,Arithmetic,81C1,10,31500
    0x96C42958FFB1F6D1, // [ 46] DIVS,DIVS #0  D0 (Vector 5),Immediate,Arithmetic,81FC 0000,42,31500
    0xF9CD94B6B1AF5788, // [ 47] NEG,NEG.W D0,DataRegDirect,Arithmetic,4440,4,31500
    0xF9B5D1D20CCAD1B0, // [ 48] NEGX,NEGX.W D0,DataRegDirect,Arithmetic,4040,4,31500
    0x3C1CFBBDC0F3485A, // [ 49] CLR,CLR.W D0,DataRegDirect,Arithmetic,4240,4,31500
    0x6AFF028100797392, // [ 50] CLR,CLR.W (A0)+,PostIncrement,Arithmetic,4258,8,31500
    0xADD16D5135B49D0C, // [ 51] EXT,EXT.W D0,DataRegDirect,Arithmetic,4880,4,31500
    0xB37827833FE9DBAC, // [ 52] EXT,EXT.L D0,DataRegDirect,Arithmetic,48C0,4,31500
    0xC06B2C412C97E1B1, // [ 53] AND,AND.W D1  D0,DataRegDirect,Logic,C041,4,31500
    0xB683D7BFA48EAC7C, // [ 54] AND,AND.L D1  D0,DataRegDirect,Logic,C081,8,31500
    0x6AECF607FC226C39, // [ 55] ANDI,ANDI.W #$AAAA  D0,Immediate,Logic,0240 AAAA,8,31500
    0x8D9F837DC4860EE4, // [ 56] OR,OR.W D1  D0,DataRegDirect,Logic,8041,4,31500
    0xE66406A290C4DDD7, // [ 57] ORI,ORI.W #$5555  D0,Immediate,Logic,0040 5555,8,31500
    0x980D5281632EA588, // [ 58] EOR,EOR.W D1  D0,DataRegDirect,Logic,B140,4,31500
    0x0EDBE9F4907AC07C, // [ 59] EORI,EORI.W #$00FF  D0,Immediate,Logic,0A40 00FF,8,31500
    0x1BCF71706691CA22, // [ 60] NOT,NOT.W D0,DataRegDirect,Logic,4640,4,31500
    0x5890CC734C612ED7, // [ 61] BTST,BTST #5  D0,Immediate,Logic,0800 0005,10,31500
    0x62C23FF21046EE0D, // [ 62] BTST,BTST D1  D0,DataRegDirect,Logic,0100,6,31500
    0xCDC1E49547A61EE6, // [ 63] BSET,BSET D1  D0,DataRegDirect,Logic,01C0,8,31500
    0xA46C081347C4B796, // [ 64] BCLR,BCLR D1  D0,DataRegDirect,Logic,0180,10,31500
    0x036E08004B443C3D, // [ 65] BCHG,BCHG D1  D0,DataRegDirect,Logic,0140,8,31500
    0x245080D9F9137C2C, // [ 66] TAS,TAS (A0)+,PostIncrement,Logic,4AD8,14,31500
    0x5BB864F5889D88EC, // [ 67] LSL,LSL.W #3  D0,Immediate,ShiftRotate,E748,12,31500
    0xE4CF7A1CD736B509, // [ 68] LSR,LSR.W #3  D0,Immediate,ShiftRotate,E648,12,31500
    0x32B19BDF2E75625D, // [ 69] LSL,LSL.W D1  D0,DataRegDirect,ShiftRotate,E368,12,31500
    0x886B867AA2BDC98F, // [ 70] ASL,ASL.W #2  D0,Immediate,ShiftRotate,E540,10,31500
    0x89DBD3C7FD62EFDE, // [ 71] ASR,ASR.W #2  D0,Immediate,ShiftRotate,E440,10,31500
    0x9ECA1D82CC0AD35A, // [ 72] ROL,ROL.W #4  D0,Immediate,ShiftRotate,E958,14,31500
    0x4B9814A8FBCF23A7, // [ 73] ROR,ROR.W #4  D0,Immediate,ShiftRotate,E858,14,31500
    0xAFCE976730F4D83E, // [ 74] SWAP,SWAP D0,DataRegDirect,ShiftRotate,4840,4,31500
    0xB79D08C1AF99D2E8, // [ 75] CMP,CMP.B D1  D0,DataRegDirect,Comparison,B001,4,31500
    0x7314DEF23CD30BB5, // [ 76] CMP,CMP.W D1  D0,DataRegDirect,Comparison,B041,4,31500
    0xDA3DBAF6CE93D754, // [ 77] CMP,CMP.L D1  D0,DataRegDirect,Comparison,B081,6,31500
    0xE3523293DAB17B93, // [ 78] CMPA,CMPA.W D0  A0,DataRegDirect,Comparison,B0C0,6,31500
    0x9FE8D94416C5DF3E, // [ 79] CMPA,CMPA.L D0  A0,DataRegDirect,Comparison,B0E0,6,31500
    0x1454D233EFC04A39, // [ 80] CMPI,CMPI.W #$1234  D0,Immediate,Comparison,0C40 1234,8,31500
    0xE089193913FEFD90, // [ 81] CMPM,CMPM.W (A0)+  (A1)+,PostIncrement,Comparison,B348,12,31500
    0xB6DECD7520EF18DC, // [ 82] TST,TST.W D0,DataRegDirect,Comparison,4A40,4,31500
    0x99458DC7343B1C20, // [ 83] TST,TST.W (A0)+,PostIncrement,Comparison,4A58,8,31500
    0xB40DE058254A60EF, // [ 84] CHK,CHK.W D1  D0,DataRegDirect,Comparison,4181,10,31500
    0x31EAE173060E284E, // [ 85] ABCD,ABCD D1  D0,DataRegDirect,Bcd,C101,6,31500
    0x0372AB4A3B06C1E0, // [ 86] ABCD,ABCD -(A1)  -(A0),PreDecrement,Bcd,C109,18,31500
    0xEBD4B1645E328BCD, // [ 87] SBCD,SBCD D1  D0,DataRegDirect,Bcd,8101,6,31500
    0x2E82E8C6BCC39469, // [ 88] SBCD,SBCD -(A1)  -(A0),PreDecrement,Bcd,8109,18,31500
    0xF38E806C17E74DE2, // [ 89] NBCD,NBCD D0,DataRegDirect,Bcd,4800,6,31500
    0x7ED9C119F273570E, // [ 90] BRA,BRA.S +2,PcDisplacement,ControlFlow,6002,10,31500
    0xC6685708CFF56DFB, // [ 91] Bcc,BEQ.S +2 (Taken),PcDisplacement,ControlFlow,6702,10,31500
    0xB77F95E157C1ECA3, // [ 92] Bcc,BEQ.S +2 (Untaken),PcDisplacement,ControlFlow,6702,8,31500
    0x4E05B66902644884, // [ 93] BSR,BSR.S sub,PcDisplacement,ControlFlow,6104,18,31500
    0x2126D9BF1B2311DF, // [ 94] RTS,RTS,Implied,ControlFlow,4E75,16,31500
    0xCBF21AD3B130E310, // [ 95] RTR,RTR,Implied,ControlFlow,4E77,20,31500
    0x69453550038FFE3E, // [ 96] DBcc,DBF D7  target,Displacement,ControlFlow,51CF 0002,10,31500
    0xA67108D84DB5443E, // [ 97] DBcc,DBF D7  exit (Fallthrough),Displacement,ControlFlow,51CF 0002,14,31500
    0xDC5E22E755245713, // [ 98] MOVE,MOVE.W D0  CCR,DataRegDirect,System,44C0,12,31500
    0x060D1E78306545C2, // [ 99] MOVE,MOVE.W D0  SR,DataRegDirect,System,46C0,12,31500
    0x6616552CAFF7E50D, // [100] MOVE,MOVE.W SR  D0,DataRegDirect,System,40C0,6,31500
    0xB244ADC7047E7222, // [101] ANDI,ANDI.W #$001F  CCR,Immediate,System,023C 001F,20,31500
    0xE63EAE2FA9BD7FCC, // [102] ORI,ORI.W #$0000  CCR,Immediate,System,003C 0000,20,31500
    0xCEE308A8EF0D279B, // [103] MOVE,MOVE USP  A0,AddrRegDirect,System,4E68,4,31500
    0x5DAFF1D541568FA5, // [104] MOVE,MOVE A0  USP,AddrRegDirect,System,4E60,4,31500
    0x5A20A2C18C0EAB2E, // [105] RTE,RTE,Implied,System,4E73,20,31500
    0x051D3F5818303E3F, // [106] TRAP,TRAP #0,Immediate,System,4E40,34,31500
    0x7A2027DBFFB42E39, // [107] TRAPV,TRAPV,Implied,System,4E76,4,31500
    // Profile 1: STANDARD (total_ops = 6997200)
    0x14FD2449CB8701E4, // [108] NOP,NOP,Implied,Baseline,4E71,4,6997200
    0x85C69379CC80805B, // [109] MOVE,MOVE.B D1  D0,DataRegDirect,DataMovement,1001,4,6997200
    0xDD7937B670854CFC, // [110] MOVE,MOVE.W D1  D0,DataRegDirect,DataMovement,3001,4,6997200
    0xFDE0D6FF61FC0926, // [111] MOVE,MOVE.L D1  D0,DataRegDirect,DataMovement,2001,4,6997200
    0xD6C7B61CD5A8F88F, // [112] MOVE,MOVE.W (A0)  D0,AddrIndirect,DataMovement,3010,8,6997200
    0x6103E79C273F9C30, // [113] MOVE,MOVE.W (A0)+  D0,PostIncrement,DataMovement,3018,8,6997200
    0x9A84D53A33AAF8D5, // [114] MOVE,MOVE.W -(A0)  D0,PreDecrement,DataMovement,3020,10,6997200
    0x43F7CFAA0C59D37F, // [115] MOVE,MOVE.W 16(A0)  D0,Displacement,DataMovement,3028 0010,12,6997200
    0x4AB9D86BA722DF88, // [116] MOVE,MOVE.W 8(A0  D2.W)  D0,Index,DataMovement,3030 2008,14,6997200
    0xF16B851073A3E29A, // [117] MOVE,MOVE.W ($2000).W  D0,AbsoluteShort,DataMovement,3038 2000,12,6997200
    0x6CCA2DC188FD30BC, // [118] MOVE,MOVE.W ($00002000).L  D0,AbsoluteLong,DataMovement,3039 0000 2000,16,6997200
    0x97866481C83CE7EE, // [119] MOVE,MOVE.W 8(PC)  D0,PcDisplacement,DataMovement,303A 0008,12,6997200
    0x74D4C1D60A6D18AF, // [120] MOVE,MOVE.W #$1234  D0,Immediate,DataMovement,303C 1234,8,6997200
    0xC932FA82A744B7CC, // [121] MOVE,MOVE.W (A0)+  (A1)+,PostIncrement,DataMovement,32D8,12,6997200
    0xEEA79DFE7910A9E3, // [122] MOVEA,MOVEA.W D0  A0,DataRegDirect,DataMovement,3040,4,6997200
    0xF6C0880D4574BF65, // [123] MOVEA,MOVEA.L D0  A0,DataRegDirect,DataMovement,2040,4,6997200
    0x34B59AC1CB8D28E9, // [124] MOVEM,MOVEM.W D0-D3  -(SP),PreDecrement,DataMovement,48A7 000F,24,6997200
    0xBF27C10AE39B2F85, // [125] MOVEM,MOVEM.W (SP)+  D0-D3,PostIncrement,DataMovement,4CDF 000F,28,6997200
    0xEFBCE99B8266E1B6, // [126] EXG,EXG D0  D1,DataRegDirect,DataMovement,C141,6,6997200
    0x87FC3FAB3CC923F7, // [127] EXG,EXG A0  A1,AddrRegDirect,DataMovement,C149,6,6997200
    0x7622A5F6AD55B964, // [128] LEA,LEA 16(A0)  A1,Displacement,DataMovement,43E8 0010,8,6997200
    0x573F03A7FC141BB5, // [129] PEA,PEA 16(A0),Displacement,DataMovement,4868 0010,16,6997200
    0x9D959B10A97CE5F6, // [130] LINK,LINK A6  #-16,Implied,DataMovement,4E56 FFF0,16,6997200
    0xFB65D03CA33635E6, // [131] UNLK,UNLK A6,Implied,DataMovement,4E5E,12,6997200
    0x1045A5A428E611B3, // [132] ADD,ADD.B D1  D0,DataRegDirect,Arithmetic,D001,4,6997200
    0xDBF5651B742D6AE2, // [133] ADD,ADD.W D1  D0,DataRegDirect,Arithmetic,D041,4,6997200
    0x886D54001C064D19, // [134] ADD,ADD.L D1  D0,DataRegDirect,Arithmetic,D081,8,6997200
    0x16AA0842634363BE, // [135] ADD,ADD.W (A0)+  D0,PostIncrement,Arithmetic,D058,8,6997200
    0x93D68BD9278E7BA7, // [136] ADD,ADD.W D0  (A0),AddrIndirect,Arithmetic,D150,12,6997200
    0x58B2B29644C624C4, // [137] ADDA,ADDA.W D0  A0,DataRegDirect,Arithmetic,D0C0,8,6997200
    0x7161796456DCC37D, // [138] ADDA,ADDA.L D0  A0,DataRegDirect,Arithmetic,D0E0,8,6997200
    0x29FCA963CF77CEDE, // [139] ADDQ,ADDQ.W #4  D0,Immediate,Arithmetic,5840,4,6997200
    0x16302667CF952587, // [140] ADDX,ADDX.W D1  D0,DataRegDirect,Arithmetic,D141,4,6997200
    0xB9BCEA4E250E61B1, // [141] ADDX,ADDX.W -(A1)  -(A0),PreDecrement,Arithmetic,D149,18,6997200
    0x50804ECCFE49AA43, // [142] SUB,SUB.W D1  D0,DataRegDirect,Arithmetic,9041,4,6997200
    0x3B32069C1E420260, // [143] SUB,SUB.L D1  D0,DataRegDirect,Arithmetic,9081,8,6997200
    0xB27BE0F40C3A5D97, // [144] SUBA,SUBA.W D0  A0,DataRegDirect,Arithmetic,90C0,8,6997200
    0x0931A4F2C06F2627, // [145] SUBQ,SUBQ.W #4  D0,Immediate,Arithmetic,5940,4,6997200
    0x4F4823F1915B0158, // [146] SUBX,SUBX.W D1  D0,DataRegDirect,Arithmetic,9141,4,6997200
    0x3FE6969221B5DAE8, // [147] MULU,MULU D1  D0,DataRegDirect,Arithmetic,C0C1,54,6997200
    0x16139DF0C9770845, // [148] MULS,MULS D1  D0,DataRegDirect,Arithmetic,C1C1,54,6997200
    0xBDF964523E95FC8F, // [149] DIVU,DIVU D1  D0,DataRegDirect,Arithmetic,80C1,140,6997200
    0x134164558480D834, // [150] DIVU,DIVU D1  D0 (Overflow),DataRegDirect,Arithmetic,80C1,10,6997200
    0x405349B8FD5862BC, // [151] DIVU,DIVU #0  D0 (Vector 5),Immediate,Arithmetic,80FC 0000,42,6997200
    0xD7F22F0331A82C6D, // [152] DIVS,DIVS D1  D0,DataRegDirect,Arithmetic,81C1,158,6997200
    0xFEB53B34468D29C9, // [153] DIVS,DIVS D1  D0 (Overflow),DataRegDirect,Arithmetic,81C1,10,6997200
    0xD59D11A0451A13DF, // [154] DIVS,DIVS #0  D0 (Vector 5),Immediate,Arithmetic,81FC 0000,42,6997200
    0xB01026E73F46E76E, // [155] NEG,NEG.W D0,DataRegDirect,Arithmetic,4440,4,6997200
    0x73DC3583D9625A56, // [156] NEGX,NEGX.W D0,DataRegDirect,Arithmetic,4040,4,6997200
    0x800A09BC1D95F148, // [157] CLR,CLR.W D0,DataRegDirect,Arithmetic,4240,4,6997200
    0xC5040C9EBD8BA9C0, // [158] CLR,CLR.W (A0)+,PostIncrement,Arithmetic,4258,8,6997200
    0xBF64674A0A3392FA, // [159] EXT,EXT.W D0,DataRegDirect,Arithmetic,4880,4,6997200
    0x3A15178445872A9A, // [160] EXT,EXT.L D0,DataRegDirect,Arithmetic,48C0,4,6997200
    0x592B93B58B11613F, // [161] AND,AND.W D1  D0,DataRegDirect,Logic,C041,4,6997200
    0xE6A627BA9E57C5EA, // [162] AND,AND.L D1  D0,DataRegDirect,Logic,C081,8,6997200
    0x522396B739261567, // [163] ANDI,ANDI.W #$AAAA  D0,Immediate,Logic,0240 AAAA,8,6997200
    0xA8CD421EA5013F32, // [164] OR,OR.W D1  D0,DataRegDirect,Logic,8041,4,6997200
    0x56672E93FF904B4D, // [165] ORI,ORI.W #$5555  D0,Immediate,Logic,0040 5555,8,6997200
    0xAC39AAC02AD4656E, // [166] EOR,EOR.W D1  D0,DataRegDirect,Logic,B140,4,6997200
    0x9547D73EE4E2F9EA, // [167] EORI,EORI.W #$00FF  D0,Immediate,Logic,0A40 00FF,8,6997200
    0x7C63E4AE58F774D0, // [168] NOT,NOT.W D0,DataRegDirect,Logic,4640,4,6997200
    0x19E26A96496C444D, // [169] BTST,BTST #5  D0,Immediate,Logic,0800 0005,10,6997200
    0x545F50FB703DE5F3, // [170] BTST,BTST D1  D0,DataRegDirect,Logic,0100,6,6997200
    0x046BE81004E8810C, // [171] BSET,BSET D1  D0,DataRegDirect,Logic,01C0,8,6997200
    0xDDDBE2A9E854F53C, // [172] BCLR,BCLR D1  D0,DataRegDirect,Logic,0180,10,6997200
    0x1C9AF3B9AABE3523, // [173] BCHG,BCHG D1  D0,DataRegDirect,Logic,0140,8,6997200
    0x4E13BEEB17835F1A, // [174] TAS,TAS (A0)+,PostIncrement,Logic,4AD8,14,6997200
    0x3C8756B0976DA9DA, // [175] LSL,LSL.W #3  D0,Immediate,ShiftRotate,E748,12,6997200
    0xF4EB0BF0D61CAFB7, // [176] LSR,LSR.W #3  D0,Immediate,ShiftRotate,E648,12,6997200
    0x78A1081560874943, // [177] LSL,LSL.W D1  D0,DataRegDirect,ShiftRotate,E368,12,6997200
    0x773917DD74CDC325, // [178] ASL,ASL.W #2  D0,Immediate,ShiftRotate,E540,10,6997200
    0xFC0A0203CFB3F244, // [179] ASR,ASR.W #2  D0,Immediate,ShiftRotate,E440,10,6997200
    0x18A2414FDA9C3448, // [180] ROL,ROL.W #4  D0,Immediate,ShiftRotate,E958,14,6997200
    0x5ABAF4A48DB09B1D, // [181] ROR,ROR.W #4  D0,Immediate,ShiftRotate,E858,14,6997200
    0x00AE81D83C092F24, // [182] SWAP,SWAP D0,DataRegDirect,ShiftRotate,4840,4,6997200
    0x794CFBCE72215F4E, // [183] CMP,CMP.B D1  D0,DataRegDirect,Comparison,B001,4,6997200
    0x74F2FE7F755EE21B, // [184] CMP,CMP.W D1  D0,DataRegDirect,Comparison,B041,4,6997200
    0xD1FDF745EE082922, // [185] CMP,CMP.L D1  D0,DataRegDirect,Comparison,B081,6,6997200
    0x155C145B39F38431, // [186] CMPA,CMPA.W D0  A0,DataRegDirect,Comparison,B0C0,6,6997200
    0x9ECDB9A371B84E24, // [187] CMPA,CMPA.L D0  A0,DataRegDirect,Comparison,B0E0,6,6997200
    0xC4CCBA01F548A367, // [188] CMPI,CMPI.W #$1234  D0,Immediate,Comparison,0C40 1234,8,6997200
    0x9E1C12FAB6FC80B6, // [189] CMPM,CMPM.W (A0)+  (A1)+,PostIncrement,Comparison,B348,12,6997200
    0x98074E5C7506984A, // [190] TST,TST.W D0,DataRegDirect,Comparison,4A40,4,6997200
    0x8ADB80A0519C31C6, // [191] TST,TST.W (A0)+,PostIncrement,Comparison,4A58,8,6997200
    0x326D05AA2E76E085, // [192] CHK,CHK.W D1  D0,DataRegDirect,Comparison,4181,10,6997200
    0xDB681F4F316CE9B4, // [193] ABCD,ABCD D1  D0,DataRegDirect,Bcd,C101,6,6997200
    0xDD90F73A47B2FD86, // [194] ABCD,ABCD -(A1)  -(A0),PreDecrement,Bcd,C109,18,6997200
    0x3FEAF339135B69B3, // [195] SBCD,SBCD D1  D0,DataRegDirect,Bcd,8101,6,6997200
    0xD9731D52D4E2ED17, // [196] SBCD,SBCD -(A1)  -(A0),PreDecrement,Bcd,8109,18,6997200
    0x2FFC5BE79AFCCE90, // [197] NBCD,NBCD D0,DataRegDirect,Bcd,4800,6,6997200
    0x9A294D732395A674, // [198] BRA,BRA.S +2,PcDisplacement,ControlFlow,6002,10,6997200
    0x0F2716567FB83FF9, // [199] Bcc,BEQ.S +2 (Taken),PcDisplacement,ControlFlow,6702,10,6997200
    0x4D8B41095F503F41, // [200] Bcc,BEQ.S +2 (Untaken),PcDisplacement,ControlFlow,6702,8,6997200
    0x56D1FD6ECF894952, // [201] BSR,BSR.S sub,PcDisplacement,ControlFlow,6104,18,6997200
    0x2D1DAED8154CA275, // [202] RTS,RTS,Implied,ControlFlow,4E75,16,6997200
    0x037FA884ED7AC236, // [203] RTR,RTR,Implied,ControlFlow,4E77,20,6997200
    0x61A8E417D5944524, // [204] DBcc,DBF D7  target,Displacement,ControlFlow,51CF 0002,10,6997200
    0x5D3432F54E9E7B24, // [205] DBcc,DBF D7  exit (Fallthrough),Displacement,ControlFlow,51CF 0002,14,6997200
    0xC4FB8C3365FB2BB1, // [206] MOVE,MOVE.W D0  CCR,DataRegDirect,System,44C0,12,6997200
    0xC1BD168B2A6D7EF0, // [207] MOVE,MOVE.W D0  SR,DataRegDirect,System,46C0,12,6997200
    0xFD88F25ADC9A74F3, // [208] MOVE,MOVE.W SR  D0,DataRegDirect,System,40C0,6,6997200
    0x4548986FCCD65CD0, // [209] ANDI,ANDI.W #$001F  CCR,Immediate,System,023C 001F,20,6997200
    0x5DBE73E0463023BA, // [210] ORI,ORI.W #$0000  CCR,Immediate,System,003C 0000,20,6997200
    0xE22346E59D839699, // [211] MOVE,MOVE USP  A0,AddrRegDirect,System,4E68,4,6997200
    0xF6D9DAE548A7EB0B, // [212] MOVE,MOVE A0  USP,AddrRegDirect,System,4E60,4,6997200
    0x96AE8ED566904694, // [213] RTE,RTE,Implied,System,4E73,20,6997200
    0xC60F7D611F567455, // [214] TRAP,TRAP #0,Immediate,System,4E40,34,6997200
    0xE37997F98E842767, // [215] TRAPV,TRAPV,Implied,System,4E76,4,6997200
    // Profile 2: THOROUGH (total_ops = 149992500)
    0x6F7AE21CB1F967CA, // [216] NOP,NOP,Implied,Baseline,4E71,4,149992500
    0x6E81CD4CA73D86DD, // [217] MOVE,MOVE.B D1  D0,DataRegDirect,DataMovement,1001,4,149992500
    0x312A60A2F26FE352, // [218] MOVE,MOVE.W D1  D0,DataRegDirect,DataMovement,3001,4,149992500
    0xC92C7A0A59819E24, // [219] MOVE,MOVE.L D1  D0,DataRegDirect,DataMovement,2001,4,149992500
    0xCA27273C2763EC01, // [220] MOVE,MOVE.W (A0)  D0,AddrIndirect,DataMovement,3010,8,149992500
    0x5F1121FD2E626A76, // [221] MOVE,MOVE.W (A0)+  D0,PostIncrement,DataMovement,3018,8,149992500
    0x398B142C9E78C76F, // [222] MOVE,MOVE.W -(A0)  D0,PreDecrement,DataMovement,3020,10,149992500
    0x006FDF376E6FD371, // [223] MOVE,MOVE.W 16(A0)  D0,Displacement,DataMovement,3028 0010,12,149992500
    0x8499D4CADC76691E, // [224] MOVE,MOVE.W 8(A0  D2.W)  D0,Index,DataMovement,3030 2008,14,149992500
    0x6D87ACCDA29B2CE8, // [225] MOVE,MOVE.W ($2000).W  D0,AbsoluteShort,DataMovement,3038 2000,12,149992500
    0x4B62500BF4169D12, // [226] MOVE,MOVE.W ($00002000).L  D0,AbsoluteLong,DataMovement,3039 0000 2000,16,149992500
    0x85359272643A7FDC, // [227] MOVE,MOVE.W 8(PC)  D0,PcDisplacement,DataMovement,303A 0008,12,149992500
    0x719C959EBFC5D521, // [228] MOVE,MOVE.W #$1234  D0,Immediate,DataMovement,303C 1234,8,149992500
    0x6B47F7C034506E22, // [229] MOVE,MOVE.W (A0)+  (A1)+,PostIncrement,DataMovement,32D8,12,149992500
    0x9C4513BBCF156A95, // [230] MOVEA,MOVEA.W D0  A0,DataRegDirect,DataMovement,3040,4,149992500
    0x0BBB887F0BEE85FF, // [231] MOVEA,MOVEA.L D0  A0,DataRegDirect,DataMovement,2040,4,149992500
    0x5F45C49862B30433, // [232] MOVEM,MOVEM.W D0-D3  -(SP),PreDecrement,DataMovement,48A7 000F,24,149992500
    0x156CDDE4FCBFAF1F, // [233] MOVEM,MOVEM.W (SP)+  D0-D3,PostIncrement,DataMovement,4CDF 000F,28,149992500
    0xA22908FDE5A8D634, // [234] EXG,EXG D0  D1,DataRegDirect,DataMovement,C141,6,149992500
    0x8A7AC343A0B5FE59, // [235] EXG,EXG A0  A1,AddrRegDirect,DataMovement,C149,6,149992500
    0xC76FB6A497CB4B4A, // [236] LEA,LEA 16(A0)  A1,Displacement,DataMovement,43E8 0010,8,149992500
    0x8B8EC1879C5FEB4F, // [237] PEA,PEA 16(A0),Displacement,DataMovement,4868 0010,16,149992500
    0xD4942495B6744474, // [238] LINK,LINK A6  #-16,Implied,DataMovement,4E56 FFF0,16,149992500
    0xF308B824BB0108E4, // [239] UNLK,UNLK A6,Implied,DataMovement,4E5E,12,149992500
    0x139B612E075B9565, // [240] ADD,ADD.B D1  D0,DataRegDirect,Arithmetic,D001,4,149992500
    0xD8D7EA81AB65F900, // [241] ADD,ADD.W D1  D0,DataRegDirect,Arithmetic,D041,4,149992500
    0x52842AA9BE3B6DE3, // [242] ADD,ADD.L D1  D0,DataRegDirect,Arithmetic,D081,8,149992500
    0x80C5B257F66E0C2C, // [243] ADD,ADD.W (A0)+  D0,PostIncrement,Arithmetic,D058,8,149992500
    0xC02A07DB5E73C589, // [244] ADD,ADD.W D0  (A0),AddrIndirect,Arithmetic,D150,12,149992500
    0xF4AF0FCAB2FEED2A, // [245] ADDA,ADDA.W D0  A0,DataRegDirect,Arithmetic,D0C0,8,149992500
    0x9217EA76B1C4DD27, // [246] ADDA,ADDA.L D0  A0,DataRegDirect,Arithmetic,D0E0,8,149992500
    0xC0C80CD18F295FCC, // [247] ADDQ,ADDQ.W #4  D0,Immediate,Arithmetic,5840,4,149992500
    0x79547DF0A2F54269, // [248] ADDX,ADDX.W D1  D0,DataRegDirect,Arithmetic,D141,4,149992500
    0xA4799C401702214B, // [249] ADDX,ADDX.W -(A1)  -(A0),PreDecrement,Arithmetic,D149,18,149992500
    0xF95207C43EBAB2F5, // [250] SUB,SUB.W D1  D0,DataRegDirect,Arithmetic,9041,4,149992500
    0x6336776BABD251A6, // [251] SUB,SUB.L D1  D0,DataRegDirect,Arithmetic,9081,8,149992500
    0xEFD3EC4E5FFB63F9, // [252] SUBA,SUBA.W D0  A0,DataRegDirect,Arithmetic,90C0,8,149992500
    0x08A4F145669E9409, // [253] SUBQ,SUBQ.W #4  D0,Immediate,Arithmetic,5940,4,149992500
    0x67D7D4AB77AC56EE, // [254] SUBX,SUBX.W D1  D0,DataRegDirect,Arithmetic,9141,4,149992500
    0x42EF2C90EC42DD7E, // [255] MULU,MULU D1  D0,DataRegDirect,Arithmetic,C0C1,54,149992500
    0x027B65406FA7A5DF, // [256] MULS,MULS D1  D0,DataRegDirect,Arithmetic,C1C1,54,149992500
    0xB6983BAE27E59001, // [257] DIVU,DIVU D1  D0,DataRegDirect,Arithmetic,80C1,140,149992500
    0xE778A08C831D889A, // [258] DIVU,DIVU D1  D0 (Overflow),DataRegDirect,Arithmetic,80C1,10,149992500
    0xE3FF1F90D8079F12, // [259] DIVU,DIVU #0  D0 (Vector 5),Immediate,Arithmetic,80FC 0000,42,149992500
    0xACD96CE608CEF297, // [260] DIVS,DIVS D1  D0,DataRegDirect,Arithmetic,81C1,158,149992500
    0xDE56AB25C9800C93, // [261] DIVS,DIVS D1  D0 (Overflow),DataRegDirect,Arithmetic,81C1,10,149992500
    0x67845B0F7A2A8551, // [262] DIVS,DIVS #0  D0 (Vector 5),Immediate,Arithmetic,81FC 0000,42,149992500
    0x19BD9B545162EB5C, // [263] NEG,NEG.W D0,DataRegDirect,Arithmetic,4440,4,149992500
    0xE2AB12A705AEEB54, // [264] NEGX,NEGX.W D0,DataRegDirect,Arithmetic,4040,4,149992500
    0x9B1F809C012180DE, // [265] CLR,CLR.W D0,DataRegDirect,Arithmetic,4240,4,149992500
    0x15C3253405933586, // [266] CLR,CLR.W (A0)+,PostIncrement,Arithmetic,4258,8,149992500
    0x6331B64E779F3EC8, // [267] EXT,EXT.W D0,DataRegDirect,Arithmetic,4880,4,149992500
    0xF924783D58D9B4E8, // [268] EXT,EXT.L D0,DataRegDirect,Arithmetic,48C0,4,149992500
    0xA061F3DA9806C731, // [269] AND,AND.W D1  D0,DataRegDirect,Logic,C041,4,149992500
    0x7383E29A922DC838, // [270] AND,AND.L D1  D0,DataRegDirect,Logic,C081,8,149992500
    0xD5625234D090A549, // [271] ANDI,ANDI.W #$AAAA  D0,Immediate,Logic,0240 AAAA,8,149992500
    0xFB621B43D8DEFBD0, // [272] OR,OR.W D1  D0,DataRegDirect,Logic,8041,4,149992500
    0x2710B1CA23CF6A77, // [273] ORI,ORI.W #$5555  D0,Immediate,Logic,0040 5555,8,149992500
    0x2E32E90B7DC6195C, // [274] EOR,EOR.W D1  D0,DataRegDirect,Logic,B140,4,149992500
    0x3BCBD66DB5951C38, // [275] EORI,EORI.W #$00FF  D0,Immediate,Logic,0A40 00FF,8,149992500
    0x142FABF983768416, // [276] NOT,NOT.W D0,DataRegDirect,Logic,4640,4,149992500
    0xC43DE95DF7D54B77, // [277] BTST,BTST #5  D0,Immediate,Logic,0800 0005,10,149992500
    0x90A6451AFE2153A5, // [278] BTST,BTST D1  D0,DataRegDirect,Logic,0100,6,149992500
    0xDB1498B4017F6962, // [279] BSET,BSET D1  D0,DataRegDirect,Logic,01C0,8,149992500
    0xCA3E147D95629592, // [280] BCLR,BCLR D1  D0,DataRegDirect,Logic,0180,10,149992500
    0x2FF4AF1EBCDA77D5, // [281] BCHG,BCHG D1  D0,DataRegDirect,Logic,0140,8,149992500
    0x649617E005CB9D68, // [282] TAS,TAS (A0)+,PostIncrement,Logic,4AD8,14,149992500
    0x7E4106C2D819D628, // [283] LSL,LSL.W #3  D0,Immediate,ShiftRotate,E748,12,149992500
    0x4154E4055B82A019, // [284] LSR,LSR.W #3  D0,Immediate,ShiftRotate,E648,12,149992500
    0x47C5A7F3DC9629F5, // [285] LSL,LSL.W D1  D0,DataRegDirect,ShiftRotate,E368,12,149992500
    0x28EF098497035FBF, // [286] ASL,ASL.W #2  D0,Immediate,ShiftRotate,E540,10,149992500
    0x505500626B4D56AA, // [287] ASR,ASR.W #2  D0,Immediate,ShiftRotate,E440,10,149992500
    0xBBA1E1EE5A8B3BDE, // [288] ROL,ROL.W #4  D0,Immediate,ShiftRotate,E958,14,149992500
    0x254AF9E116EE74C7, // [289] ROR,ROR.W #4  D0,Immediate,ShiftRotate,E858,14,149992500
    0x95BC8CB47F72670A, // [290] SWAP,SWAP D0,DataRegDirect,ShiftRotate,4840,4,149992500
    0x9BBB1367E0E0363C, // [291] CMP,CMP.B D1  D0,DataRegDirect,Comparison,B001,4,149992500
    0xB5C2C7AEA7086E9D, // [292] CMP,CMP.W D1  D0,DataRegDirect,Comparison,B041,4,149992500
    0x60B57111A2A13140, // [293] CMP,CMP.L D1  D0,DataRegDirect,Comparison,B081,6,149992500
    0x7A9A1BA9E24C27CB, // [294] CMPA,CMPA.W D0  A0,DataRegDirect,Comparison,B0C0,6,149992500
    0x2A2618389EFB5E0A, // [295] CMPA,CMPA.L D0  A0,DataRegDirect,Comparison,B0E0,6,149992500
    0xE1EDB644B2036349, // [296] CMPI,CMPI.W #$1234  D0,Immediate,Comparison,0C40 1234,8,149992500
    0xBE2E7E093B9C4D34, // [297] CMPM,CMPM.W (A0)+  (A1)+,PostIncrement,Comparison,B348,12,149992500
    0xC525FC17B4478998, // [298] TST,TST.W D0,DataRegDirect,Comparison,4A40,4,149992500
    0x79FB166484C0C144, // [299] TST,TST.W (A0)+,PostIncrement,Comparison,4A58,8,149992500
    0x1C22B08029E2081F, // [300] CHK,CHK.W D1  D0,DataRegDirect,Comparison,4181,10,149992500
    0x06E450B6AB70D61A, // [301] ABCD,ABCD D1  D0,DataRegDirect,Bcd,C101,6,149992500
    0x06D80670BC92A304, // [302] ABCD,ABCD -(A1)  -(A0),PreDecrement,Bcd,C109,18,149992500
    0xAD19AED8512EAD65, // [303] SBCD,SBCD D1  D0,DataRegDirect,Bcd,8101,6,149992500
    0xB85F34BB1938DF79, // [304] SBCD,SBCD -(A1)  -(A0),PreDecrement,Bcd,8109,18,149992500
    0x26C78DBA886723D6, // [305] NBCD,NBCD D0,DataRegDirect,Bcd,4800,6,149992500
    0xAD7AF8840CD550DA, // [306] BRA,BRA.S +2,PcDisplacement,ControlFlow,6002,10,149992500
    0xBB9A0E58B8594AC3, // [307] Bcc,BEQ.S +2 (Taken),PcDisplacement,ControlFlow,6702,10,149992500
    0x50102FB7B83BB7DB, // [308] Bcc,BEQ.S +2 (Untaken),PcDisplacement,ControlFlow,6702,8,149992500
    0xD3748A0C7979A0F0, // [309] BSR,BSR.S sub,PcDisplacement,ControlFlow,6104,18,149992500
    0xAE548509D369C00F, // [310] RTS,RTS,Implied,ControlFlow,4E75,16,149992500
    0x86686C7A6CE94AB4, // [311] RTR,RTR,Implied,ControlFlow,4E77,20,149992500
    0x9437E120973AED0A, // [312] DBcc,DBF D7  target,Displacement,ControlFlow,51CF 0002,10,149992500
    0xB5B4D70173BF930A, // [313] DBcc,DBF D7  exit (Fallthrough),Displacement,ControlFlow,51CF 0002,14,149992500
    0xF5565463080C7B4B, // [314] MOVE,MOVE.W D0  CCR,DataRegDirect,System,44C0,12,149992500
    0x5FCE1C72264BFB36, // [315] MOVE,MOVE.W D0  SR,DataRegDirect,System,46C0,12,149992500
    0x6321933289C13AA5, // [316] MOVE,MOVE.W SR  D0,DataRegDirect,System,40C0,6,149992500
    0xFC27E77189E1AC16, // [317] ANDI,ANDI.W #$001F  CCR,Immediate,System,023C 001F,20,149992500
    0x05DFE98828ECAD88, // [318] ORI,ORI.W #$0000  CCR,Immediate,System,003C 0000,20,149992500
    0xB49D96382678B363, // [319] MOVE,MOVE USP  A0,AddrRegDirect,System,4E68,4,149992500
    0x09ACA1C66C72038D, // [320] MOVE,MOVE A0  USP,AddrRegDirect,System,4E60,4,149992500
    0x83DF5DC5A064E1FA, // [321] RTE,RTE,Implied,System,4E73,20,149992500
    0x3BDD5CB862720EEF, // [322] TRAP,TRAP #0,Immediate,System,4E40,34,149992500
    0x12F702430A978749, // [323] TRAPV,TRAPV,Implied,System,4E76,4,149992500
];

#[test]
fn test_golden_row_hashes_integrity() {
    assert_eq!(
        GOLDEN_ROW_HASHES_324.len(),
        324,
        "Expected exactly 324 golden row hashes (108 per profile across 3 profiles)"
    );

    // All hashes must be non-zero
    for (idx, &hash) in GOLDEN_ROW_HASHES_324.iter().enumerate() {
        assert_ne!(
            hash, 0,
            "Golden row hash at index {} is zero, which indicates uninitialized or corrupt vector",
            idx
        );
    }

    // 3 distinct profiles: Quick (0..108), Standard (108..216), Thorough (216..324)
    let quick = &GOLDEN_ROW_HASHES_324[0..108];
    let standard = &GOLDEN_ROW_HASHES_324[108..216];
    let thorough = &GOLDEN_ROW_HASHES_324[216..324];

    let mut quick_set = std::collections::HashSet::new();
    for &h in quick {
        assert!(
            quick_set.insert(h),
            "Duplicate hash 0x{:016X} detected within Quick profile",
            h
        );
    }
    assert_eq!(quick_set.len(), 108);

    let mut standard_set = std::collections::HashSet::new();
    for &h in standard {
        assert!(
            standard_set.insert(h),
            "Duplicate hash 0x{:016X} detected within Standard profile",
            h
        );
    }
    assert_eq!(standard_set.len(), 108);

    let mut thorough_set = std::collections::HashSet::new();
    for &h in thorough {
        assert!(
            thorough_set.insert(h),
            "Duplicate hash 0x{:016X} detected within Thorough profile",
            h
        );
    }
    assert_eq!(thorough_set.len(), 108);

    // Overall uniqueness across all 324 entries
    let mut all_set = std::collections::HashSet::new();
    for &h in &GOLDEN_ROW_HASHES_324 {
        assert!(
            all_set.insert(h),
            "Cross-profile collision or duplicate hash 0x{:016X} detected in golden catalog",
            h
        );
    }
    assert_eq!(all_set.len(), 324);
}
