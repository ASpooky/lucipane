// Measure how far a window's content moves for N mouse wheel notches, by matching screenshots
// taken before and after.
using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class Scroll
{
    [DllImport("user32.dll")] static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr param);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    delegate bool EnumProc(IntPtr hwnd, IntPtr param);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    const uint WHEEL = 0x0800;
    public const int NOTCH = 120;

    /// First visible top-level window whose title contains `part`.
    public static IntPtr FindWindow(string part)
    {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, p) =>
        {
            if (!IsWindowVisible(h)) return true;
            var sb = new StringBuilder(512);
            GetWindowText(h, sb, 512);
            if (sb.ToString().Contains(part)) { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static string Title(IntPtr hwnd)
    {
        var sb = new StringBuilder(512);
        GetWindowText(hwnd, sb, 512);
        return sb.ToString();
    }

    /// Grayscale pixels of a screen rectangle, row by row.
    public static byte[] Grab(Rectangle r)
    {
        using (var bmp = new Bitmap(r.Width, r.Height, PixelFormat.Format24bppRgb))
        {
            using (var g = Graphics.FromImage(bmp)) g.CopyFromScreen(r.Left, r.Top, 0, 0, r.Size);
            var data = bmp.LockBits(new Rectangle(0, 0, r.Width, r.Height), ImageLockMode.ReadOnly, bmp.PixelFormat);
            var raw = new byte[data.Stride * r.Height];
            Marshal.Copy(data.Scan0, raw, 0, raw.Length);
            bmp.UnlockBits(data);
            var gray = new byte[r.Width * r.Height];
            for (int y = 0; y < r.Height; y++)
                for (int x = 0; x < r.Width; x++)
                {
                    int i = y * data.Stride + x * 3;
                    gray[y * r.Width + x] = (byte)((raw[i] + raw[i + 1] + raw[i + 2]) / 3);
                }
            return gray;
        }
    }

    static double RowDiff(byte[] a, int ay, byte[] b, int by, int w)
    {
        long sum = 0;
        int oa = ay * w, ob = by * w;
        for (int x = 0; x < w; x += 2) sum += Math.Abs(a[oa + x] - b[ob + x]);
        return sum;
    }

    /// How far the content moved up between `before` and `after` (positive: scrolled down),
    /// as the shift with the lowest mean difference over the overlapping rows. `score` is that
    /// mean difference per sampled pixel (0 = exact match).
    public static int Shift(byte[] before, byte[] after, int w, int h, out double score)
    {
        int max = (int)(h * 0.7);
        int best = 0;
        double bestScore = double.MaxValue;
        for (int s = -max; s <= max; s++)
        {
            int y0 = Math.Max(0, -s), y1 = Math.Min(h, h - s);
            double sum = 0;
            for (int y = y0; y < y1; y++) sum += RowDiff(before, y + s, after, y, w);
            double mean = sum / ((y1 - y0) * (w / 2.0));
            if (mean < bestScore) { bestScore = mean; best = s; }
        }
        score = bestScore;
        return best;
    }

    static bool Same(byte[] a, byte[] b)
    {
        for (int i = 0; i < a.Length; i += 7) if (a[i] != b[i]) return false;
        return true;
    }

    /// Wait until the region stops changing (smooth scrolling), up to `timeoutMs`.
    static byte[] Settle(Rectangle r, int timeoutMs, out int elapsed)
    {
        var sw = Stopwatch.StartNew();
        var prev = Grab(r);
        int stable = 0;
        while (sw.ElapsedMilliseconds < timeoutMs)
        {
            Thread.Sleep(40);
            var now = Grab(r);
            stable = Same(prev, now) ? stable + 1 : 0;
            prev = now;
            if (stable >= 4) break;
        }
        elapsed = (int)sw.ElapsedMilliseconds;
        return prev;
    }

    /// Put the cursor at (cx, cy), send `notches` wheel notches (negative: scroll down), and
    /// return how far the content in `r` moved up.
    public static int Measure(Rectangle r, int cx, int cy, int notches, out double score, out int settleMs)
    {
        SetCursorPos(cx, cy);
        Thread.Sleep(150);
        int dummy;
        var before = Settle(r, 1500, out dummy);
        int dir = notches < 0 ? -1 : 1;
        for (int i = 0; i < Math.Abs(notches); i++)
        {
            mouse_event(WHEEL, 0, 0, dir * NOTCH, UIntPtr.Zero);
            Thread.Sleep(30);
        }
        var after = Settle(r, 3000, out settleMs);
        return Shift(before, after, r.Width, r.Height, out score);
    }
}
