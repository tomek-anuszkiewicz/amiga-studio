
download amiga-stuff-testkit-v1.21
##  Mushashi - generate sources

```powershell
docker run --rm -v "${PWD}/Musashi:/src" -w /src gcc:latest sh -c "gcc -o m68kmake m68kmake.c && ./m68kmake"
```

## MAME - delete 


```powershell
cd mame-mame0289

# 1. Przeniesienie katalogu m68000 tymczasowo do głównego poziomu 
Move-Item -Path "src\devices\cpu\m68000" -Destination "m68000_temp" 
# 2. Usunięcie wszystkich pozostałych plików i podfolderów w repozytorium 
Get-ChildItem -Exclude "m68000_temp" | Remove-Item -Recurse -Force 
# 3. Odtworzenie docelowej struktury i przeniesienie plików na miejsce 
New-Item -ItemType Directory -Path "src\devices\cpu" -Force | Out-Null
Move-Item -Path "m68000_temp" -Destination "src\devices\cpu\m68000"
```


## Graphify

if `graphify` command is not reachable after installation run

```powershell
uv tool update-shell
```