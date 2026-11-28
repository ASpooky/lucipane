# Scroll calibration (Windows)

How the per-notch amounts in `native/src-tauri/src/scroll.rs` were measured. `measure.ps1`
puts the cursor over a window, turns the wheel N notches, waits for smooth scrolling to stop,
and finds how far the content moved by matching screenshots taken before and after.

```powershell
# a window whose title contains "long.txt"; skip 110 px of toolbars at the top and 350 px at
# each side, scroll down 1, 1, 1, 3 and 5 notches
powershell -File measure.ps1 -Title "long.txt" -Inset "110,350,40,350" -Dir down -Series "1,1,1,3,5"
```

Use content where every line differs (numbered lines), give the window enough room to scroll,
and keep the region tall: a shift over 70% of its height is not found. The overlay's own marks
must not be in the region.
