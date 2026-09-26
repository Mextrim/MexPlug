// FLHumanMix — авто-сведение + "очеловечивание" стерео-трека для FL Studio.
// Использование: FLHumanMix.exe input.wav [-o output.wav] [--drive 2.0] [--width 1.18] [--room 0.07] [--human 0.6]
// Только System.*, без внешних пакетов. Читает PCM16/24/32 и FLOAT32, пишет FLOAT32.
using System;
using System.IO;
using System.Text;

class FLHumanMix
{
    static int Main(string[] args)
    {
        if (args.Length == 0 || Has(args, "--help") || Has(args, "-h"))
        {
            Console.WriteLine("FLHumanMix — красивый микс + аналоговая живость (против стерильности ИИ-треков)");
            Console.WriteLine("Usage: FLHumanMix.exe input.wav [-o output.wav] [--drive 2.0] [--width 1.18] [--room 0.07] [--human 0.6]");
            Console.WriteLine("  --drive 1.0..4.0  сатурация (по умолч. 2.0)");
            Console.WriteLine("  --width 1.0..1.5  стерео-ширина M/S (по умолч. 1.18)");
            Console.WriteLine("  --room  0.0..0.25 малая комната (по умолч. 0.07)");
            Console.WriteLine("  --human 0.0..1.0  сила wow/flutter+шума (по умолч. 0.6)");
            Console.WriteLine("Пример: FLHumanMix.exe ai_track.wav -o ai_track_MIXED.wav --drive 2.2 --human 0.7");
            return 0;
        }
        string input = args[0];
        string output = GetArg(args, "-o") ?? GetArg(args, "--out")
            ?? (Path.GetDirectoryName(input) == "" ? "" : Path.GetDirectoryName(input) + Path.DirectorySeparatorChar)
               + Path.GetFileNameWithoutExtension(input) + "_MIXED.wav";
        double drive = ParseD(args, "--drive", 2.0);
        double width = ParseD(args, "--width", 1.18);
        double room = ParseD(args, "--room", 0.07);
        double human = ParseD(args, "--human", 0.6);

        if (input == "gen") { GenTest(output == null ? "test_in.wav" : output); return 0; }
        if (!File.Exists(input)) { Console.Error.WriteLine("Нет файла: " + input); return 1; }

        var wav = Wav.Read(input);
        Console.WriteLine($"IN: {wav.SampleRate} Hz, {wav.Channels} ch, {wav.Frames} frames");

        float[][] ch = wav.Data;
        int sr = wav.SampleRate, n = wav.Frames, C = wav.Channels;

        // 1. DC-blocker
        for (int c = 0; c < C; c++)
        {
            double x1 = 0, y1 = 0; const double R = 0.995;
            for (int i = 0; i < n; i++) { double x = ch[c][i]; double y = x - x1 + R * y1; x1 = x; y1 = y; ch[c][i] = (float)y; }
        }
        // 2-4. EQ: HP 28Hz + mud-cut 320Hz + air-shelf 8.2kHz
        var hp = Biquad.Highpass(sr, 28, 0.707);
        var mud = Biquad.Peak(sr, 320, 0.9, -1.2);
        var air = Biquad.HighShelf(sr, 8200, 0.8, 1.6);
        for (int c = 0; c < C; c++) { hp.Reset(); mud.Reset(); air.Reset();
            for (int i = 0; i < n; i++) { double s = ch[c][i]; s = hp.Run(s); s = mud.Run(s); s = air.Run(s); ch[c][i] = (float)s; } }

        // 5. Сатурация tanh (маскирует цифровую стерильность)
        var rnd = new Random( unchecked((int)0xA11CE) );
        for (int c = 0; c < C; c++)
            for (int i = 0; i < n; i++)
            {
                double s = ch[c][i];
                double d = drive * (1.0 + 0.06 * human * (rnd.NextDouble() - 0.5)); // лёгкая вариация драйва
                s = Math.Tanh(s * d) / Math.Tanh(d * 0.9);
                ch[c][i] = (float)(s * 0.92);
            }

        // 6. Стерео-ширина M/S (только если стерео)
        if (C == 2 && Math.Abs(width - 1.0) > 1e-4)
            for (int i = 0; i < n; i++)
            {
                double mid = (ch[0][i] + ch[1][i]) * 0.5, side = (ch[0][i] - ch[1][i]) * 0.5;
                side *= width;
                ch[0][i] = (float)(mid + side); ch[1][i] = (float)(mid - side);
            }

        // 7. Wow/flutter: микроплавание высоты/времени (убирает идеальную квантизацию ИИ)
        if (human > 0.01)
        {
            int maxD = Math.Max(8, sr / 400); // ~2.5мс окно
            for (int c = 0; c < C; c++)
            {
                float[] src = (float[])ch[c].Clone();
                double depth = maxD * 0.55 * human;
                for (int i = 0; i < n; i++)
                {
                    double t = (double)i / sr;
                    double lfo = Math.Sin(2 * Math.PI * 0.45 * t) + 0.5 * Math.Sin(2 * Math.PI * 1.1 * t + 1.3);
                    double dly = maxD * 0.5 + depth * 0.5 * lfo;
                    double pos = i - dly;
                    int p0 = (int)Math.Floor(pos); double fr = pos - p0;
                    float a = (p0 >= 0 && p0 < n) ? src[p0] : 0f;
                    float b = (p0 + 1 >= 0 && p0 + 1 < n) ? src[p0 + 1] : 0f;
                    ch[c][i] = (float)(a + (b - a) * fr);
                }
            }
        }

        // 8. Малая комната (Schroeder: 4 гребёнки + 2 allpass), убирает "сухой" ИИ-звук
        if (room > 0.001 && C >= 1)
        {
            for (int c = 0; c < C; c++)
            {
                var rev = new SimpleRoom(sr, room, c * 97);
                for (int i = 0; i < n; i++) ch[c][i] = rev.Run(ch[c][i]);
            }
        }

        // 9. Лёгкий плёночный шум (-70дБ * human)
        if (human > 0.01)
        {
            double amp = Math.Pow(10, -70.0 / 20.0) * (0.4 + human);
            double lp = 0;
            for (int c = 0; c < C; c++)
                for (int i = 0; i < n; i++)
                { double w = rnd.NextDouble() * 2 - 1; lp = 0.94 * lp + 0.06 * w; ch[c][i] += (float)(lp * amp * 2.0); }
        }

        // 10. Glue-компрессор 2:1, ~2-3дБ
        Glue(ch, sr, threshDb: -18.0, ratio: 2.0, attackMs: 10, releaseMs: 140, makeupDb: 3.0);

        // 11. Лимитер + нормализация под -1.0 dBFS
        double peak = 1e-9;
        for (int c = 0; c < C; c++) for (int i = 0; i < n; i++) peak = Math.Max(peak, Math.Abs(ch[c][i]));
        double target = Math.Pow(10, -1.0 / 20.0);
        double g = target / peak;
        if (g > 4.0) g = 4.0; // не разгонять тишину
        for (int c = 0; c < C; c++)
            for (int i = 0; i < n; i++)
            { double s = ch[c][i] * g; if (s > 1.0) s = 1.0; else if (s < -1.0) s = -1.0; ch[c][i] = (float)s; }

        Wav.WriteFloat32(output, sr, C, ch);
        Console.WriteLine($"OK -> {output}  peak=-1.0 dBFS  drive={drive} width={width} room={room} human={human}");
        return 0;
    }

