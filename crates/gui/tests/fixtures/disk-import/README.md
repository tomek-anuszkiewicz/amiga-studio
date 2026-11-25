# Disk import fixtures

The 7z member bytes consist of exactly 880 KiB of the given byte for each
valid ADF. The native tests also generate ZIP archives in memory.

| Archive | Members |
| --- | --- |
| `single-disk.7z` | `Archive Fill.adf` (0x33) |
| `mixed-disks.7z` | `Archive Fill.adf` (0x33), `nested/Old copy.ADF` (0x22), `nested/Other.adf` (0x44), `bad.adf` (short), `unrelated.txt` |
| `encrypted-header.7z` | `Archive Fill.adf` (0x33), with password-protected contents and member names |
| `encrypted.zip` | `Archive Fill.adf` (0x33), with password-protected contents |

The fixtures were generated with 7-Zip. The 7z fixtures use solid LZMA2 and
omit file timestamps. The encrypted archive fixtures use the password
`fixture`. The application and the tests read these bytes directly.
