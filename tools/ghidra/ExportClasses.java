// Finds Delphi 1 class entries: [u16 selfptr=offset of name pstr][u16 instSize][u16 x][far ptrs (off,seg)*][pstr name]
// Exports analysis/out/classes.json and labels virtual methods as Class_vNN.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.util.*;

public class ExportClasses extends GhidraScript {
  @Override public void run() throws Exception {
    Set<Integer> segs = new HashSet<>();
    for (MemoryBlock b : currentProgram.getMemory().getBlocks()) segs.add((int) b.getStart().getOffset() >> 0 == 0 ? 0 : 0);
    Set<Integer> sels = new HashSet<>();
    for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
      String s = b.getStart().toString(); sels.add(Integer.parseInt(s.substring(0, s.indexOf(':')), 16));
    }
    StringBuilder out = new StringBuilder("[\n"); boolean first = true; int count = 0;
    for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
      if (b.getName().startsWith("Rsrc") || !b.isInitialized()) continue;
      int n = (int) b.getSize(); byte[] d = new byte[n]; b.getBytes(b.getStart(), d);
      String bs = b.getStart().toString(); int sel = Integer.parseInt(bs.substring(0, bs.indexOf(':')), 16);
      for (int p = 6; p < n - 3; p++) {
        int len = d[p] & 0xFF; if (len < 2 || len > 40 || p + 1 + len > n || d[p+1] != 'T') continue;
        boolean ok = true; for (int i = 1; i <= len; i++) { int c = d[p+i] & 0xFF; if (!(Character.isLetterOrDigit(c) || c == '_')) { ok = false; break; } }
        if (!ok) continue;
        int found = -1;
        for (int e = p - 8; e >= Math.max(0, p - 200); e -= 2) {
          if ((((d[e+1] & 0xFF) << 8) | (d[e] & 0xFF)) != p) continue;
          if (((p - (e + 8)) & 3) != 0) continue;
          boolean good = true;
          for (int q = e + 8; q < p; q += 4) {
            int seg = ((d[q+3] & 0xFF) << 8) | (d[q+2] & 0xFF);
            if (!sels.contains(seg)) { good = false; break; }
          }
          if (good) { found = e; break; }
        }
        if (found < 0) continue;
        int e = found; int size = ((d[e+3] & 0xFF) << 8) | (d[e+2] & 0xFF);
        int w = ((d[e+5] & 0xFF) << 8) | (d[e+4] & 0xFF); int hs = ((d[e+7] & 0xFF) << 8) | (d[e+6] & 0xFF);
        String name = new String(d, p + 1, len, "ISO-8859-1");
        if (!first) out.append(",\n"); first = false; count++;
        out.append(String.format("{\"seg\":\"%04x\",\"off\":%d,\"name\":\"%s\",\"size\":%d,\"w\":%d,\"hs\":%d,\"virtual\":[", sel, e, name, size, w, hs));
        int idx = 0;
        for (int q = e + 8; q < p; q += 4, idx++) {
          int off = ((d[q+1] & 0xFF) << 8) | (d[q] & 0xFF), seg = ((d[q+3] & 0xFF) << 8) | (d[q+2] & 0xFF);
          if (idx > 0) out.append(",");
          out.append(String.format("\"%04x:%04x\"", seg, off));
        }
        out.append("]}");
      }
    }
    out.append("\n]\n");
    File f = new File(getScriptArgs()[0]); f.getParentFile().mkdirs();
    try (PrintWriter w = new PrintWriter(f)) { w.print(out); }
    println("classes=" + count);
  }
}