    static void Glue(float[][] ch, int sr, double threshDb, double ratio, double attackMs, double releaseMs, double makeupDb)
    {
        double thr = Math.Pow(10, threshDb / 20.0);
        double atk = Math.Exp(-1.0 / (sr * attackMs / 1000.0));
        double rel = Math.Exp(-1.0 / (sr * releaseMs / 1000.0));
        double mk = Math.Pow(10, makeupDb / 20.0);
        double env = 0;
        int n = ch[0].Length, C = ch.Length;
        for (int i = 0; i < n; i++)
        {
            double det = 0; for (int c = 0; c < C; c++) det = Math.Max(det, Math.Abs(ch[c][i]));
            env = det > env ? atk * env + (1 - atk) * det : rel * env + (1 - rel) * det;
            double gr = 1.0;
            if (env > thr) gr = Math.Pow(thr / env, 1.0 - 1.0 / ratio);
            for (int c = 0; c < C; c++) ch[c][i] = (float)(ch[c][i] * gr * mk);
        }
    }

    static bool Has(string[] a, string k) { foreach (var s in a) if (s == k) return true; return false; }
    static string GetArg(string[] a, string k) { for (int i = 0; i + 1 < a.Length; i++) if (a[i] == k) return a[i + 1]; return null; }
    static double ParseD(string[] a, string k, double d) { var s = GetArg(a, k); double v; return (s != null && double.TryParse(s, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out v)) ? v : d; }

    static void GenTest(string path)
    {
        int sr = 44100; double dur = 3.0; int n = (int)(sr * dur);
        var L = new float[n]; var R = new float[n];
        for (int i = 0; i < n; i++) { double t = (double)i / sr;
            double s = 0.5 * Math.Sin(2 * Math.PI * 440 * t) + 0.25 * Math.Sin(2 * Math.PI * 880 * t) + 0.12 * Math.Sin(2 * Math.PI * 220 * t);
            s *= 0.6 + 0.4 * Math.Sin(2 * Math.PI * 2 * t); // огибающая
            L[i] = (float)(s * 0.5); R[i] = (float)(s * 0.45); }
        Wav.WriteFloat32(path, sr, 2, new float[][] { L, R });
        Console.WriteLine("gen -> " + path);
    }
}

class Biquad
{
    double b0, b1, b2, a1, a2, x1, x2, y1, y2;
    public void Reset() { x1 = x2 = y1 = y2 = 0; }
    public double Run(double x) { double y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2; x2 = x1; x1 = x; y2 = y1; y1 = y; return y; }
    static Biquad Make(double b0, double b1, double b2, double a0, double a1, double a2)
    { var f = new Biquad(); f.b0 = b0 / a0; f.b1 = b1 / a0; f.b2 = b2 / a0; f.a1 = a1 / a0; f.a2 = a2 / a0; return f; }
    public static Biquad Highpass(int sr, double fc, double q)
    { double w = 2 * Math.PI * fc / sr, c = Math.Cos(w), s = Math.Sin(w), a = s / (2 * q);
      return Make((1 + c) / 2, -(1 + c), (1 + c) / 2, 1 + a, -2 * c, 1 - a); }
    public static Biquad Peak(int sr, double fc, double q, double gdb)
    { double A = Math.Pow(10, gdb / 40.0), w = 2 * Math.PI * fc / sr, c = Math.Cos(w), s = Math.Sin(w), a = s / (2 * q);
      return Make(1 + a * A, -2 * c, 1 - a * A, 1 + a / A, -2 * c, 1 - a / A); }
    public static Biquad HighShelf(int sr, double fc, double S, double gdb)
    { double A = Math.Pow(10, gdb / 40.0), w = 2 * Math.PI * fc / sr, c = Math.Cos(w), s = Math.Sin(w);
      double a = s / 2 * Math.Sqrt((A + 1 / A) * (1 / S - 1) + 2);
      double b0 = A * ((A + 1) + (A - 1) * c + 2 * Math.Sqrt(A) * a);
      double b1 = -2 * A * ((A - 1) + (A + 1) * c);
      double b2 = A * ((A + 1) + (A - 1) * c - 2 * Math.Sqrt(A) * a);
      double a0 = (A + 1) - (A - 1) * c + 2 * Math.Sqrt(A) * a;
      double a1 = 2 * ((A - 1) - (A + 1) * c), a2 = (A + 1) - (A - 1) * c - 2 * Math.Sqrt(A) * a;
      return Make(b0, b1, b2, a0, a1, a2); }
}

// Малая комната: 4 параллельные гребёнки + последовательный allpass.
class SimpleRoom
{
    readonly float[][] combBuf; readonly int[] combIdx; readonly int[] combLen;
    readonly float[] apBuf; int apIdx; readonly int apLen;
    readonly double wet;
    public SimpleRoom(int sr, double roomMix, int seedOff)
    {
        wet = roomMix;
        int[] baseMs = { 29, 37, 44, 53 };
        combBuf = new float[4][]; combIdx = new int[4]; combLen = new int[4];
        for (int k = 0; k < 4; k++) { int L = (sr * (baseMs[k] + seedOff % 7)) / 1000; if (L < 16) L = 16; combLen[k] = L; combBuf[k] = new float[L]; }
        apLen = sr * 7 / 1000; if (apLen < 16) apLen = 16; apBuf = new float[apLen];
    }
    public float Run(float x)
    {
        double r = 0;
        for (int k = 0; k < 4; k++)
        {
            float d = combBuf[k][combIdx[k]];
            combBuf[k][combIdx[k]] = (float)(x + d * 0.72);
            combIdx[k] = (combIdx[k] + 1) % combLen[k];
            r += d;
        }
        r *= 0.25;
        float ad = apBuf[apIdx];
        float y = (float)(-r + ad * 0.5);
        apBuf[apIdx] = (float)(r + ad * 0.5);
        apIdx = (apIdx + 1) % apLen;
        return (float)(x * (1 - wet) + (r * 0.6 + y * 0.4) * wet * 2.0);
    }
}

class Wav
{
    public int SampleRate; public int Channels; public int Frames; public float[][] Data;
    public static Wav Read(string path)
    {
        using (var fs = File.OpenRead(path))
        using (var br = new BinaryReader(fs))
        {
            string riff = Encoding.ASCII.GetString(br.ReadBytes(4));
            if (riff != "RIFF") throw new Exception("Не WAV (нет RIFF)");
            br.ReadInt32();
            string wave = Encoding.ASCII.GetString(br.ReadBytes(4));
            if (wave != "WAVE") throw new Exception("Не WAVE");
            int fmtTag = 0, ch = 0, sr = 0, bits = 0, audioFmt = 0;
            byte[] dataBytes = null;
            while (fs.Position + 8 <= fs.Length)
            {
                string id = Encoding.ASCII.GetString(br.ReadBytes(4));
                int sz = br.ReadInt32();
                long next = fs.Position + sz + (sz % 2);
                if (id == "fmt ")
                {
                    audioFmt = br.ReadInt16(); ch = br.ReadInt16(); sr = br.ReadInt32();
                    br.ReadInt32(); br.ReadInt16(); bits = br.ReadInt16();
                    if (sz > 16) br.ReadBytes(sz - 16);
                }
                else if (id == "data") { dataBytes = br.ReadBytes(sz); }
                fs.Position = next;
                if (dataBytes != null && ch != 0) break;
            }
            if (dataBytes == null) throw new Exception("Нет чанка data");
            int bytesPerSample = bits / 8, frames = dataBytes.Length / (bytesPerSample * ch);
            var data = new float[ch][];
            for (int c = 0; c < ch; c++) data[c] = new float[frames];
            if (audioFmt == 3 && bits == 32)
                for (int i = 0; i < frames; i++) for (int c = 0; c < ch; c++)
                    data[c][i] = BitConverter.ToSingle(dataBytes, (i * ch + c) * 4);
            else if (audioFmt == 1 && bits == 16)
                for (int i = 0; i < frames; i++) for (int c = 0; c < ch; c++)
                    data[c][i] = BitConverter.ToInt16(dataBytes, (i * ch + c) * 2) / 32768f;
            else if (audioFmt == 1 && bits == 24)
                for (int i = 0; i < frames; i++) for (int c = 0; c < ch; c++)
                { int o = (i * ch + c) * 3; int v = dataBytes[o] | (dataBytes[o + 1] << 8) | (dataBytes[o + 2] << 16); if ((v & 0x800000) != 0) v |= unchecked((int)0xFF000000); data[c][i] = v / 8388608f; }
            else if (audioFmt == 1 && bits == 32)
                for (int i = 0; i < frames; i++) for (int c = 0; c < ch; c++)
                    data[c][i] = BitConverter.ToInt32(dataBytes, (i * ch + c) * 4) / 2147483648f;
            else throw new Exception($"Формат не поддержан: fmt={audioFmt} bits={bits}");
            return new Wav { SampleRate = sr, Channels = ch, Frames = frames, Data = data };
        }
    }
    public static void WriteFloat32(string path, int sr, int ch, float[][] data)
    {
        int n = data[0].Length;
        using (var fs = File.Create(path))
        using (var bw = new BinaryWriter(fs))
        {
            int dataSz = n * ch * 4;
            bw.Write(Encoding.ASCII.GetBytes("RIFF")); bw.Write(36 + dataSz); bw.Write(Encoding.ASCII.GetBytes("WAVE"));
            bw.Write(Encoding.ASCII.GetBytes("fmt ")); bw.Write(16);
            bw.Write((short)3); bw.Write((short)ch); bw.Write(sr); bw.Write(sr * ch * 4); bw.Write((short)(ch * 4)); bw.Write((short)32);
            bw.Write(Encoding.ASCII.GetBytes("data")); bw.Write(dataSz);
            for (int i = 0; i < n; i++) for (int c = 0; c < ch; c++) bw.Write(data[c][i]);
        }
    }
}
